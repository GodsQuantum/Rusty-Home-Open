//! P8cw: bounded native read-only admission of a REAL Supervisor-selected
//! App-only unprotected PARTIAL backup, *not* App restore or migration.
//! Backups can contain private data: never return member names, values or
//! app manifests to logs. Preserve the original archive via existing escrow.
use crate::registry_stage::StrictValue;
use anyhow::{Context, Result, bail};
use flate2::read::MultiGzDecoder;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::OpenOptions,
    io::{Cursor, Read, Seek, SeekFrom},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
};
use tar::Archive;

const MAX_ARCHIVE: u64 = 1_073_741_824;
const MAX_PART: u64 = 134_217_728;
const MAX_MANIFEST: u64 = 1_048_576;
const MAX_DECOMPRESSED: u64 = 268_435_456;
const MAX_FILES: usize = 20_000;
const MAX_APPS: usize = 8;

pub(crate) fn canonical(path: &Path, dir: bool) -> Result<String> {
    let full = path.to_str().context("invalid tar name encoding")?;
    let name = full.strip_prefix("./").unwrap_or(full);
    let name = if dir {
        name.strip_suffix('/').unwrap_or(name)
    } else {
        name
    };
    if name.is_empty()
        || name.len() > 512
        || name.starts_with('/')
        || name.chars().any(|c| c.is_control() || c == '\\')
        || name
            .split('/')
            .any(|s| s.is_empty() || s == "." || s == "..")
    {
        bail!("unsafe tar member")
    }
    Ok(name.to_string())
}
fn read_bounded<R: Read>(source: &mut R, max: u64) -> Result<Vec<u8>> {
    let mut data = Vec::new();
    source.take(max + 1).read_to_end(&mut data)?;
    if data.len() as u64 > max {
        bail!("archive exceeds boundary")
    }
    Ok(data)
}
pub(crate) fn safe_slug(value: &str) -> bool {
    let b = value.as_bytes();
    !b.is_empty()
        && b.len() <= 120
        && b[0].is_ascii_lowercase()
        && b.iter()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'_' || *c == b'-')
}
fn scan_gzip_part(bytes: Vec<u8>) -> Result<(usize, u64)> {
    if !bytes.starts_with(&[0x1f, 0x8b]) {
        bail!("App part is not unprotected gzip")
    }
    // Bounded decoded stream. All tar entries must be ordinary files or dirs;
    // no symlink/device/hardlink, absolute paths, traversal, or duplicate.
    let reader = MultiGzDecoder::new(Cursor::new(bytes)).take(MAX_DECOMPRESSED + 1);
    let mut tar = Archive::new(reader);
    let mut seen = BTreeSet::new();
    let mut files = 0usize;
    let mut declared = 0u64;
    for (i, entry) in tar.entries()?.enumerate() {
        if i >= MAX_FILES {
            bail!("too many App members")
        }
        let entry = entry?;
        let kind = entry.header().entry_type();
        if kind.is_dir() && matches!(entry.path()?.to_str(), Some(".") | Some("./")) {
            continue;
        }
        if !(kind.is_file() || kind.is_dir()) {
            bail!("App backup links or devices refused")
        }
        if !seen.insert(canonical(&entry.path()?, kind.is_dir())?) {
            bail!("duplicate canonical App member")
        }
        declared = declared
            .checked_add(entry.size())
            .context("App byte overflow")?;
        if declared > MAX_DECOMPRESSED {
            bail!("App uncompressed bytes over limit")
        }
        if kind.is_file() {
            files += 1;
        }
    }
    // Ensure gzip CRC/footer are actually consumed and decompression is
    // bounded, not merely that tar parsing stopped at end markers.
    let mut remaining = tar.into_inner();
    let mut tail = [0u8; 16384];
    loop {
        let count = remaining.read(&mut tail).context("truncated gzip App")?;
        if count == 0 {
            break;
        }
        if tail[..count].iter().any(|b| *b != 0) {
            bail!("nonzero App archive trailing data")
        }
    }
    if remaining.limit() == 0 {
        bail!("App decompression exceeded bound")
    }
    if files == 0 {
        bail!("empty App part")
    }
    Ok((files, declared))
}

#[derive(Debug, Serialize)]
pub struct AppBackupInspection {
    pub archive_sha256: String,
    pub app_count: usize,
    pub files: usize,
    pub declared_payload_bytes: u64,
    pub all_payloads_unprotected_gzip: bool,
    pub manifest_partial: bool,
    pub core_migrated: bool,
    pub app_runtime_migrated: bool,
    pub actions_executable: bool,
    pub production_ready: bool,
}

/// Preflight a read-only O_NOFOLLOW inode and scan only bounded inert tar
/// structure. No payload bytes are returned or unpacked.
pub fn inspect_unprotected_app_only(source: &Path) -> Result<AppBackupInspection> {
    let mut input = OpenOptions::new()
        .read(true)
        .custom_flags(nix::libc::O_NOFOLLOW | nix::libc::O_NONBLOCK | nix::libc::O_CLOEXEC)
        .open(source)
        .context("unavailable safe App archive")?;
    let stat = input.metadata()?;
    if !stat.is_file() || stat.nlink() != 1 || stat.len() < 1024 || stat.len() > MAX_ARCHIVE {
        bail!("invalid App backup inode or size")
    }
    let mut hash = Sha256::new();
    let mut cloned = input.try_clone()?;
    let mut buf = [0u8; 65536];
    loop {
        let n = cloned.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hash.update(&buf[..n]);
    }
    drop(cloned);
    input.seek(SeekFrom::Start(0))?;
    let mut outer = Archive::new(input);
    let mut names = BTreeSet::new();
    let mut manifest = None;
    let mut parts: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    for (idx, entry) in outer.entries()?.enumerate() {
        if idx >= MAX_FILES {
            bail!("too many outer members")
        }
        let mut entry = entry?;
        let kind = entry.header().entry_type();
        if kind.is_dir() && matches!(entry.path()?.to_str(), Some(".") | Some("./")) {
            continue;
        }
        if !(kind.is_file() || kind.is_dir()) {
            bail!("outer links or devices forbidden")
        }
        let name = canonical(&entry.path()?, kind.is_dir())?;
        if !names.insert(name.clone()) {
            bail!("duplicate outer archive member")
        }
        if name == "backup.json" && kind.is_file() {
            manifest = Some(read_bounded(&mut entry, MAX_MANIFEST)?);
        } else if kind.is_file() && name.ends_with(".tar.gz") {
            if parts.len() >= MAX_APPS {
                bail!("too many App payloads")
            }
            parts.insert(name, read_bounded(&mut entry, MAX_PART)?);
        } else if kind.is_file() {
            bail!("unsupported outer App archive member")
        }
    }
    let final_stat = outer.into_inner().metadata()?;
    if stat.len() != final_stat.len()
        || stat.mtime() != final_stat.mtime()
        || stat.mtime_nsec() != final_stat.mtime_nsec()
    {
        bail!("App archive changed during inspection")
    }
    let raw = manifest.context("missing official backup.json")?;
    let StrictValue(value): StrictValue = serde_json::from_slice(&raw)?;
    let obj = value.as_object().context("invalid App backup metadata")?;
    if obj.get("type") != Some(&Value::String("partial".into()))
        || obj.get("protected") != Some(&Value::Bool(false))
    {
        bail!("App-only archive requires explicitly unprotected partial Supervisor backup")
    }
    let entries = obj
        .get("addons")
        .and_then(Value::as_array)
        .context("App-only metadata must declare Apps")?;
    if entries.is_empty() || entries.len() > MAX_APPS {
        bail!("invalid App backup count")
    }
    let mut slugs = BTreeSet::new();
    for item in entries {
        let slug = if let Some(s) = item.as_str() {
            s
        } else {
            item.get("slug")
                .and_then(Value::as_str)
                .context("invalid App slug")?
        };
        if !safe_slug(slug) || !slugs.insert(slug.to_owned()) {
            bail!("duplicate or unsafe App slug")
        }
    }
    if slugs.len() != parts.len() {
        bail!("App payloads mismatch metadata")
    }
    let mut files = 0usize;
    let mut bytes = 0u64;
    for slug in slugs {
        let expected = format!("{slug}.tar.gz");
        let part = parts
            .remove(&expected)
            .context("missing exact declared App part")?;
        let (f, b) = scan_gzip_part(part)?;
        files = files.checked_add(f).context("App files overflow")?;
        bytes = bytes.checked_add(b).context("App size overflow")?;
    }
    if !parts.is_empty() {
        bail!("undeclared App part")
    }
    Ok(AppBackupInspection {
        archive_sha256: hex::encode(hash.finalize()),
        app_count: entries.len(),
        files,
        declared_payload_bytes: bytes,
        all_payloads_unprotected_gzip: true,
        manifest_partial: true,
        core_migrated: false,
        app_runtime_migrated: false,
        actions_executable: false,
        production_ready: false,
    })
}
