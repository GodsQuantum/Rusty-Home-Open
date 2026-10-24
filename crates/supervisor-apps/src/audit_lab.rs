//! LAB-only durable audit sink for P8i policy decisions.
//! No production identity, encryption, external audit integrity anchor,
//! WORM storage, nor live production Supervisor execution.

use crate::managed_apps::PreparedRequest;
use crate::rbac_lab::{Acknowledgement, Actor, AuditRecord, Decision, Policy};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_EXISTING_BYTES: u64 = 8 * 1024 * 1024;

pub trait AuditSink {
    /// Must finish a durable synchronous append, or return an error.
    fn append_sync(&mut self, event: &AuditRecord) -> io::Result<()>;
}

/// Caller MUST use this result rather than Policy::evaluate for any
/// future protected operation. A failed audit prevents an allow verdict.
pub fn authorize_recorded(
    policy: &Policy,
    actor: Option<&Actor>,
    request: &PreparedRequest,
    acknowledgement: Option<&Acknowledgement>,
    sink: &mut impl AuditSink,
) -> io::Result<Decision> {
    let decision = policy.evaluate(actor, request, acknowledgement);
    sink.append_sync(&decision.audit)?;
    Ok(decision)
}

/// Single-use laboratory mutation capability. Fields are private and this type
/// is deliberately not Clone: the Supervisor transport consumes the permit.
/// Only grant_recorded may construct it, after synchronous journal success.
/// NOT production identity authentication or cross-process replay protection.
#[derive(Debug)]
pub struct AuditedPermit {
    request: PreparedRequest,
    /// Recorded, authorized fixture subject; cannot be supplied to transport separately.
    actor_subject: String,
}

impl AuditedPermit {
    pub(crate) fn request(&self) -> &PreparedRequest {
        &self.request
    }
    pub(crate) fn actor_subject(&self) -> &str {
        &self.actor_subject
    }
}

/// Produce a transport capability only after full policy evaluation and a
/// synchronous successful append to a real FileAuditJournal (not an arbitrary
/// caller-provided AuditSink implementation).
pub fn grant_recorded(
    policy: &Policy,
    actor: Option<&Actor>,
    request: &PreparedRequest,
    acknowledgement: Option<&Acknowledgement>,
    journal: &mut FileAuditJournal,
) -> io::Result<Option<AuditedPermit>> {
    let decision = authorize_recorded(policy, actor, request, acknowledgement, journal)?;
    Ok(if decision.allowed() {
        actor.map(|trusted| AuditedPermit {
            request: request.clone(),
            actor_subject: trusted.subject().to_owned(),
        })
    } else {
        None
    })
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    time_unix_ms: u64,
    event: AuditRecord,
    prev_hash: String,
    hash: String,
}

/// Advisory Linux lock, released on early return and panic. This protects only
/// cooperating writers; it is not a WORM or hostile-root security boundary.
struct FlockGuard(std::os::fd::RawFd);

impl FlockGuard {
    fn take(file: &File, mode: i32) -> io::Result<Self> {
        use std::os::fd::AsRawFd;
        if unsafe { nix::libc::flock(file.as_raw_fd(), mode) } == -1 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self(file.as_raw_fd()))
    }
}

impl Drop for FlockGuard {
    fn drop(&mut self) {
        // File outlives this guard in every use; unlock on all early returns.
        unsafe { nix::libc::flock(self.0, nix::libc::LOCK_UN) };
    }
}

fn chained_hash(previous: &str, stamp: u64, event: &AuditRecord) -> io::Result<String> {
    use sha2::{Digest, Sha256};
    let mut hash = Sha256::new();
    hash.update(b"HARS-LAB-AUDIT-V1");
    hash.update(previous.as_bytes());
    hash.update(stamp.to_be_bytes());
    hash.update(serde_json::to_vec(event).map_err(|_| invalid())?);
    let mut encoded = String::with_capacity(64);
    for octet in hash.finalize() {
        use std::fmt::Write as _;
        write!(encoded, "{octet:02x}").expect("write hexadecimal to String");
    }
    Ok(encoded)
}

fn read_and_verify(file: &mut File) -> io::Result<(String, u64)> {
    use std::io::{Seek, SeekFrom};
    let len = file.metadata()?.len();
    if len > MAX_EXISTING_BYTES {
        return Err(insecure());
    }
    file.seek(SeekFrom::Start(0))?;
    let mut raw = Vec::new();
    file.read_to_end(&mut raw)?;
    if raw.len() as u64 != len {
        return Err(invalid());
    }
    let mut previous = "0".repeat(64);
    if !raw.is_empty() {
        if !raw.ends_with(b"\n") {
            return Err(invalid());
        }
        for line in raw[..raw.len() - 1].split(|&c| c == b'\n') {
            if line.is_empty() {
                return Err(invalid());
            }
            // Reject duplicate JSON keys before typed deserialization.
            let strict = serde_json::from_slice::<crate::NoDuplicates>(line)
                .map_err(|_| invalid())?
                .0;
            let entry: Entry = serde_json::from_value(strict).map_err(|_| invalid())?;
            if entry.prev_hash != previous
                || entry.hash != chained_hash(&previous, entry.time_unix_ms, &entry.event)?
            {
                return Err(invalid());
            }
            previous = entry.hash;
        }
    }
    Ok((previous, len))
}

pub struct FileAuditJournal {
    file: File,
}

impl FileAuditJournal {
    /// Caller supplies private, owner-only 0700 directory. Linux-specific lab
    /// format v1: older unchained JSONL journals are deliberately rejected.
    pub fn open(path: &Path) -> io::Result<Self> {
        let parent = path.parent().ok_or_else(insecure)?;
        let meta = fs::symlink_metadata(parent)?;
        if meta.file_type().is_symlink()
            || !meta.is_dir()
            || meta.uid() != nix::unistd::geteuid().as_raw()
            || meta.permissions().mode() & 0o777 != 0o700
        {
            return Err(insecure());
        }
        let mut file = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .mode(0o600)
            .custom_flags(nix::libc::O_NOFOLLOW | nix::libc::O_CLOEXEC)
            .open(path)?;
        let fmeta = file.metadata()?;
        if !fmeta.is_file()
            || fmeta.uid() != nix::unistd::geteuid().as_raw()
            || fmeta.nlink() != 1
            || fmeta.permissions().mode() & 0o777 != 0o600
        {
            return Err(insecure());
        }
        {
            let _lock = FlockGuard::take(&file, nix::libc::LOCK_SH)?;
            read_and_verify(&mut file)?;
        }
        // Ensure a newly created directory entry survives a crash; this does
        // not make the underlying storage power-loss-proof.
        File::open(parent)?.sync_all()?;
        Ok(Self { file })
    }
}

impl AuditSink for FileAuditJournal {
    fn append_sync(&mut self, event: &AuditRecord) -> io::Result<()> {
        let _lock = FlockGuard::take(&self.file, nix::libc::LOCK_EX)?;
        // Verify the complete chain under the lock, including events appended
        // by other processes after we opened this descriptor.
        let (prev_hash, len) = read_and_verify(&mut self.file)?;
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| invalid())?
            .as_millis();
        let stamp = u64::try_from(stamp).map_err(|_| invalid())?;
        let line = serde_json::to_vec(&Entry {
            time_unix_ms: stamp,
            event: event.clone(),
            hash: chained_hash(&prev_hash, stamp, event)?,
            prev_hash,
        })
        .map_err(|_| invalid())?;
        if len
            .checked_add(u64::try_from(line.len()).map_err(|_| invalid())? + 1)
            .is_none_or(|new_size| new_size > MAX_EXISTING_BYTES)
        {
            return Err(insecure());
        }
        self.file.write_all(&line)?;
        self.file.write_all(b"\n")?;
        self.file.sync_data()
    }
}

fn insecure() -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, "unsafe audit journal")
}
fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "invalid audit journal")
}
