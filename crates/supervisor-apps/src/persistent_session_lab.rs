//! Lab-only durable fixture identities and one-shot confirmations.
//! NO trusted IdP, no HTTP ingress, no approval UI. Not a production auth layer.
//! Requires owner-private 0700 parent, owner-only 0600 redb file.
//! redb 2-phase commit ensures spent confirmations survive restart.
use crate::{
    audit_lab::{AuditedPermit, FileAuditJournal, grant_recorded},
    managed_apps::PreparedRequest,
    rbac_lab::{Acknowledgement, Actor, DecisionCode, Policy},
    session_lab::{OpaqueToken, digest_secret, fresh_token},
};
use redb::{Database, ReadableTable, ReadableTableMetadata, TableDefinition};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io,
    os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
    path::Path,
};

// Hard global caps on durable fixture state; no unbounded redb growth.
const MAX_SESSIONS: u64 = 1_024;
const MAX_APPROVALS: u64 = 4_096;
const MAX_APPROVALS_PER_SESSION: usize = 32;
const APPROVAL_TTL: u64 = 60;

const SESSIONS: TableDefinition<&str, &str> = TableDefinition::new("lab_sessions_v1");
const APPROVALS: TableDefinition<&str, &str> = TableDefinition::new("lab_approvals_v1");

#[derive(Serialize, Deserialize)]
struct SessionRow {
    actor: Actor,
    expires: u64,
    idle: u64,
    last_seen: u64,
}
#[derive(Serialize, Deserialize)]
struct ApprovalRow {
    owner_hash: String,
    request_hash: String,
    expires: u64,
}

fn error<E: std::error::Error + Send + Sync + 'static>(e: E) -> io::Error {
    io::Error::other(e)
}
fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, "invalid lab token or time")
}
fn hashed(token: &str) -> Option<String> {
    let x = digest_secret(token)?;
    let mut s = String::with_capacity(64);
    for b in x {
        use std::fmt::Write;
        write!(s, "{b:02x}").ok()?;
    }
    Some(s)
}
fn request_digest(req: &PreparedRequest) -> String {
    let mut h = Sha256::new();
    h.update(b"HARS-P8I-REQUEST-V1");
    h.update(serde_json::to_vec(&req.action()).expect("serializable Action"));
    h.update([0]);
    h.update(req.path().as_bytes());
    h.update([0]);
    h.update(req.body());
    let mut s = String::new();
    for b in h.finalize() {
        use std::fmt::Write;
        write!(s, "{b:02x}").unwrap();
    }
    s
}

pub struct DurableFixtureSessions {
    db: Database,
}

impl DurableFixtureSessions {
    pub fn open(path: &Path) -> io::Result<Self> {
        let parent = path.parent().ok_or_else(invalid)?;
        let meta = fs::symlink_metadata(parent)?;
        if !meta.is_dir()
            || meta.file_type().is_symlink()
            || meta.uid() != nix::unistd::geteuid().as_raw()
            || meta.permissions().mode() & 0o777 != 0o700
        {
            return Err(invalid());
        }
        // O_EXCL creates with owner-only access. Subsequent opens may not follow
        // symlinks or share links; the database itself is the only state file.
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)
        {
            Ok(f) => {
                f.sync_all()?;
                File::open(parent)?.sync_all()?;
            }
            Err(ref e) if e.kind() == io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e),
        }
        let m = fs::symlink_metadata(path)?;
        if !m.is_file()
            || m.file_type().is_symlink()
            || m.nlink() != 1
            || m.uid() != nix::unistd::geteuid().as_raw()
            || m.permissions().mode() & 0o777 != 0o600
        {
            return Err(invalid());
        }
        let db = Database::create(path).map_err(error)?;
        let mut tx = db.begin_write().map_err(error)?;
        tx.set_two_phase_commit(true);
        {
            tx.open_table(SESSIONS).map_err(error)?;
            tx.open_table(APPROVALS).map_err(error)?;
        }
        tx.commit().map_err(error)?;
        Ok(Self { db })
    }

    pub fn issue_fixture(
        &mut self,
        actor: Actor,
        now: u64,
        ttl: u64,
        idle: u64,
    ) -> io::Result<OpaqueToken> {
        if ttl == 0 || ttl > 3600 || idle == 0 || idle > 900 || idle > ttl {
            return Err(invalid());
        }
        let expires = now.checked_add(ttl).ok_or_else(invalid)?;
        let token = fresh_token().map_err(|_| invalid())?;
        let key = hashed(token.as_str()).ok_or_else(invalid)?;
        let row = serde_json::to_string(&SessionRow {
            actor,
            expires,
            idle,
            last_seen: now,
        })
        .map_err(error)?;
        let mut tx = self.db.begin_write().map_err(error)?;
        tx.set_two_phase_commit(true);
        {
            let mut table = tx.open_table(SESSIONS).map_err(error)?;
            // Prune expired, idle and future-clock-skewed fixtures inside the
            // same ACID transaction before checking capacity.
            if table.len().map_err(error)? > MAX_SESSIONS {
                return Err(invalid());
            }
            let mut obsolete = Vec::new();
            for result in table.iter().map_err(error)? {
                let (id, item) = result.map_err(error)?;
                let old: SessionRow = serde_json::from_str(item.value()).map_err(error)?;
                if now < old.last_seen || now >= old.expires || now - old.last_seen >= old.idle {
                    obsolete.push(id.value().to_owned());
                }
            }
            for id in obsolete {
                table.remove(id.as_str()).map_err(error)?;
            }
            if table.len().map_err(error)? >= MAX_SESSIONS {
                return Err(io::Error::new(
                    io::ErrorKind::OutOfMemory,
                    "durable fixture session cap",
                ));
            }
            if table.get(key.as_str()).map_err(error)?.is_some() {
                return Err(invalid());
            }
            table.insert(key.as_str(), row.as_str()).map_err(error)?;
        }
        tx.commit().map_err(error)?;
        Ok(token)
    }

    pub fn revoke(&mut self, token: &str) -> io::Result<bool> {
        let Some(key) = hashed(token) else {
            return Ok(false);
        };
        let mut tx = self.db.begin_write().map_err(error)?;
        tx.set_two_phase_commit(true);
        let result = {
            let mut table = tx.open_table(SESSIONS).map_err(error)?;
            table.remove(key.as_str()).map_err(error)?.is_some()
        };
        // Revoke ALL associated approvals in the same two-phase transaction.
        // Without this, a burst of revoked sessions leaves dangling tokens
        // until normal expiration; never keep unbounded abandoned approvals.
        let mut table = tx.open_table(APPROVALS).map_err(error)?;
        if table.len().map_err(error)? > MAX_APPROVALS {
            return Err(invalid());
        }
        let mut belonging = Vec::new();
        for result in table.iter().map_err(error)? {
            let (id, item) = result.map_err(error)?;
            let approval: ApprovalRow = serde_json::from_str(item.value()).map_err(error)?;
            if approval.owner_hash == key {
                belonging.push(id.value().to_owned());
            }
        }
        for id in belonging {
            table.remove(id.as_str()).map_err(error)?;
        }
        drop(table);
        tx.commit().map_err(error)?;
        Ok(result)
    }

    pub fn issue_confirmation(
        &mut self,
        policy: &Policy,
        token: &str,
        req: &PreparedRequest,
        now: u64,
    ) -> io::Result<Option<OpaqueToken>> {
        let Some(key) = hashed(token) else {
            return Ok(None);
        };
        let mut tx = self.db.begin_write().map_err(error)?;
        tx.set_two_phase_commit(true);
        let mut result = None;
        {
            let mut sessions = tx.open_table(SESSIONS).map_err(error)?;
            let raw = sessions
                .get(key.as_str())
                .map_err(error)?
                .map(|g| g.value().to_owned());
            if let Some(raw) = raw {
                let mut row: SessionRow = serde_json::from_str(&raw).map_err(error)?;
                if now < row.last_seen || now >= row.expires || now - row.last_seen >= row.idle {
                    sessions.remove(key.as_str()).map_err(error)?;
                } else {
                    row.last_seen = now;
                    let encoded = serde_json::to_string(&row).map_err(error)?;
                    sessions
                        .insert(key.as_str(), encoded.as_str())
                        .map_err(error)?;
                    if policy.evaluate(Some(&row.actor), req, None).code
                        == DecisionCode::AcknowledgementRequired
                    {
                        let token = fresh_token().map_err(|_| invalid())?;
                        let approval_key = hashed(token.as_str()).ok_or_else(invalid)?;
                        let expires = now.checked_add(APPROVAL_TTL).ok_or_else(invalid)?;
                        let approval = serde_json::to_string(&ApprovalRow {
                            owner_hash: key.clone(),
                            request_hash: request_digest(req),
                            expires,
                        })
                        .map_err(error)?;
                        {
                            let mut approvals = tx.open_table(APPROVALS).map_err(error)?;
                            if approvals.len().map_err(error)? > MAX_APPROVALS {
                                return Err(invalid());
                            }
                            let mut expired = Vec::new();
                            let mut same_session = 0;
                            for result in approvals.iter().map_err(error)? {
                                let (id, item) = result.map_err(error)?;
                                let old: ApprovalRow =
                                    serde_json::from_str(item.value()).map_err(error)?;
                                if now >= old.expires {
                                    expired.push(id.value().to_owned());
                                } else if old.owner_hash == key {
                                    same_session += 1;
                                }
                            }
                            for id in expired {
                                approvals.remove(id.as_str()).map_err(error)?;
                            }
                            if approvals.len().map_err(error)? >= MAX_APPROVALS
                                || same_session >= MAX_APPROVALS_PER_SESSION
                            {
                                return Err(io::Error::new(
                                    io::ErrorKind::OutOfMemory,
                                    "durable fixture confirmation cap",
                                ));
                            }
                            if approvals
                                .get(approval_key.as_str())
                                .map_err(error)?
                                .is_some()
                            {
                                return Err(invalid());
                            }
                            approvals
                                .insert(approval_key.as_str(), approval.as_str())
                                .map_err(error)?;
                        }
                        result = Some(token);
                    }
                }
            }
        }
        tx.commit().map_err(error)?;
        Ok(result)
    }

    pub fn grant_recorded_session(
        &mut self,
        policy: &Policy,
        token: &str,
        req: &PreparedRequest,
        approval: Option<&str>,
        journal: &mut FileAuditJournal,
        now: u64,
    ) -> io::Result<Option<AuditedPermit>> {
        let mut tx = self.db.begin_write().map_err(error)?;
        tx.set_two_phase_commit(true);
        let mut identity: Option<(String, Actor)> = None;
        if let Some(key) = hashed(token) {
            let mut sessions = tx.open_table(SESSIONS).map_err(error)?;
            let raw = sessions
                .get(key.as_str())
                .map_err(error)?
                .map(|g| g.value().to_owned());
            if let Some(raw) = raw {
                let mut row: SessionRow = serde_json::from_str(&raw).map_err(error)?;
                if now < row.last_seen || now >= row.expires || now - row.last_seen >= row.idle {
                    sessions.remove(key.as_str()).map_err(error)?;
                } else {
                    row.last_seen = now;
                    let encoded = serde_json::to_string(&row).map_err(error)?;
                    sessions
                        .insert(key.as_str(), encoded.as_str())
                        .map_err(error)?;
                    identity = Some((key, row.actor));
                }
            }
        }
        let mut acknowledgement = None;
        if let Some(approval_key) = approval.and_then(hashed) {
            let mut table = tx.open_table(APPROVALS).map_err(error)?;
            // Consume even mismatched attempts before grant decision and audit.
            let raw = table
                .remove(approval_key.as_str())
                .map_err(error)?
                .map(|g| g.value().to_owned());
            if let (Some(raw), Some((key, actor))) = (raw, identity.as_ref()) {
                let approval: ApprovalRow = serde_json::from_str(&raw).map_err(error)?;
                if approval.owner_hash == *key
                    && approval.request_hash == request_digest(req)
                    && now < approval.expires
                {
                    acknowledgement = Some(Acknowledgement::bind(actor, req));
                }
            }
        }
        tx.commit().map_err(error)?;
        grant_recorded(
            policy,
            identity.as_ref().map(|(_, actor)| actor),
            req,
            acknowledgement.as_ref(),
            journal,
        )
    }
}
