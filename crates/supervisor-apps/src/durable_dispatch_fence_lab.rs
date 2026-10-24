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
