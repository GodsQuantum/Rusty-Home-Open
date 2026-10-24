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
    /// Authenticated official job and independent installed App state agreed.
    /// Completed operation history is retained; App lock may be cleared.
    Confirmed,
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
        || (row.state == DispatchState::Confirmed && row.job_id.is_none())
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

/// A remote poll error or local durable storage fault is never success.
#[cfg(feature = "durable-job-watch")]
#[derive(Debug)]
pub enum VerifiedCloseError {
    Storage(io::Error),
    Poll(crate::durable_job_watch::OfficialPollError),
    /// Neither authoritative job detail nor the separately authenticated
    /// official index currently proves this original operation's job.
    MissingOfficialJob,
}
#[cfg(feature = "durable-job-watch")]
impl From<io::Error> for VerifiedCloseError {
    fn from(value: io::Error) -> Self {
        Self::Storage(value)
    }
}
#[cfg(feature = "durable-job-watch")]
impl From<crate::durable_job_watch::OfficialPollError> for VerifiedCloseError {
    fn from(value: crate::durable_job_watch::OfficialPollError) -> Self {
        Self::Poll(value)
    }
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

    /// Resume an accepted asynchronous INSTALL through authenticated,
    /// official Supervisor GET only. The observed job and independent installed
    /// App postcondition must both be confirmed before releasing the App lock.
    /// Never retries a POST. No user HTTP route is exposed.
    #[cfg(feature = "durable-job-watch")]
    pub fn verify_and_close_install(
        &mut self,
        operation_id: &str,
        watches: &mut crate::durable_job_watch::DurableJobWatches,
        client: &AppSupervisorClient,
        version: ApiVersion,
        now_ms: u64,
    ) -> Result<Option<DispatchRecord>, VerifiedCloseError> {
        self.verify_and_close_with(
            operation_id,
            watches,
            version,
            now_ms,
            (
                |v, id| client.fetch_job(v, id),
                |v| client.fetch_installed(v),
                |v| client.fetch_jobs(v),
            ),
        )
    }

    /// Identical official GET protocol with a strictly loopback-only client.
    #[cfg(feature = "durable-job-watch")]
    pub fn verify_and_close_local_install(
        &mut self,
        operation_id: &str,
        watches: &mut crate::durable_job_watch::DurableJobWatches,
        client: &crate::official_transport::LocalSupervisorClient,
        version: ApiVersion,
        now_ms: u64,
    ) -> Result<Option<DispatchRecord>, VerifiedCloseError> {
        self.verify_and_close_with(
            operation_id,
            watches,
            version,
            now_ms,
            (
                |v, id| client.fetch_job(v, id),
                |v| client.fetch_installed(v),
                |v| client.fetch_jobs(v),
            ),
        )
    }

    /// Authorization-sensitive release MUST get fresh authenticated responses.
    /// The generic watch ledger deliberately allows caller-supplied fixtures
    /// and returns cached terminal statuses, so it cannot alone unlock Apps.
    #[cfg(feature = "durable-job-watch")]
    fn verify_and_close_with<FJ, FI, FL>(
        &mut self,
        operation_id: &str,
        watches: &mut crate::durable_job_watch::DurableJobWatches,
        version: ApiVersion,
        now_ms: u64,
        reads: (FJ, FI, FL),
    ) -> Result<Option<DispatchRecord>, VerifiedCloseError>
    where
        FJ: FnOnce(ApiVersion, &str) -> Result<crate::job_status::JobStatus, TransportError>,
        FI: FnOnce(
            ApiVersion,
        ) -> Result<crate::installed_inventory::RunningInventory, TransportError>,
        FL: FnOnce(ApiVersion) -> Result<crate::job_status::JobListing, TransportError>,
    {
        use crate::{
            durable_job_watch::OfficialPollError,
            job_status::State,
            operation_reconcile::{Operation, Outcome, Watch, reconcile},
        };
        let (fetch_job, fetch_inventory, fetch_job_index) = reads;
        let row = self.read_pending_install(operation_id, watches, now_ms)?;
        if row.state == DispatchState::Confirmed {
            // Persisted final state already verified before its ACID commit.
            return Ok(Some(row));
        }
        let job_id = row.job_id.as_deref().ok_or_else(invalid)?;
        // Always request an actual job again. A previously persisted
        // "confirmed" watch might have come from a caller-supplied synthetic
        // observation and is NOT sufficient proof of remote completion.
        let job = match fetch_job(version, job_id) {
            Ok(job) => job,
            Err(TransportError::UnexpectedStatus) => {
                // Some genuine Supervisor builds stop exposing a completed
                // job's detail endpoint. Never infer success from a 404:
                // consult a SECOND authenticated official GET /jobs/info,
                // whose strict parser validates all returned job IDs,
                // duplicates, tree errors and completion state.
                let listing = fetch_job_index(version)
                    .map_err(|e| VerifiedCloseError::Poll(OfficialPollError::Transport(e)))?;
                if !listing.read_only {
                    return Err(VerifiedCloseError::Storage(invalid()));
                }
                let item = listing
                    .jobs
                    .into_iter()
                    .find(|entry| entry.read_only && entry.id == job_id)
                    .ok_or(VerifiedCloseError::MissingOfficialJob)?;
                crate::job_status::JobStatus {
                    id: item.id,
                    state: item.state,
                    progress: item.progress,
                    error_count: item.error_count,
                    stage: None,
                    read_only: true,
                }
            }
            Err(other) => {
                return Err(VerifiedCloseError::Poll(OfficialPollError::Transport(
                    other,
                )));
            }
        };
        let installed = if job.state == State::Succeeded {
            Some(
                fetch_inventory(version)
                    .map_err(|e| VerifiedCloseError::Poll(OfficialPollError::Transport(e)))?,
            )
        } else {
            None
        };
        let watch = Watch::new(&row.app, job_id, Operation::Install, None)
            .map_err(|_| VerifiedCloseError::Storage(invalid()))?;
        let fresh = reconcile(&watch, Some(&job), installed.as_ref());
        // Preserve a read-only observation after the two authenticated GETs.
        // Storage failures must not silently release the safety fence.
        let persisted = watches.observe(job_id, Some(&job), installed.as_ref(), now_ms)?;
        if fresh.outcome != Outcome::Confirmed {
            return Ok(None);
        }
        self.close_after_official_observation(operation_id, persisted.as_ref())
            .map_err(Into::into)
    }

    /// Read the trusted submission-to-job binding, never a UI-provided
    /// receipt, and prepare its idempotent durable watch before polling.
    #[cfg(feature = "durable-job-watch")]
    fn read_pending_install(
        &self,
        operation_id: &str,
        watches: &mut crate::durable_job_watch::DurableJobWatches,
        now_ms: u64,
    ) -> io::Result<DispatchRecord> {
        let row = self.get(operation_id)?.ok_or_else(invalid)?;
        if now_ms < row.created_ms || row.action != Action::Install {
            return Err(invalid());
        }
        if row.state == DispatchState::Accepted {
            self.register_accepted_install_watch(operation_id, watches, now_ms)?;
        } else if row.state != DispatchState::Confirmed {
            // Uncertain POST receipt must never trigger a GET-based unlock.
            return Err(invalid());
        }
        Ok(row)
    }

    #[cfg(feature = "durable-job-watch")]
    fn close_after_official_observation(
        &mut self,
        operation_id: &str,
        observed: Option<&crate::durable_job_watch::WatchReceipt>,
    ) -> io::Result<Option<DispatchRecord>> {
        use crate::operation_reconcile::{Operation, Outcome};
        let row = self.get(operation_id)?.ok_or_else(invalid)?;
        if row.state == DispatchState::Confirmed {
            return Ok(Some(row));
        }
        if row.state != DispatchState::Accepted || row.action != Action::Install {
            return Err(invalid());
        }
        let Some(watch) = observed else {
            return Ok(None);
        };
        if watch.outcome != Some(Outcome::Confirmed) {
            return Ok(None);
        }
        // Require strict origin, actor, operation and creation chronology.
        if !watch.postcondition_observed
            || watch.action != Operation::Install
            || watch.prior_version.is_some()
            || row.job_id.as_deref() != Some(watch.job_id.as_str())
            || watch.app_slug != row.app
            || watch.actor_label != row.actor_subject
            || watch.created_ms < row.created_ms
            || watch.observed_ms.is_none_or(|t| t < watch.created_ms)
        {
            return Err(invalid());
        }
        let mut tx = self.db.begin_write().map_err(error)?;
        tx.set_two_phase_commit(true);
        let completed;
        {
            // Re-read under write transaction, preventing stale competing
            // closers from removing a newer operation's App fence.
            let mut records = tx.open_table(RECORDS).map_err(error)?;
            let raw = records
                .get(operation_id)
                .map_err(error)?
                .map(|g| g.value().to_owned())
                .ok_or_else(invalid)?;
            let mut current = decode(&raw)?;
            if current.state == DispatchState::Confirmed {
                return Ok(Some(current));
            }
            if current != row {
                return Err(invalid());
            }
            let mut active = tx.open_table(ACTIVE).map_err(error)?;
            let active_id = active
                .get(current.app.as_str())
                .map_err(error)?
                .map(|v| v.value().to_owned())
                .ok_or_else(invalid)?;
            if active_id != operation_id {
                return Err(invalid());
            }
            current.state = DispatchState::Confirmed;
            let encoded = serde_json::to_string(&current).map_err(error)?;
            records
                .insert(operation_id, encoded.as_str())
                .map_err(error)?;
            active.remove(current.app.as_str()).map_err(error)?;
            completed = current;
        }
        tx.commit().map_err(error)?;
        Ok(Some(completed))
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
                // Re-entrant or delayed transport receipts cannot regress a
                // terminal, independently verified completion back to accepted.
                if row.state != DispatchState::Uncertain {
                    return Err(invalid());
                }
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
        let mut fence = DurableDispatchFence::open(&db).unwrap();
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
        let closed = fence
            .verify_and_close_local_install(
                &operation_id,
                &mut watches,
                &client,
                ApiVersion::V1,
                110,
            )
            .unwrap()
            .unwrap();
        assert_eq!(closed.state, DispatchState::Confirmed);
        let verified = watches.get(JOB).unwrap().unwrap();
        assert_eq!(
            verified.outcome,
            Some(crate::operation_reconcile::Outcome::Confirmed)
        );
        assert!(verified.postcondition_observed);
        handler.join().unwrap();
        // A durable, independently confirmed completion permits a new
        // separately audited App operation, not a replay of the old permit.
        let new_armed = fence
            .arm(
                permit(dir.path(), "future_vendor_2029", Operation::Install),
                111,
            )
            .unwrap();
        assert_ne!(new_armed.id(), operation_id);
        drop(new_armed);
        drop(fence);
        let mut fence = DurableDispatchFence::open(&db).unwrap();
        let disconnected = crate::official_transport::LocalSupervisorClient::new(
            "127.0.0.1:1".parse().unwrap(),
            crate::official_transport::SupervisorToken::new("offline_fixture".into()).unwrap(),
        )
        .unwrap();
        assert_eq!(
            fence
                .verify_and_close_local_install(
                    &operation_id,
                    &mut watches,
                    &disconnected,
                    ApiVersion::V1,
                    112
                )
                .unwrap()
                .unwrap()
                .state,
            DispatchState::Confirmed
        );
        assert!(
            fence
                .register_accepted_install_watch(&"f".repeat(64), &mut watches, 110)
                .is_err()
        );
    }

    /// Disposable, authenticated loopback GET fixture. Explicit request paths
    /// and credentials are inspected so these tests cannot fake a network poll.
    fn serve_reads(
        expected: Vec<(String, String)>,
    ) -> (
        crate::official_transport::LocalSupervisorClient,
        std::thread::JoinHandle<()>,
    ) {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let addr = listener.local_addr().unwrap();
        let worker = std::thread::spawn(move || {
            for (path, body) in expected {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(std::time::Duration::from_secs(2)))
                    .unwrap();
                let mut data = Vec::new();
                let mut chunk = [0_u8; 1024];
                while !data.windows(4).any(|w| w == b"\r\n\r\n") {
                    let n = stream.read(&mut chunk).unwrap();
                    assert!(n > 0 && data.len() < 8192);
                    data.extend_from_slice(&chunk[..n]);
                }
                let request = String::from_utf8(data).unwrap();
                assert!(request.starts_with(&format!("GET {path} HTTP/1.1\r\n")));
                assert!(request.contains("Authorization: Bearer sandbox_token\r\n"));
                let response = if body == "DENIED" {
                    "HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                        .to_owned()
                } else if body == "NOT_FOUND" {
                    "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                        .to_owned()
                } else {
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    )
                };
                stream.write_all(response.as_bytes()).unwrap();
            }
        });
        let client = crate::official_transport::LocalSupervisorClient::new(
            addr,
            crate::official_transport::SupervisorToken::new("sandbox_token".into()).unwrap(),
        )
        .unwrap();
        (client, worker)
    }

    #[test]
    fn authenticated_job_index_recovery_cannot_forge_or_skip_install_postcondition() {
        let dir = private_dir();
        let slug = "future_vendor_with_job_eviction";
        let mut fence = DurableDispatchFence::open(&dir.path().join("fence.redb")).unwrap();
        let mut watches = DurableJobWatches::open(&dir.path().join("watches.redb")).unwrap();
        let intent = fence
            .arm(permit(dir.path(), slug, Operation::Install), 100)
            .unwrap();
        let id = intent.id().to_owned();
        fence
            .record_transport_result(
                &id,
                &Ok(PreviewReceipt::Accepted {
                    app: slug.into(),
                    action: Action::Install,
                    job_id: Some(JOB.into()),
                    completion_verified: false,
                }),
            )
            .unwrap();

        let job_index = |uuid: &str| {
            serde_json::json!({
                "result":"ok","data":{"jobs":[{
                    "uuid":uuid,"progress":100,"done":true,"errors":[],"child_jobs":[]
                }]}
            })
            .to_string()
        };

        // Detail GET 404 plus *unrelated* successful job does NOT let the
        // caller borrow another operation's completion.
        let (remote, worker) = serve_reads(vec![
            (format!("/jobs/{JOB}"), "NOT_FOUND".into()),
            (
                "/jobs/info".into(),
                job_index("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
            ),
        ]);
        assert!(matches!(
            fence.verify_and_close_local_install(&id, &mut watches, &remote, ApiVersion::V1, 101),
            Err(VerifiedCloseError::MissingOfficialJob)
        ));
        worker.join().unwrap();
        assert_eq!(
            fence.get(&id).unwrap().unwrap().state,
            DispatchState::Accepted
        );
        assert!(
            fence
                .arm(permit(dir.path(), slug, Operation::Install), 102)
                .is_err()
        );

        // Exact official index job success alone cannot bypass independent
        // installed-App inventory confirmation.
        let (remote, worker) = serve_reads(vec![
            (format!("/jobs/{JOB}"), "NOT_FOUND".into()),
            ("/jobs/info".into(), job_index(JOB)),
            (
                "/addons".into(),
                r#"{"result":"ok","data":{"addons":[]}}"#.into(),
            ),
        ]);
        assert!(
            fence
                .verify_and_close_local_install(&id, &mut watches, &remote, ApiVersion::V1, 103)
                .unwrap()
                .is_none()
        );
        worker.join().unwrap();
        assert_eq!(
            fence.get(&id).unwrap().unwrap().state,
            DispatchState::Accepted
        );

        // Only job with EXACT original UUID + separate official installed
        // App GET allow the ACID transition and unlock.
        let inventory = serde_json::json!({
            "result":"ok","data":{"addons":[{
                "slug":slug,"name":"Future unseen vendor",
                "state":"stopped","version":"1.0.0"
            }]}
        })
        .to_string();
        let (remote, worker) = serve_reads(vec![
            (format!("/jobs/{JOB}"), "NOT_FOUND".into()),
            ("/jobs/info".into(), job_index(JOB)),
            ("/addons".into(), inventory),
        ]);
        let confirmed = fence
            .verify_and_close_local_install(&id, &mut watches, &remote, ApiVersion::V1, 104)
            .unwrap()
            .unwrap();
        worker.join().unwrap();
        assert_eq!(confirmed.state, DispatchState::Confirmed);
        let new_operation = fence
            .arm(permit(dir.path(), slug, Operation::Install), 105)
            .unwrap();
        assert_ne!(new_operation.id(), id);
    }

    #[test]
    fn unauthorized_or_unverified_official_reads_never_unlock_an_app() {
        let dir = private_dir();
        let mut fence = DurableDispatchFence::open(&dir.path().join("dispatch.redb")).unwrap();
        let mut watches = DurableJobWatches::open(&dir.path().join("watches.redb")).unwrap();
        let armed = fence
            .arm(
                permit(dir.path(), "future_vendor_safe", Operation::Install),
                100,
            )
            .unwrap();
        let id = armed.id().to_owned();
        fence
            .record_transport_result(
                &id,
                &Ok(PreviewReceipt::Accepted {
                    app: "future_vendor_safe".into(),
                    action: Action::Install,
                    job_id: Some(JOB.into()),
                    completion_verified: false,
                }),
            )
            .unwrap();

        // A generic watch can be poisoned by a synthetic caller. Even a
        // cached terminal success MUST NOT authorize dispatch release.
        fence
            .register_accepted_install_watch(&id, &mut watches, 100)
            .unwrap();
        let fake_job = crate::job_status::JobStatus {
            id: JOB.into(),
            progress: 100.0,
            state: crate::job_status::State::Succeeded,
            error_count: 0,
            stage: None,
            read_only: true,
        };
        let fake_inventory = crate::installed_inventory::RunningInventory {
            api_version: ApiVersion::V1,
            read_only: true,
            apps: vec![crate::installed_inventory::RunningApp {
                slug: "future_vendor_safe".into(),
                name: "Forged fixture".into(),
                repository: Some("fixture".into()),
                state: Some("started".into()),
                version: Some("1.0".into()),
                update_available: Some(false),
            }],
        };
        let cached = watches
            .observe(JOB, Some(&fake_job), Some(&fake_inventory), 100)
            .unwrap()
            .unwrap();
        assert_eq!(
            cached.outcome,
            Some(crate::operation_reconcile::Outcome::Confirmed)
        );

        let (client, worker) = serve_reads(vec![(
            format!("/jobs/{JOB}"),
            format!(
                r#"{{"result":"ok","data":{{"uuid":"{JOB}","progress":40,"done":false,"errors":[],"child_jobs":[]}}}}"#
            ),
        )]);
        assert!(
            fence
                .verify_and_close_local_install(&id, &mut watches, &client, ApiVersion::V1, 101)
                .unwrap()
                .is_none()
        );
        worker.join().unwrap();
        assert_eq!(
            fence.get(&id).unwrap().unwrap().state,
            DispatchState::Accepted
        );

        let (client, worker) = serve_reads(vec![
            (
                format!("/jobs/{JOB}"),
                format!(
                    r#"{{"result":"ok","data":{{"uuid":"{JOB}","progress":100,"done":true,"errors":[],"child_jobs":[]}}}}"#
                ),
            ),
            (
                "/addons".into(),
                r#"{"result":"ok","data":{"addons":[]}}"#.into(),
            ),
        ]);
        assert!(
            fence
                .verify_and_close_local_install(&id, &mut watches, &client, ApiVersion::V1, 102)
                .unwrap()
                .is_none()
        );
        worker.join().unwrap();
        // Generic job-watch terminal state is deliberately immutable:
        // even its forged Confirmed cache does not bypass fresh GET gating.
        assert_eq!(
            watches.get(JOB).unwrap().unwrap().outcome,
            Some(crate::operation_reconcile::Outcome::Confirmed)
        );

        let (client, worker) = serve_reads(vec![(format!("/jobs/{JOB}"), "DENIED".into())]);
        assert!(matches!(
            fence.verify_and_close_local_install(&id, &mut watches, &client, ApiVersion::V1, 103),
            Err(VerifiedCloseError::Poll(_))
        ));
        worker.join().unwrap();
        assert!(
            fence
                .arm(
                    permit(dir.path(), "future_vendor_safe", Operation::Install),
                    104
                )
                .is_err()
        );
        assert_eq!(
            fence.get(&id).unwrap().unwrap().state,
            DispatchState::Accepted
        );

        // Even a client pretending the job is complete cannot close through
        // a forged receipt lacking the independent official postcondition.
        let mut fake = watches.get(JOB).unwrap().unwrap();
        fake.outcome = Some(crate::operation_reconcile::Outcome::Confirmed);
        fake.postcondition_observed = true;
        fake.actor_label = "different_actor".into();
        assert!(
            fence
                .close_after_official_observation(&id, Some(&fake))
                .is_err()
        );
        assert_eq!(
            fence.get(&id).unwrap().unwrap().state,
            DispatchState::Accepted
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
