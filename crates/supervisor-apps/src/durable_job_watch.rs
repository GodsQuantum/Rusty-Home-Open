//! Durable, read-only Supervisor job watches for arbitrary App vendors.
//!
//! This ledger is NOT an authorization, audit or operation-execution receipt:
//! its actor label is caller supplied and MUST be verified by the future Core
//! gateway. It never submits a Supervisor write or infers a job's target from
//! its UUID. Two-phase commits preserve watches and observed states on restart.
use crate::{
    installed_inventory::RunningInventory,
    job_status::{JobStatus, State, job_path},
    official_store::ApiVersion,
    official_transport::{AppSupervisorClient, LocalSupervisorClient, TransportError},
    operation_reconcile::{Operation, Outcome, Watch, reconcile},
};
use redb::{Database, ReadableDatabase, ReadableTable, ReadableTableMetadata, TableDefinition};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io,
    os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
    path::Path,
};

const WATCHES: TableDefinition<&str, &str> = TableDefinition::new("supervisor_readonly_watches_v1");
const MAX_WATCHES: u64 = 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WatchReceipt {
    pub job_id: String,
    pub app_slug: String,
    pub action: Operation,
    pub prior_version: Option<String>,
    /// A label, NOT proof of identity. Never use this field for authorization.
    pub actor_label: String,
    pub created_ms: u64,
    pub observed_ms: Option<u64>,
    pub outcome: Option<Outcome>,
    /// An observed postcondition, NOT proof that this job targeted the App.
    pub postcondition_observed: bool,
}

fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "invalid read-only watch ledger")
}
fn io_error<E: std::error::Error + Send + Sync + 'static>(e: E) -> io::Error {
    io::Error::other(e)
}
fn validate_receipt(row: &WatchReceipt) -> io::Result<Watch> {
    let watch = Watch::new(
        &row.app_slug,
        &row.job_id,
        row.action,
        row.prior_version.as_deref(),
    )
    .map_err(|_| invalid())?;
    if row.actor_label.is_empty()
        || row.actor_label.len() > 128
        || !row
            .actor_label
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-'))
        || row.observed_ms.is_some() != row.outcome.is_some()
        || row.observed_ms.is_some_and(|t| t < row.created_ms)
        || row.postcondition_observed != (row.outcome == Some(Outcome::Confirmed))
    {
        return Err(invalid());
    }
    Ok(watch)
}
fn decode(raw: &str) -> io::Result<WatchReceipt> {
    // No duplicate JSON keys or silently accepted extra fields.
    let strict = serde_json::from_str::<crate::NoDuplicates>(raw)
        .map_err(|_| invalid())?
        .0;
    let row: WatchReceipt = serde_json::from_value(strict).map_err(|_| invalid())?;
    validate_receipt(&row)?;
    Ok(row)
}
fn rank(outcome: Outcome) -> u8 {
    match outcome {
        Outcome::NoEvidence => 0,
        Outcome::Queued => 1,
        Outcome::Running => 2,
        Outcome::AwaitingState => 3,
        Outcome::Failed | Outcome::Confirmed => 4,
    }
}
pub struct DurableJobWatches {
    db: Database,
}

/// Distinguish failed authenticated Supervisor reads from local persistence
/// failures. Neither should be interpreted as a successful App operation.
#[derive(Debug)]
pub enum OfficialPollError {
    Transport(TransportError),
    Storage(io::Error),
}

impl From<io::Error> for OfficialPollError {
    fn from(error: io::Error) -> Self {
        Self::Storage(error)
    }
}

impl From<TransportError> for OfficialPollError {
    fn from(error: TransportError) -> Self {
        Self::Transport(error)
    }
}

impl DurableJobWatches {
    /// Read a previously registered watch using only the authenticated official
    /// App Supervisor's read-only job/inventory APIs, never caller-supplied JSON.
    /// This does NOT check Core-user authorization or prove POST provenance.
    pub fn poll_app_supervisor(
        &mut self,
        job_id: &str,
        version: ApiVersion,
        client: &AppSupervisorClient,
        now_ms: u64,
    ) -> Result<Option<WatchReceipt>, OfficialPollError> {
        self.poll_with(
            job_id,
            version,
            now_ms,
            |v, id| client.fetch_job(v, id),
            |v| client.fetch_installed(v),
        )
    }

    /// Same bounded official GET protocol on an explicitly loopback-only
    /// Supervisor adapter, suitable for isolated integration tests.
    pub fn poll_local_supervisor(
        &mut self,
        job_id: &str,
        version: ApiVersion,
        client: &LocalSupervisorClient,
        now_ms: u64,
    ) -> Result<Option<WatchReceipt>, OfficialPollError> {
        self.poll_with(
            job_id,
            version,
            now_ms,
            |v, id| client.fetch_job(v, id),
            |v| client.fetch_installed(v),
        )
    }

    fn poll_with<FJ, FI>(
        &mut self,
        job_id: &str,
        version: ApiVersion,
        now_ms: u64,
        fetch_job: FJ,
        fetch_inventory: FI,
    ) -> Result<Option<WatchReceipt>, OfficialPollError>
    where
        FJ: FnOnce(ApiVersion, &str) -> Result<JobStatus, TransportError>,
        FI: FnOnce(ApiVersion) -> Result<RunningInventory, TransportError>,
    {
        let Some(previous) = self.get(job_id)? else {
            // No remote GET is made for unregistered or unsafe job identifiers.
            return Ok(None);
        };
        if now_ms < previous.created_ms || previous.observed_ms.is_some_and(|last| now_ms < last) {
            return Err(OfficialPollError::Storage(invalid()));
        }
        if matches!(previous.outcome, Some(Outcome::Failed | Outcome::Confirmed)) {
            // A terminal observation is immutable, including after a restart.
            return Ok(Some(previous));
        }
        let job = fetch_job(version, job_id)?;
        let inventory = if job.state == State::Succeeded {
            // No cached inventory, fabricated empty results or optimistic
            // success when this independent official GET fails.
            Some(fetch_inventory(version)?)
        } else {
            None
        };
        self.observe(job_id, Some(&job), inventory.as_ref(), now_ms)
            .map_err(Into::into)
    }

    /// Owner-private 0700 directory, owner-only regular 0600 file.
    /// Never accepts a symlink or a shared/hard-linked database file.
    pub fn open(path: &Path) -> io::Result<Self> {
        let parent = path.parent().ok_or_else(invalid)?;
        let dir = fs::symlink_metadata(parent)?;
        if !dir.is_dir()
            || dir.file_type().is_symlink()
            || dir.uid() != nix::unistd::geteuid().as_raw()
            || dir.permissions().mode() & 0o777 != 0o700
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
        let file = fs::symlink_metadata(path)?;
        if !file.is_file()
            || file.file_type().is_symlink()
            || file.nlink() != 1
            || file.uid() != nix::unistd::geteuid().as_raw()
            || file.permissions().mode() & 0o777 != 0o600
        {
            return Err(invalid());
        }
        let db = Database::create(path).map_err(io_error)?;
        let mut tx = db.begin_write().map_err(io_error)?;
        tx.set_two_phase_commit(true);
        tx.open_table(WATCHES).map_err(io_error)?;
        tx.commit().map_err(io_error)?;
        Ok(Self { db })
    }

    pub fn get(&self, job_id: &str) -> io::Result<Option<WatchReceipt>> {
        job_path(ApiVersion::V1, job_id).map_err(|_| invalid())?;
        let tx = self.db.begin_read().map_err(io_error)?;
        let table = tx.open_table(WATCHES).map_err(io_error)?;
        let raw = table
            .get(job_id)
            .map_err(io_error)?
            .map(|g| g.value().to_owned());
        raw.map(|s| decode(&s)).transpose()
    }

    /// Store an observation watch, not a trusted mutation receipt.
    /// Repeated identical calls are idempotent; conflicting job reuse fails.
    pub fn register(
        &mut self,
        job_id: &str,
        app_slug: &str,
        action: Operation,
        prior_version: Option<&str>,
        actor_label: &str,
        now_ms: u64,
    ) -> io::Result<WatchReceipt> {
        let row = WatchReceipt {
            job_id: job_id.into(),
            app_slug: app_slug.into(),
            action,
            prior_version: prior_version.map(str::to_owned),
            actor_label: actor_label.into(),
            created_ms: now_ms,
            observed_ms: None,
            outcome: None,
            postcondition_observed: false,
        };
        validate_receipt(&row)?;
        let mut tx = self.db.begin_write().map_err(io_error)?;
        tx.set_two_phase_commit(true);
        {
            let mut table = tx.open_table(WATCHES).map_err(io_error)?;
            if let Some(old) = table
                .get(job_id)
                .map_err(io_error)?
                .map(|g| g.value().to_owned())
            {
                let existing = decode(&old)?;
                if existing.app_slug != row.app_slug
                    || existing.action != row.action
                    || existing.prior_version != row.prior_version
                    || existing.actor_label != row.actor_label
                {
                    return Err(io::Error::new(
                        io::ErrorKind::AlreadyExists,
                        "job already bound to a different watch",
                    ));
                }
                return Ok(existing);
            }
            if table.len().map_err(io_error)? >= MAX_WATCHES {
                return Err(io::Error::new(
                    io::ErrorKind::OutOfMemory,
                    "watch ledger capacity reached",
                ));
            }
            let encoded = serde_json::to_string(&row).map_err(io_error)?;
            table.insert(job_id, encoded.as_str()).map_err(io_error)?;
        }
        tx.commit().map_err(io_error)?;
        Ok(row)
    }

    /// Resume from a persisted watch; only the shared official reconciler
    /// can derive an observation. Terminal results never regress on retry.
    pub fn observe(
        &mut self,
        job_id: &str,
        job: Option<&JobStatus>,
        inventory: Option<&RunningInventory>,
        now_ms: u64,
    ) -> io::Result<Option<WatchReceipt>> {
        job_path(ApiVersion::V1, job_id).map_err(|_| invalid())?;
        let mut tx = self.db.begin_write().map_err(io_error)?;
        tx.set_two_phase_commit(true);
        let mut result = None;
        {
            let mut table = tx.open_table(WATCHES).map_err(io_error)?;
            let raw = table
                .get(job_id)
                .map_err(io_error)?
                .map(|g| g.value().to_owned());
            if let Some(raw) = raw {
                let mut row = decode(&raw)?;
                if now_ms < row.created_ms || row.observed_ms.is_some_and(|t| now_ms < t) {
                    return Err(invalid());
                }
                let watch = validate_receipt(&row)?;
                let observed = reconcile(&watch, job, inventory).outcome;
                let terminal = matches!(row.outcome, Some(Outcome::Failed | Outcome::Confirmed));
                if !terminal && row.outcome.is_none_or(|old| rank(observed) >= rank(old)) {
                    row.observed_ms = Some(now_ms);
                    row.outcome = Some(observed);
                    row.postcondition_observed = observed == Outcome::Confirmed;
                    let encoded = serde_json::to_string(&row).map_err(io_error)?;
                    table.insert(job_id, encoded.as_str()).map_err(io_error)?;
                }
                result = Some(row);
            }
        }
        tx.commit().map_err(io_error)?;
        Ok(result)
    }
}
