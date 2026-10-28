//! P8dq: offline, atomic PRIVATE staging of two independently verified
//! Supervisor App-only archives. No App activation, secrets, mounts or devices.
use crate::app_backup_import::safe_slug;
use crate::app_backup_stage::stage_inactive_app;
use anyhow::{Context, Result, bail};
use serde::Serialize;
use std::{
    ffi::CString,
    fs::{self, File, OpenOptions, Permissions},
    io::{Read, Write},
    os::fd::AsRawFd,
    os::unix::ffi::OsStrExt,
    os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
    path::Path,
};
const PREFIX: &str = ".inactive-pair-";
const MARKER: &str = ".hars-pair-lease-v1";
const MAGIC: &[u8] = b"HARS_INACTIVE_PAIR_STAGE_V1\n";

fn private_directory(path: &Path) -> Result<()> {
    let m = fs::symlink_metadata(path)?;
    if !m.is_dir()
        || m.file_type().is_symlink()
        || m.uid() != nix::unistd::Uid::current().as_raw()
        || m.permissions().mode() & 0o077 != 0
    {
        bail!("owned private 0700 directory required");
    }
    Ok(())
}
fn lock_directory(path: &Path) -> Result<File> {
    let dir = OpenOptions::new()
        .read(true)
        .custom_flags(nix::libc::O_DIRECTORY | nix::libc::O_NOFOLLOW)
        .open(path)?;
    if unsafe { nix::libc::flock(dir.as_raw_fd(), nix::libc::LOCK_EX) } != 0 {
        return Err(std::io::Error::last_os_error()).context("pair lease lock");
    }
    let mut marker = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path.join(MARKER))?;
    marker.write_all(MAGIC)?;
    marker.sync_all()?;
    dir.sync_all()?;
    Ok(dir)
}
fn reap_owned_interrupted(root: &Path) -> Result<()> {
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        if !entry.file_name().to_string_lossy().starts_with(PREFIX) {
            continue;
        }
        let path = entry.path();
        if private_directory(&path).is_err() {
            continue; // never touch unknown, unowned or less-private paths
        }
        let dir = OpenOptions::new()
            .read(true)
            .custom_flags(nix::libc::O_DIRECTORY | nix::libc::O_NOFOLLOW)
            .open(&path)?;
        if unsafe { nix::libc::flock(dir.as_raw_fd(), nix::libc::LOCK_EX | nix::libc::LOCK_NB) }
            != 0
        {
            if std::io::Error::last_os_error().kind() == std::io::ErrorKind::WouldBlock {
                continue; // active pair transaction
            }
            bail!("pair lease acquisition failed");
        }
        let marker = match OpenOptions::new()
            .read(true)
            .custom_flags(nix::libc::O_NOFOLLOW | nix::libc::O_NONBLOCK)
            .open(path.join(MARKER))
        {
            Ok(f) => f,
            Err(e)
                if e.kind() == std::io::ErrorKind::NotFound
                    || e.raw_os_error() == Some(nix::libc::ELOOP) =>
            {
                continue;
            }
            Err(e) => return Err(e.into()),
        };
        let m = marker.metadata()?;
        if !m.is_file()
            || m.nlink() != 1
            || m.uid() != nix::unistd::Uid::current().as_raw()
            || m.len() != MAGIC.len() as u64
            || m.permissions().mode() & 0o077 != 0
        {
            continue;
        }
        let mut content = Vec::new();
        marker.take(128).read_to_end(&mut content)?;
        if content != MAGIC {
            continue;
        }
        fs::remove_dir_all(&path).context("reap only marked interrupted pair")?;
        File::open(root)?.sync_all()?;
    }
    Ok(())
}
fn publish_exclusive(from: &Path, to: &Path) -> Result<()> {
    let old = CString::new(from.as_os_str().as_bytes())?;
    let new = CString::new(to.as_os_str().as_bytes())?;
    if unsafe {
        nix::libc::renameat2(
            nix::libc::AT_FDCWD,
            old.as_ptr(),
            nix::libc::AT_FDCWD,
            new.as_ptr(),
            nix::libc::RENAME_NOREPLACE,
        )
    } != 0
    {
        return Err(std::io::Error::last_os_error()).context("pair already published");
    }
    Ok(())
}
fn valid_digest(digest: &str) -> bool {
    digest.len() == 64
        && digest
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
#[derive(Debug, Serialize)]
pub struct PairReceipt {
    pub app_count: usize,
    pub files: usize,
    pub bytes: u64,
    pub pair_id: String,
    pub atomic_private_pair_published: bool,
    pub app_runtime_started: bool,
    pub live_data_modified: bool,
    pub production_ready: bool,
}
/// The caller must supply an existing private SHA escrow and an independent
/// precreated owned 0700 destination. Neither source may be the destination.
pub fn stage_inactive_pair(
    escrow: &Path,
    first: (&str, &str),
    second: (&str, &str),
    target: &Path,
    pair_id: &str,
) -> Result<PairReceipt> {
    private_directory(escrow)?;
    private_directory(target)?;
    // Lexical inequality is not enough: `escrow/../escrow`, a symlinked
    // ancestor, or a nested output can alias the immutable source directory.
    // Refuse either ancestry direction before any orphan reaping or staging.
    let source_root = fs::canonicalize(escrow).context("canonical source escrow")?;
    let output_root = fs::canonicalize(target).context("canonical pair output")?;
    if source_root.starts_with(&output_root)
        || output_root.starts_with(&source_root)
        || !safe_slug(pair_id)
        || !safe_slug(first.0)
        || !safe_slug(second.0)
        || first.0 == second.0
        || !valid_digest(first.1)
        || !valid_digest(second.1)
        || first.1 == second.1
    {
        bail!("invalid or duplicate App pair");
    }
    let final_dir = target.join(format!("pair-{pair_id}"));
    if fs::symlink_metadata(&final_dir).is_ok() {
        bail!("refuse to replace published App pair");
    }
    reap_owned_interrupted(target)?;
    let temp = tempfile::Builder::new().prefix(PREFIX).tempdir_in(target)?;
    fs::set_permissions(temp.path(), Permissions::from_mode(0o700))?;
    private_directory(temp.path())?;
    let _lease = lock_directory(temp.path())?;
    let source1 = escrow.join(format!("{}.tar", first.1));
    let source2 = escrow.join(format!("{}.tar", second.1));
    let a = stage_inactive_app(&source1, first.1, first.0, temp.path())?;
    #[cfg(debug_assertions)]
    {
        if std::env::var("HARS_P8DQ_FAIL_AFTER_FIRST").as_deref() == Ok("1") {
            bail!("lab-only fault after first App");
        }
        if let Ok(value) = std::env::var("HARS_P8DQ_PAUSE_AFTER_FIRST_MS") {
            let millis: u64 = value.parse().context("invalid lab pause")?;
            if millis > 30_000 {
                bail!("lab pause exceeds bound");
            }
            std::thread::sleep(std::time::Duration::from_millis(millis));
        }
    }
    let b = stage_inactive_app(&source2, second.1, second.0, temp.path())?;
    #[cfg(debug_assertions)]
    if std::env::var("HARS_P8DQ_FAIL_AFTER_SECOND").as_deref() == Ok("1") {
        bail!("lab-only fault after second App");
    }
    File::open(temp.path())?.sync_all()?;
    publish_exclusive(temp.path(), &final_dir)?;
    File::open(target)?.sync_all()?;
    fs::remove_file(final_dir.join(MARKER))?;
    File::open(&final_dir)?.sync_all()?;
    File::open(target)?.sync_all()?;
    Ok(PairReceipt {
        app_count: 2,
        files: a.files + b.files,
        bytes: a.file_bytes + b.file_bytes,
        pair_id: pair_id.to_owned(),
        atomic_private_pair_published: true,
        app_runtime_started: false,
        live_data_modified: false,
        production_ready: false,
    })
}
