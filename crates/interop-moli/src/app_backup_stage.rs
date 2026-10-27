//! P8cy: OFFLINE reconstruction of ONE verified real HAOS App-only backup into
//! a fresh owned private native directory. NO Supervisor/Core/App activation,
//! OCI image switch, autorun or general production restore.
use crate::app_backup_import::{canonical, inspect_unprotected_app_only, safe_slug};
use anyhow::{Context, Result, bail};
use flate2::read::MultiGzDecoder;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    ffi::CString,
    fs::{self, File, OpenOptions, Permissions},
    io::{Cursor, Read, Write},
    os::unix::ffi::OsStrExt,
    os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};
use tar::Archive;

const MAX_PART: u64 = 134_217_728;
const MAX_TOTAL: u64 = 268_435_456;
const MAX_FILES: usize = 20_000;

fn private_owned_directory(path: &Path) -> Result<()> {
    let info = fs::symlink_metadata(path).context("private native directory missing")?;
    if !info.is_dir()
        || info.file_type().is_symlink()
        || info.uid() != nix::unistd::Uid::current().as_raw()
        || info.permissions().mode() & 0o077 != 0
    {
        bail!("private owned 0700 directory required")
    }
    Ok(())
}

fn verify_content_address(source: &Path, expected: &str) -> Result<()> {
    let mut f = OpenOptions::new()
        .read(true)
        .custom_flags(nix::libc::O_NOFOLLOW | nix::libc::O_NONBLOCK | nix::libc::O_CLOEXEC)
        .open(source)?;
    let metadata = f.metadata()?;
    if !metadata.is_file() || metadata.nlink() != 1 || metadata.len() > 1_073_741_824 {
        bail!("untrusted App escrow inode")
    }
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let n = f.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    if hex::encode(hash.finalize()) != expected {
        bail!("original source SHA changed")
    }
    Ok(())
}

fn publish_no_replace(from: &Path, to: &Path) -> Result<()> {
    let old = CString::new(from.as_os_str().as_bytes())?;
    let new = CString::new(to.as_os_str().as_bytes())?;
    // Linux exclusive rename refuses a pre-existing file OR empty directory.
    // Never silently replace a previous independent staging directory.
    let code = unsafe {
        nix::libc::renameat2(
            nix::libc::AT_FDCWD,
            old.as_ptr(),
            nix::libc::AT_FDCWD,
            new.as_ptr(),
            nix::libc::RENAME_NOREPLACE,
        )
    };
    if code != 0 {
        return Err(std::io::Error::last_os_error()).context("refuse existing target");
    }
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct InactiveStageReceipt {
    pub archive_sha256: String,
    pub app_slug: String,
    pub files: usize,
    pub file_bytes: u64,
    pub copied_as_private_regular_files: bool,
    pub app_image_installed: bool,
    pub app_runtime_started: bool,
    pub automation_executed: bool,
    pub live_data_modified: bool,
    pub production_ready: bool,
}

/// The input *must* already exist in private content-addressed escrow; the
/// caller enforces the exact 64-char digest. A single declared App is supported.
pub fn stage_inactive_app(
    source: &Path,
    digest: &str,
    slug: &str,
    target_root: &Path,
) -> Result<InactiveStageReceipt> {
    if !safe_slug(slug) {
        bail!("invalid selected App identifier")
    }
    private_owned_directory(target_root).context("output root")?;
    let info = inspect_unprotected_app_only(source)?;
    if info.app_count != 1 || info.archive_sha256 != digest {
        bail!("only one SHA-exact official App-only source supported")
    }
    verify_content_address(source, digest)?;
    let mut source_file = OpenOptions::new()
        .read(true)
        .custom_flags(nix::libc::O_NOFOLLOW | nix::libc::O_NONBLOCK | nix::libc::O_CLOEXEC)
        .open(source)?;
    let before = source_file.metadata()?;
    if !before.is_file() || before.nlink() != 1 {
        bail!("invalid App archive inode")
    }
    let payload = format!("{slug}.tar.gz");
    let mut outer = Archive::new(&mut source_file);
    let mut part = None;
    for entry in outer.entries()? {
        let entry = entry?;
        let kind = entry.header().entry_type();
        if kind.is_file() && canonical(&entry.path()?, false)? == payload {
            if part.is_some() {
                bail!("duplicate declared part")
            }
            let mut bytes = Vec::new();
            entry.take(MAX_PART + 1).read_to_end(&mut bytes)?;
            if bytes.len() as u64 > MAX_PART {
                bail!("bounded part exceeded")
            }
            part = Some(bytes);
        }
    }
    let part = part.context("selected App payload absent")?;
    if !part.starts_with(&[0x1f, 0x8b]) {
        bail!("encrypted or invalid selected App part")
    }
    let temp = tempfile::Builder::new()
        .prefix(".inactive-app-")
        .tempdir_in(target_root)?;
    // Tempfile may inherit a broader umask than 0700; protect it before
    // placing the first App byte, otherwise discard the temp destination.
    fs::set_permissions(temp.path(), Permissions::from_mode(0o700))?;
    private_owned_directory(temp.path()).context("temp dir")?;
    let reader = MultiGzDecoder::new(Cursor::new(part)).take(MAX_TOTAL + 1);
    let mut nested = Archive::new(reader);
    let mut seen = BTreeSet::new();
    let mut directories = BTreeSet::<PathBuf>::new();
    directories.insert(temp.path().to_path_buf());
    let mut files = 0usize;
    let mut copied = 0u64;
    for (index, entry) in nested.entries()?.enumerate() {
        if index >= MAX_FILES {
            bail!("too many App entries")
        }
        let mut entry = entry?;
        let kind = entry.header().entry_type();
        if kind.is_dir() && matches!(entry.path()?.to_str(), Some(".") | Some("./")) {
            continue;
        }
        if !(kind.is_file() || kind.is_dir()) {
            bail!("links/devices refused for native App staging")
        }
        let canonical = canonical(&entry.path()?, kind.is_dir())?;
        if !seen.insert(canonical.clone()) {
            bail!("duplicate native App relative path")
        }
        let relative = Path::new(&canonical);
        let output = temp.path().join(relative);
        if kind.is_dir() {
            fs::create_dir_all(&output)?;
            fs::set_permissions(&output, Permissions::from_mode(0o700))?;
            directories.insert(output);
        } else {
            let parent = output.parent().context("missing relative App parent")?;
            fs::create_dir_all(parent)?;
            // The temporary root itself is 0700, no other user can see files.
            // Lock the created parents before committing any staged files.
            let mut walk = temp.path().to_path_buf();
            for fragment in relative
                .parent()
                .context("missing App path parent")?
                .components()
            {
                walk.push(fragment.as_os_str());
                fs::set_permissions(&walk, Permissions::from_mode(0o700))?;
                directories.insert(walk.clone());
            }
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&output)?;
            let size = entry.size();
            copied = copied
                .checked_add(size)
                .context("native staged App overflow")?;
            if copied > MAX_TOTAL {
                bail!("App native staging too large")
            }
            let n = std::io::copy(&mut entry, &mut file)?;
            if n != size {
                bail!("truncated App file body")
            }
            file.flush()?;
            file.sync_all()?;
            files += 1;
        }
    }
    if files == 0 {
        bail!("empty App data cannot be staged")
    }
    let mut tail = nested.into_inner();
    let mut buf = [0u8; 16384];
    loop {
        let n = tail.read(&mut buf)?;
        if n == 0 {
            break;
        }
        if buf[..n].iter().any(|b| *b != 0) {
            bail!("App gzip trailing data invalid")
        }
    }
    if tail.limit() == 0 {
        bail!("App decompression limit reached")
    }
    // Re-check on-disk SHA after all extracted bytes have been durably staged.
    // A concurrent writer's corruption must NEVER cause a public stage.
    verify_content_address(source, digest)?;
    let after = source_file.metadata()?;
    if before.len() != after.len()
        || before.mtime() != after.mtime()
        || before.mtime_nsec() != after.mtime_nsec()
    {
        bail!("escrow source changed during stage")
    }
    if std::env::var("HARS_P8CY_FAIL_BEFORE_COMMIT").as_deref() == Ok("1") {
        bail!("disposable fault injected before atomic publish")
    }
    for d in directories.iter().rev() {
        File::open(d)?.sync_all()?;
    }
    let final_dir = target_root.join(format!("app-{slug}-{digest}"));
    publish_no_replace(temp.path(), &final_dir)?;
    File::open(target_root)?.sync_all()?;
    Ok(InactiveStageReceipt {
        archive_sha256: digest.to_owned(),
        app_slug: slug.to_owned(),
        files,
        file_bytes: copied,
        copied_as_private_regular_files: true,
        app_image_installed: false,
        app_runtime_started: false,
        automation_executed: false,
        live_data_modified: false,
        production_ready: false,
    })
}
