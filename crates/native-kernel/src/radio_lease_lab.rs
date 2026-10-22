//! Local cooperating-process lab writer fence. NO radio, USB, HAOS or fabric access.
//! A Unix flock is not a production-proof distributed radio owner.
use anyhow::{Context, Result, bail};
use nix::fcntl::{Flock, FlockArg};
use std::fs::{self, File, OpenOptions};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::Path;

/// Only a private immediate child of this checkout's disposable runtime dir.
/// No arbitrary lock path, root, symlink or world-writable directory allowed.
pub fn acquire(root: &Path) -> Result<Flock<File>> {
    if nix::unistd::Uid::effective().is_root() {
        bail!("radio lease simulation must never run as root");
    }
    let expected_runtime = std::env::current_dir()?.join("runtime").canonicalize()?;
    let entry = fs::symlink_metadata(root).context("radio lab directory unavailable")?;
    if !entry.file_type().is_dir()
        || entry.uid() != nix::unistd::Uid::current().as_raw()
        || entry.permissions().mode() & 0o077 != 0
    {
        bail!("radio lease requires real owner-private fixture directory");
    }
    let dir = root.canonicalize()?;
    if dir.parent() != Some(expected_runtime.as_path()) {
        bail!("radio lease confined to private immediate runtime child");
    }
    let path = dir.join("synthetic-radio-owner.lock");
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .mode(0o600)
        .custom_flags(nix::libc::O_NOFOLLOW | nix::libc::O_CLOEXEC)
        .open(&path)
        .context("cannot open no-follow synthetic lease")?;
    let meta = file.metadata().context("cannot stat opened lease")?;
    if !meta.file_type().is_file()
        || meta.uid() != nix::unistd::Uid::current().as_raw()
        || meta.permissions().mode() & 0o077 != 0
        || meta.nlink() != 1
        || meta.len() != 0
    {
        bail!("unsafe synthetic radio lease descriptor");
    }
    let lease = Flock::lock(file, FlockArg::LockExclusiveNonblock)
        .map_err(|(_file, _)| anyhow::anyhow!("exclusive simulated writer already held"))?;
    // Check the pathname and descriptor still identify the same inode.
    let current = fs::symlink_metadata(&path)?;
    if !current.file_type().is_file()
        || current.dev() != lease.metadata()?.dev()
        || current.ino() != lease.metadata()?.ino()
    {
        bail!("radio lease pathname changed while acquiring");
    }
    Ok(lease)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_one_writer_until_guard_dropped() {
        let runtime = std::env::current_dir().unwrap().join("runtime");
        fs::create_dir_all(&runtime).unwrap();
        let root = tempfile::Builder::new()
            .prefix("p20-exclusive-")
            .tempdir_in(&runtime)
            .unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let guard = acquire(root.path()).unwrap();
        assert!(
            acquire(root.path()).is_err(),
            "second writer must fail immediately"
        );
        drop(guard);
        assert!(acquire(root.path()).is_ok(), "OS lock must release on drop");
    }

    #[test]
    fn directory_permission_and_alias_are_denied() {
        let runtime = std::env::current_dir().unwrap().join("runtime");
        fs::create_dir_all(&runtime).unwrap();
        let root = tempfile::Builder::new()
            .prefix("p20-perms-")
            .tempdir_in(&runtime)
            .unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o770)).unwrap();
        assert!(acquire(root.path()).is_err());
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let alias = root.path().join("alias");
        std::os::unix::fs::symlink(root.path(), &alias).unwrap();
        assert!(acquire(&alias).is_err());
    }
}
