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
    os::fd::AsRawFd,
    os::unix::ffi::OsStrExt,
    os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};
use tar::Archive;

const MAX_PART: u64 = 134_217_728;
const MAX_TOTAL: u64 = 268_435_456;
const MAX_FILES: usize = 20_000;
const STAGE_LEASE: &str = ".hars-stage-lease-v1";
const STAGE_LEASE_MAGIC: &[u8] = b"HARS_INACTIVE_APP_STAGE_V1\n";

/// Reap only our own interrupted temporary App stages. A directory flock
/// excludes live concurrent writers, while the private lease marker prevents
/// deleting arbitrary similarly named user directories.
fn reap_interrupted_stages(root: &Path) -> Result<()> {
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let name = entry.file_name();
        if !name.to_string_lossy().starts_with(".inactive-app-") {
            continue;
        }
        let path = entry.path();
        let info = match fs::symlink_metadata(&path) {
            Ok(info) => info,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error).context("inspect private stage"),
        };
        if !info.is_dir()
            || info.file_type().is_symlink()
            || info.uid() != nix::unistd::Uid::current().as_raw()
            || info.permissions().mode() & 0o077 != 0
        {
            continue;
        }
        let directory = match OpenOptions::new()
            .read(true)
            .custom_flags(nix::libc::O_DIRECTORY | nix::libc::O_NOFOLLOW)
            .open(&path)
        {
            Ok(dir) => dir,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error).context("open private stage"),
        };
        let rc = unsafe {
            nix::libc::flock(
                directory.as_raw_fd(),
                nix::libc::LOCK_EX | nix::libc::LOCK_NB,
            )
        };
        if rc != 0 {
            let error = std::io::Error::last_os_error();
            if error.kind() == std::io::ErrorKind::WouldBlock {
                continue; // another active native stage owns the directory
            }
            return Err(error).context("cannot lock private stage");
        }
        let marker = match OpenOptions::new()
            .read(true)
            .custom_flags(nix::libc::O_NOFOLLOW | nix::libc::O_NONBLOCK)
            .open(path.join(STAGE_LEASE))
        {
            Ok(file) => file,
            Err(error)
                if error.kind() == std::io::ErrorKind::NotFound
                    || error.raw_os_error() == Some(nix::libc::ELOOP) =>
            {
                continue;
            }
            Err(error) => return Err(error).context("private stage lease"),
        };
        let meta = marker.metadata()?;
        if !meta.is_file()
            || meta.nlink() != 1
            || meta.uid() != nix::unistd::Uid::current().as_raw()
            || meta.len() != STAGE_LEASE_MAGIC.len() as u64
            || meta.permissions().mode() & 0o077 != 0
        {
            continue;
        }
        let mut bytes = Vec::new();
        marker.take(128).read_to_end(&mut bytes)?;
        if bytes != STAGE_LEASE_MAGIC {
            continue;
        }
        // Unix remove_dir_all does not traverse symlinks. The parent is an
        // explicit caller-owned private 0700 directory, never a live mount.
        fs::remove_dir_all(&path).context("reap interrupted private stage")?;
        File::open(root)?.sync_all()?;
    }
    Ok(())
}

fn lock_private_stage(temp: &Path) -> Result<File> {
    let directory = OpenOptions::new()
        .read(true)
        .custom_flags(nix::libc::O_DIRECTORY | nix::libc::O_NOFOLLOW)
        .open(temp)?;
    if unsafe { nix::libc::flock(directory.as_raw_fd(), nix::libc::LOCK_EX) } != 0 {
        return Err(std::io::Error::last_os_error()).context("stage directory lock");
    }
    let mut marker = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(temp.join(STAGE_LEASE))?;
    marker.write_all(STAGE_LEASE_MAGIC)?;
    marker.sync_all()?;
    File::open(temp)?.sync_all()?;
    Ok(directory)
}

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
    // Never stage a single App into its immutable source archive directory,
    // nor into any ancestor or descendant of it. Lexical Path equality is
    // insufficient for dot-dot aliases and symlinked ancestors. Check
    // before orphan cleanup, private extraction or atomic publication.
    // This is a static ancestry check, NOT a concurrent path-swap proof.
    let source_parent = source.parent().context("App source directory missing")?;
    let source_root = fs::canonicalize(source_parent).context("canonical App source directory")?;
    let output_root = fs::canonicalize(target_root).context("canonical App stage destination")?;
    if source_root.starts_with(&output_root) || output_root.starts_with(&source_root) {
        bail!("refuse aliased or overlapping App source and stage roots")
    }
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
    reap_interrupted_stages(target_root)?;
    let temp = tempfile::Builder::new()
        .prefix(".inactive-app-")
        .tempdir_in(target_root)?;
    // Tempfile may inherit a broader umask than 0700; protect it before
    // placing the first App byte, otherwise discard the temp destination.
    fs::set_permissions(temp.path(), Permissions::from_mode(0o700))?;
    private_owned_directory(temp.path()).context("temp dir")?;
    let _stage_directory_lock = lock_private_stage(temp.path())?;
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
            // Deterministic SIGKILL fixture only in non-release laboratory builds.
            #[cfg(debug_assertions)]
            if files == 1
                && let Ok(value) = std::env::var("HARS_P8DD_LAB_PAUSE_AFTER_FIRST_FILE_MS")
            {
                let millis: u64 = value.parse().context("invalid lab pause")?;
                if millis > 30_000 {
                    bail!("lab pause exceeds 30 seconds");
                }
                std::thread::sleep(std::time::Duration::from_millis(millis));
            }
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
    // Never publish the internal lease as a user App data file. An extremely
    // narrow crash here can leave an unmarked temp; it is intentionally
    // preserved rather than risk deleting unrelated data.
    fs::remove_file(temp.path().join(STAGE_LEASE))?;
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
