//! Experimental crash-safe dispatch fence for a single App operation.
//!
//! Independent from the default build, Core login, HAOS installation and any
//! production mutation. A pre-dispatch durable intent is intentionally left
//! unresolved after a crash, timeout or missing response: never retry a POST
//! just because its HTTP response was lost.
use crate::{
    audit_lab::AuditedPermit,
    job_status::job_path,
    managed_apps::{Action, AppSlug},
    official_store::ApiVersion,
    official_transport::{AppSupervisorClient, PreviewReceipt, TransportError},
    session_lab::fresh_token,
};
use redb::{Database, ReadableDatabase, ReadableTable, ReadableTableMetadata, TableDefinition};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io,
    os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
    path::Path,
};

const RECORDS: TableDefinition<&str, &str> = TableDefinition::new("lab_dispatch_intents_v1");
const ACTIVE: TableDefinition<&str, &str> = TableDefinition::new("lab_active_app_dispatch_v1");
const LIMIT: u64 = 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DispatchState {
    /// Durably persisted *before* possible POST, including crash/no response.
    Uncertain,
    /// Response indicated accepted; actual App state is NOT yet verified.
    Accepted,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DispatchRecord {
    pub id: String,
    pub app: String,
    pub action: Action,
    /// Carried from the already-audited permit, never a post-submission parameter.
    pub actor_subject: String,
    pub state: DispatchState,
    pub job_id: Option<String>,
    pub created_ms: u64,
}
fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "invalid lab dispatch fence")
}
fn error<E: std::error::Error + Send + Sync + 'static>(e: E) -> io::Error {
    io::Error::other(e)
}
fn check_id(id: &str) -> io::Result<()> {
    if id.len() != 64
        || !id
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err(invalid());
    }
    Ok(())
}
fn decode(data: &str) -> io::Result<DispatchRecord> {
    let value = serde_json::from_str::<crate::NoDuplicates>(data)
        .map_err(|_| invalid())?
        .0;
    let row: DispatchRecord = serde_json::from_value(value).map_err(|_| invalid())?;
    check_id(&row.id)?;
    AppSlug::parse(&row.app).map_err(|_| invalid())?;
    crate::rbac_lab::Actor::new(&row.actor_subject, crate::rbac_lab::Role::ReadOnly)
        .map_err(|_| invalid())?;
    if row
        .job_id
        .as_ref()
        .is_some_and(|id| job_path(ApiVersion::V1, id).is_err())
        || (row.state == DispatchState::Uncertain && row.job_id.is_some())
    {
        return Err(invalid());
    }
    Ok(row)
}

/// Not Clone: the audited, scoped, single-use lab permit is consumed once.
pub struct ArmedDispatch {
    id: String,
    permit: AuditedPermit,
}
impl ArmedDispatch {
    /// Record identifier is NOT a bearer authorization to submit or release.
    pub fn id(&self) -> &str {
        &self.id
    }
}
pub struct DurableDispatchFence {
    db: Database,
}
#[derive(Debug)]
pub enum TrackedError {
    Transport(TransportError),
    Storage(io::Error),
}
pub struct TrackedReceipt {
    pub operation_id: String,
    pub receipt: PreviewReceipt,
}

impl DurableDispatchFence {
    pub fn open(path: &Path) -> io::Result<Self> {
        let parent = path.parent().ok_or_else(invalid)?;
        let meta = fs::symlink_metadata(parent)?;
        if meta.file_type().is_symlink()
            || !meta.is_dir()
            || meta.uid() != nix::unistd::geteuid().as_raw()
            || meta.permissions().mode() & 0o777 != 0o700
        {
            return Err(invalid());
        }
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(nix::libc::O_NOFOLLOW | nix::libc::O_CLOEXEC)
            .open(path)
        {
            Ok(f) => {
                f.sync_all()?;
                File::open(parent)?.sync_all()?;
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e),
        }
        let f = fs::symlink_metadata(path)?;
        if !f.is_file()
            || f.file_type().is_symlink()
            || f.nlink() != 1
            || f.uid() != nix::unistd::geteuid().as_raw()
            || f.permissions().mode() & 0o777 != 0o600
        {
            return Err(invalid());
        }
        let db = Database::create(path).map_err(error)?;
        let mut tx = db.begin_write().map_err(error)?;
        tx.set_two_phase_commit(true);
        {
            tx.open_table(RECORDS).map_err(error)?;
            tx.open_table(ACTIVE).map_err(error)?;
        }
        tx.commit().map_err(error)?;
        Ok(Self { db })
    }

    pub fn get(&self, operation_id: &str) -> io::Result<Option<DispatchRecord>> {
        check_id(operation_id)?;
        let tx = self.db.begin_read().map_err(error)?;
        let records = tx.open_table(RECORDS).map_err(error)?;
        records
            .get(operation_id)
            .map_err(error)?
            .map(|v| decode(v.value()))
            .transpose()
    }

    /// Recover the accepted, audited and typed install-to-job association.
    /// No caller-supplied actor or job ID is accepted. May be retried after a
    /// crash: duplicate registration is idempotent, conflicting reuse fails.
    /// This is a read-only observation binding, not permission to poll a user's
    /// data or mutate Apps. Updates require a trusted pre-update version first.
    #[cfg(feature = "durable-job-watch")]
    pub fn register_accepted_install_watch(
        &self,
        operation_id: &str,
        watches: &mut crate::durable_job_watch::DurableJobWatches,
        now_ms: u64,
    ) -> io::Result<crate::durable_job_watch::WatchReceipt> {
        let row = self.get(operation_id)?.ok_or_else(invalid)?;
        if row.state != DispatchState::Accepted
            || row.action != Action::Install
            || now_ms < row.created_ms
        {
            return Err(invalid());
        }
        let job = row.job_id.ok_or_else(invalid)?;
        watches.register(
            &job,
            &row.app,
            crate::operation_reconcile::Operation::Install,
            None,
            &row.actor_subject,
            now_ms,
        )
    }

    /// Two-phase commit BEFORE a possible write. Concurrent and restarted
    /// processes cannot arm a second action for the same App. No automatic
    /// expiry or success-based unlock is implemented in this experimental lab.
    pub fn arm(&mut self, permit: AuditedPermit, now_ms: u64) -> io::Result<ArmedDispatch> {
        let request = permit.request();
        let slug = AppSlug::parse(request.target()).map_err(|_| invalid())?;
        let token = fresh_token().map_err(|_| invalid())?;
        let id = token.as_str().to_owned();
        let row = DispatchRecord {
            id: id.clone(),
            app: slug.as_str().to_owned(),
            action: request.action(),
            actor_subject: permit.actor_subject().to_owned(),
            state: DispatchState::Uncertain,
            job_id: None,
            created_ms: now_ms,
        };
        let json = serde_json::to_string(&row).map_err(error)?;
        let mut tx = self.db.begin_write().map_err(error)?;
        tx.set_two_phase_commit(true);
        {
            let mut active = tx.open_table(ACTIVE).map_err(error)?;
            if active.get(slug.as_str()).map_err(error)?.is_some() {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "App dispatch unresolved",
                ));
            }
            active.insert(slug.as_str(), id.as_str()).map_err(error)?;
        }
        {
            let mut records = tx.open_table(RECORDS).map_err(error)?;
            if records.len().map_err(error)? >= LIMIT
                || records.get(id.as_str()).map_err(error)?.is_some()
            {
                return Err(io::Error::new(
                    io::ErrorKind::OutOfMemory,
                    "lab dispatch capacity",
                ));
            }
            records.insert(id.as_str(), json.as_str()).map_err(error)?;
        }
        tx.commit().map_err(error)?;
        Ok(ArmedDispatch { id, permit })
    }

    fn record_transport_result(
        &mut self,
        operation_id: &str,
        response: &Result<PreviewReceipt, TransportError>,
    ) -> io::Result<()> {
        check_id(operation_id)?;
        let mut tx = self.db.begin_write().map_err(error)?;
        tx.set_two_phase_commit(true);
        {
            let mut records = tx.open_table(RECORDS).map_err(error)?;
            let old = records
                .get(operation_id)
                .map_err(error)?
                .map(|v| v.value().to_owned())
                .ok_or_else(invalid)?;
            let mut row = decode(&old)?;
            // Other calls cannot spoof an accepted result or close the fence;
            // this private method sees only the actual preview_execute result.
            if let Ok(receipt) = response {
                let (app, action, id) = match receipt {
                    PreviewReceipt::Accepted {
                        app,
                        action,
                        job_id,
                        ..
                    } => (Some(app.as_str()), Some(*action), job_id.clone()),
                    PreviewReceipt::BackupCreated { .. } => (None, Some(Action::Backup), None),
                };
                if app.is_some_and(|app| app != row.app)
                    || action != Some(row.action)
                    || id
                        .as_ref()
                        .is_some_and(|j| job_path(ApiVersion::V1, j).is_err())
                {
                    return Err(invalid());
                }
                row.state = DispatchState::Accepted;
                row.job_id = id;
                let serialized = serde_json::to_string(&row).map_err(error)?;
                records
                    .insert(operation_id, serialized.as_str())
                    .map_err(error)?;
            }
        }
        tx.commit().map_err(error)
    }
}

impl AppSupervisorClient {
    /// LAB only. Consumes one already audited permit and returns an exact
    /// read-only typed receipt. Every failure leaves a durable unresolved
    /// dispatch; retry/release is deliberately NOT provided.
    pub fn preview_execute_fenced(
        &self,
        fence: &mut DurableDispatchFence,
        armed: ArmedDispatch,
    ) -> Result<TrackedReceipt, TrackedError> {
        let ArmedDispatch { id, permit } = armed;
        let result = self.preview_execute(permit);
        // A failed persistence here leaves the pre-existing Uncertain fence.
        fence
            .record_transport_result(&id, &result)
            .map_err(TrackedError::Storage)?;
        match result {
            Ok(receipt) => Ok(TrackedReceipt {
                operation_id: id,
                receipt,
            }),
            Err(e) => Err(TrackedError::Transport(e)),
        }
    }
}

#[cfg(all(test, feature = "durable-job-watch"))]
mod durable_origin_binding_tests {
    use super::*;
    use crate::{
        audit_lab::{FileAuditJournal, grant_recorded},
        durable_job_watch::DurableJobWatches,
        managed_apps::{Operation, prepare},
        rbac_lab::{Actor, Policy, Role},
    };
    use std::os::unix::fs::PermissionsExt;

    const JOB: &str = "0123456789abcdef0123456789abcdef";
    fn private_dir() -> tempfile::TempDir {
        let d = tempfile::tempdir().unwrap();
        fs::set_permissions(d.path(), fs::Permissions::from_mode(0o700)).unwrap();
        d
    }
    fn permit(path: &Path, slug: &str, op: Operation) -> AuditedPermit {
        let app = AppSlug::parse(slug).unwrap();
        let actor = Actor::new("core_verified_fixture", Role::Administrator).unwrap();
        let mut policy = Policy::default();
        policy.grant(actor.subject(), &app, &[Action::Install, Action::Start]);
        let mut journal = FileAuditJournal::open(&path.join("audit.jsonl")).unwrap();
        let request = prepare(&app, op).unwrap();
        let consent = crate::rbac_lab::Acknowledgement::bind(&actor, &request);
        assert_eq!(
            policy.evaluate(Some(&actor), &request, Some(&consent)).code,
            crate::rbac_lab::DecisionCode::Allowed
        );
        grant_recorded(
            &policy,
            Some(&actor),
            &request,
            Some(&consent),
            &mut journal,
        )
        .unwrap()
        .unwrap()
    }

    #[test]
    fn only_origin_correlated_install_can_bind_persistent_job_after_restart() {
        let dir = private_dir();
        let db = dir.path().join("fence.redb");
        let watches_db = dir.path().join("watches.redb");
        let operation_id = {
            let mut fence = DurableDispatchFence::open(&db).unwrap();
            let armed = fence
                .arm(
                    permit(dir.path(), "future_vendor_2029", Operation::Install),
                    100,
                )
                .unwrap();
            let id = armed.id().to_owned();
            let mut watches = DurableJobWatches::open(&watches_db).unwrap();
            assert!(
                fence
                    .register_accepted_install_watch(&id, &mut watches, 101)
                    .is_err()
            );
            assert!(
                fence
                    .record_transport_result(
                        &id,
                        &Ok(PreviewReceipt::Accepted {
                            app: "wrong_vendor".into(),
                            action: Action::Install,
                            job_id: Some(JOB.into()),
                            completion_verified: false
                        })
                    )
                    .is_err()
            );
            assert_eq!(
                fence.get(&id).unwrap().unwrap().state,
                DispatchState::Uncertain
            );
            assert!(
                fence
                    .record_transport_result(
                        &id,
                        &Ok(PreviewReceipt::Accepted {
                            app: "future_vendor_2029".into(),
                            action: Action::Install,
                            job_id: Some(JOB.into()),
                            completion_verified: false
                        })
                    )
                    .is_ok()
            );
            id
        };
        let fence = DurableDispatchFence::open(&db).unwrap();
        let mut watches = DurableJobWatches::open(&watches_db).unwrap();
        let row = fence
            .register_accepted_install_watch(&operation_id, &mut watches, 105)
            .unwrap();
        assert_eq!(row.job_id, JOB);
        assert_eq!(row.actor_label, "core_verified_fixture");
        assert_eq!(row.app_slug, "future_vendor_2029");
        assert!(!row.postcondition_observed);
        let replay = fence
            .register_accepted_install_watch(&operation_id, &mut watches, 106)
            .unwrap();
        assert_eq!(replay, row);
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let addr = listener.local_addr().unwrap();
        let handler = std::thread::spawn(move || {
            for (path, body) in [
                (
                    format!("/jobs/{JOB}"),
                    format!(r#"{{"result":"ok","data":{{"uuid":"{JOB}","progress":100,"done":true,"errors":[],"child_jobs":[]}}}}"#),
                ),
                (
                    "/addons".to_string(),
                    r#"{"result":"ok","data":{"addons":[{"slug":"future_vendor_2029","name":"Independent vendor","repository":"community","state":"started","version":"1.0","update_available":false}]}}"#.to_string(),
                ),
            ] {
                let (mut stream, _) = listener.accept().unwrap();
                stream.set_read_timeout(Some(std::time::Duration::from_secs(3))).unwrap();
                let mut wire = Vec::new();
                let mut buf = [0u8; 1024];
                while !wire.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
                    let count = stream.read(&mut buf).unwrap();
                    assert!(count > 0 && wire.len() < 8192);
                    wire.extend_from_slice(&buf[..count]);
                }
                let wire = String::from_utf8(wire).unwrap();
                assert!(wire.starts_with(&format!("GET {path} HTTP/1.1\r\n")));
                assert!(wire.contains("Authorization: Bearer fixture_token\r\n"));
                let response = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
                stream.write_all(response.as_bytes()).unwrap();
            }
        });
        let client = crate::official_transport::LocalSupervisorClient::new(
            addr,
            crate::official_transport::SupervisorToken::new("fixture_token".into()).unwrap(),
        )
        .unwrap();
        let verified = watches
            .poll_local_supervisor(JOB, ApiVersion::V1, &client, 110)
            .unwrap()
            .unwrap();
        assert_eq!(
            verified.outcome,
            Some(crate::operation_reconcile::Outcome::Confirmed)
        );
        assert!(verified.postcondition_observed);
        handler.join().unwrap();
        assert!(
            fence
                .register_accepted_install_watch(&"f".repeat(64), &mut watches, 110)
                .is_err()
        );
    }

    #[test]
    fn non_install_operation_cannot_forge_job_watch() {
        let dir = private_dir();
        let mut fence = DurableDispatchFence::open(&dir.path().join("fence.redb")).unwrap();
        let mut watches = DurableJobWatches::open(&dir.path().join("watches.redb")).unwrap();
        let armed = fence
            .arm(permit(dir.path(), "another_vendor", Operation::Start), 100)
            .unwrap();
        let id = armed.id().to_owned();
        fence
            .record_transport_result(
                &id,
                &Ok(PreviewReceipt::Accepted {
                    app: "another_vendor".into(),
                    action: Action::Start,
                    job_id: None,
                    completion_verified: false,
                }),
            )
            .unwrap();
        assert!(
            fence
                .register_accepted_install_watch(&id, &mut watches, 101)
                .is_err()
        );
        assert!(watches.get(JOB).unwrap().is_none());
    }
}
