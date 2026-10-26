//! Defensive read-only inspection of an UNPROTECTED official-style HAOS tar.
//! Never decrypt SecureTar, unpack arbitrary files, run automations or expose
//! passwords. Only the Core entity registry enters the private Rust stage.
use crate::registry_stage::StrictValue;
use anyhow::{Context, Result, bail};
use flate2::read::MultiGzDecoder;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs::OpenOptions,
    io::{Cursor, Read, Seek, SeekFrom},
    os::unix::fs::MetadataExt,
    os::unix::fs::OpenOptionsExt,
    path::Path,
};
use tar::Archive;

const MAX_OUTER_BYTES: u64 = 1_073_741_824;
const MAX_PART_BYTES: u64 = 134_217_728;
const MAX_INNER_TOTAL: u64 = 536_870_912;
const MAX_REGISTRY_BYTES: u64 = 25_165_824;
const MAX_AUTOMATIONS_BYTES: u64 = 4 * 1024 * 1024;
const MAX_MANIFEST_BYTES: u64 = 1_048_576;
const MAX_OUTER_ENTRIES: usize = 25_000;
const MAX_INNER_ENTRIES: usize = 50_000;

fn canonical_name(raw: &Path, is_dir: bool) -> Result<String> {
    let display = raw.to_str().context("nonutf8 archive name")?;
    let name = display.strip_prefix("./").unwrap_or(display);
    // The official Supervisor tar writer includes POSIX directory entries
    // ending in "/". Drop exactly one terminal slash ONLY for directories.
    // All remaining empty components, traversal and alias collisions fail.
    let value = if is_dir {
        name.strip_suffix('/').unwrap_or(name)
    } else {
        name
    };
    if value.is_empty()
        || value.len() > 512
        || value.starts_with('/')
        || value.chars().any(|ch| ch.is_control() || ch == '\\')
        || value
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        bail!("unsafe archive member");
    }
    Ok(value.to_string())
}

fn regular_or_directory(kind: tar::EntryType) -> Result<()> {
    if !(kind.is_file() || kind.is_dir()) {
        bail!("archive links and special files are forbidden")
    }
    Ok(())
}
fn bounded_read<R: Read>(stream: &mut R, max: u64) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    stream.take(max + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > max {
        bail!("archive part exceeds bound")
    }
    Ok(bytes)
}

pub struct SourceRegistry {
    pub raw: Vec<u8>,
    pub archive_sha256: String,
    /// Uninterpreted original HA automations, never compiled or executed.
    /// May contain sensitive user configuration: never print to stdout/logs.
    pub automations_yaml: Option<Vec<u8>>,
}

/// Read a source archive without writing extracted secrets or following links.
pub fn registry_from_unprotected_backup(source: &Path) -> Result<SourceRegistry> {
    let mut archive_file = OpenOptions::new()
        .read(true)
        .custom_flags(nix::libc::O_NOFOLLOW | nix::libc::O_NONBLOCK | nix::libc::O_CLOEXEC)
        .open(source)
        .context("unavailable archive")?;
    let meta = archive_file.metadata()?;
    if !meta.is_file() || meta.nlink() != 1 || meta.len() < 1024 || meta.len() > MAX_OUTER_BYTES {
        bail!("archive inode or size is unsupported")
    }
    // Hash only the exact opened inode. Reopen by cloning its descriptor,
    // never resolve a filename again after verification.
    let mut hash_reader = archive_file.try_clone()?;
    let mut digest = Sha256::new();
    let mut block = [0u8; 65536];
    loop {
        let n = hash_reader.read(&mut block)?;
        if n == 0 {
            break;
        }
        digest.update(&block[..n]);
    }
    drop(hash_reader);
    archive_file.seek(SeekFrom::Start(0))?;
    let hash = hex::encode(digest.finalize());
    let mut archive = Archive::new(archive_file);
    let mut seen = BTreeSet::new();
    let mut manifest: Option<Vec<u8>> = None;
    let mut core: Option<Vec<u8>> = None;
    for (i, entry) in archive.entries()?.enumerate() {
        if i >= MAX_OUTER_ENTRIES {
            bail!("too many outer entries")
        }
        let mut entry = entry?;
        let kind = entry.header().entry_type();
        if kind.is_dir() && matches!(entry.path()?.to_str(), Some(".") | Some("./")) {
            continue;
        }
        regular_or_directory(kind)?;
        let name = canonical_name(&entry.path()?, kind.is_dir())?;
        if !seen.insert(name.clone()) {
            bail!("duplicate outer member")
        }
        if name == "backup.json" && kind.is_file() {
            manifest = Some(bounded_read(&mut entry, MAX_MANIFEST_BYTES)?);
        }
        if (name == "homeassistant.tar.gz" || name == "homeassistant.tar") && kind.is_file() {
            if core.is_some() {
                bail!("ambiguous Core payload")
            }
            core = Some(bounded_read(&mut entry, MAX_PART_BYTES)?);
        }
    }
    let final_info = archive.into_inner().metadata()?;
    if final_info.len() != meta.len()
        || final_info.mtime() != meta.mtime()
        || final_info.mtime_nsec() != meta.mtime_nsec()
    {
        bail!("archive changed while inspected")
    }
    let raw_manifest = manifest.context("backup manifest missing")?;
    let StrictValue(manifest): StrictValue =
        serde_json::from_slice(&raw_manifest).context("untrusted backup metadata")?;
    let meta = manifest
        .as_object()
        .context("backup manifest is not an object")?;
    if meta.get("protected") != Some(&Value::Bool(false)) {
        bail!("encrypted/ambiguous archives require official Supervisor decryption")
    }
    if meta.get("type").and_then(Value::as_str) != Some("full") {
        bail!("only a declared full unprotected archive is supported")
    }
    let inner = core.context("missing Home Assistant Core backup payload")?;
    if inner.starts_with(b"SecureTar") {
        bail!("encrypted payload requires official Supervisor")
    }
    // The official Supervisor emits .tar or .tar.gz. Detect framing rather than
    // guessing a suffix after the archive was opened.
    let reader: Box<dyn Read> = if inner.starts_with(&[0x1f, 0x8b]) {
        Box::new(MultiGzDecoder::new(Cursor::new(inner)))
    } else {
        Box::new(Cursor::new(inner))
    };
    let bounded = reader.take(MAX_INNER_TOTAL + 1);
    let mut nested = Archive::new(bounded);
    let mut inner_seen = BTreeSet::new();
    let mut registry = None;
    let mut automations = None;
    let mut declared_total = 0u64;
    for (i, entry) in nested.entries()?.enumerate() {
        if i >= MAX_INNER_ENTRIES {
            bail!("too many Core archive entries")
        }
        let mut entry = entry?;
        let kind = entry.header().entry_type();
        if kind.is_dir() && matches!(entry.path()?.to_str(), Some(".") | Some("./")) {
            continue;
        }
        regular_or_directory(kind)?;
        let name = canonical_name(&entry.path()?, kind.is_dir())?;
        if !inner_seen.insert(name.clone()) {
            bail!("duplicate Core archive member")
        }
        declared_total = declared_total
            .checked_add(entry.size())
            .context("Core archive size overflow")?;
        if declared_total > MAX_INNER_TOTAL {
            bail!("Core archive too large")
        }
        if (name == "data/.storage/core.entity_registry" || name == ".storage/core.entity_registry")
            && kind.is_file()
        {
            // Official Supervisor backs up Core config as "data/".
            // Legacy archives may have direct config entries. Never allow
            // both forms: ambiguous identities must fail closed.
            if registry.is_some() {
                bail!("conflicting Core registry paths in backup")
            }
            registry = Some(bounded_read(&mut entry, MAX_REGISTRY_BYTES)?);
        }
        if (name == "data/automations.yaml" || name == "automations.yaml") && kind.is_file() {
            if automations.is_some() {
                bail!("conflicting Core automation paths in backup")
            }
            let bytes = bounded_read(&mut entry, MAX_AUTOMATIONS_BYTES)?;
            if bytes.is_empty() || std::str::from_utf8(&bytes).is_err() {
                bail!("invalid bounded automation source")
            }
            // Byte-exact custody only; untrusted YAML is NEVER run or interpreted.
            automations = Some(bytes);
        }
    }
    // Reading tar entries stops at the tar end-of-archive marker. Consume
    // the remaining gzip stream to its CRC/footer, while rejecting nonzero
    // trailing data, concatenated archives and excessive decompression.
    let mut tail = nested.into_inner();
    let mut buffer = [0u8; 32768];
    loop {
        let n = tail
            .read(&mut buffer)
            .context("invalid truncated gzip/footer")?;
        if n == 0 {
            break;
        }
        if buffer[..n].iter().any(|&byte| byte != 0) {
            bail!("unexpected nonzero bytes after Core archive terminator")
        }
    }
    if tail.limit() == 0 {
        bail!("Core decompression limit reached")
    }
    let raw = registry.context("Core entity registry absent from official backup")?;
    // Reject invalid registries before the caller starts a durable transaction.
    crate::registry_stage::parse_export(&raw)?;
    Ok(SourceRegistry {
        raw,
        archive_sha256: hash,
        automations_yaml: automations,
    })
}
