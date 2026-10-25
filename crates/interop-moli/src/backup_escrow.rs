//! P8ab: opaque, content-addressed HAOS full backup escrow. NOT migration or
//! archive decryption. Preserve every byte, including encrypted app payloads.
//! Files are private, fsynced, immutable by convention, and hash-verified on export.
use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};
use tempfile::NamedTempFile;

const MAX_ARCHIVE_BYTES: u64 = 1_073_741_824;

fn private_dir(dir: &Path) -> Result<()> {
    let m = fs::symlink_metadata(dir).context("missing escrow private directory")?;
    if !m.is_dir()
        || m.file_type().is_symlink()
        || m.uid() != nix::unistd::Uid::current().as_raw()
        || m.permissions().mode() & 0o077 != 0
    {
        bail!("escrow requires a real owned private directory with no group or other access")
    }
    Ok(())
}
fn digest_key(key: &str) -> bool {
    key.len() == 64
        && key
            .bytes()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
}
fn open_private_archive(path: &Path) -> Result<File> {
    let f = OpenOptions::new()
        .read(true)
        .custom_flags(nix::libc::O_NOFOLLOW | nix::libc::O_NONBLOCK | nix::libc::O_CLOEXEC)
        .open(path)
        .context("archive must be a regular nofollow file")?;
    let m = f.metadata()?;
    if !m.is_file()
        || m.nlink() != 1
        || m.len() < 1024
        || m.len() > MAX_ARCHIVE_BYTES
        || m.permissions().mode() & 0o077 != 0
    {
        bail!("unsafe, world-readable, multiply linked or oversized backup")
    }
    Ok(f)
}
fn hash_file(mut f: File) -> Result<(String, u64)> {
    let mut digest = Sha256::new();
    let mut buf = [0u8; 65536];
    let mut total = 0u64;
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        total = total
            .checked_add(n as u64)
            .context("archive length overflow")?;
        if total > MAX_ARCHIVE_BYTES {
            bail!("archive too large")
        }
        digest.update(&buf[..n]);
    }
    Ok((hex::encode(digest.finalize()), total))
}
fn same_digest(path: &Path, key: &str, len: u64) -> Result<()> {
    let m = fs::symlink_metadata(path).context("missing existing staged blob")?;
    if !m.is_file() || m.file_type().is_symlink() || m.len() != len {
        bail!("existing archive identity invalid")
    }
    let (hash, size) = hash_file(open_private_archive(path)?)?;
    if hash != key || size != len {
        bail!("staged archive mutated or corrupted")
    }
    Ok(())
}
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Receipt {
    pub sha256: String,
    pub bytes: u64,
    pub byte_exact: bool,
    pub opaque_no_decryption: bool,
    pub encrypted_supported_as_opaque: bool,
    pub restored_core_runtime: bool,
    pub migrated_automations: bool,
    pub migrated_apps: bool,
    pub production_ready: bool,
}
impl Receipt {
    fn new(sha256: String, bytes: u64) -> Self {
        Self {
            sha256,
            bytes,
            byte_exact: true,
            opaque_no_decryption: true,
            encrypted_supported_as_opaque: true,
            restored_core_runtime: false,
            migrated_automations: false,
            migrated_apps: false,
            production_ready: false,
        }
    }
}
pub struct BackupEscrow {
    private: PathBuf,
}
impl BackupEscrow {
    /// Operator creates private dir 0700 first. No automatic directory creation.
    pub fn open(dir: &Path) -> Result<Self> {
        private_dir(dir)?;
        Ok(Self {
            private: dir.to_path_buf(),
        })
    }
    /// Copies from an unlinked-by-none owner-private regular source into an
    /// exclusively created temp file; commits no mutable data before fsync.
    pub fn ingest(&self, source: &Path) -> Result<Receipt> {
        private_dir(&self.private)?;
        let mut input = open_private_archive(source)?;
        let before = input.metadata()?;
        let mut temp = NamedTempFile::new_in(&self.private)?;
        let mut digest = Sha256::new();
        let mut bytes = 0u64;
        let mut buf = [0u8; 65536];
        loop {
            let n = input.read(&mut buf)?;
            if n == 0 {
                break;
            }
            bytes = bytes.checked_add(n as u64).context("archive too large")?;
            if bytes > MAX_ARCHIVE_BYTES {
                bail!("archive exceeds cap")
            }
            digest.update(&buf[..n]);
            temp.write_all(&buf[..n])?;
        }
        let after = input.metadata()?;
        if before.len() != bytes
            || before.len() != after.len()
            || before.mtime() != after.mtime()
            || before.mtime_nsec() != after.mtime_nsec()
        {
            bail!("source archive changed during copy")
        }
        if bytes < 1024 {
            bail!("invalid small source archive")
        }
        temp.as_file_mut().sync_all()?;
        let sha256 = hex::encode(digest.finalize());
        let target = self.private.join(format!("{sha256}.tar"));
        match temp.persist_noclobber(&target) {
            Ok(_file) => {}
            Err(error) => {
                if error.error.kind() != std::io::ErrorKind::AlreadyExists {
                    return Err(error.error.into());
                }
                // Content-addressed entry already exists: verify rather than
                // trusting the filename or replacing a known-good version.
                same_digest(&target, &sha256, bytes)?;
            }
        }
        File::open(&self.private)?.sync_all()?;
        same_digest(&target, &sha256, bytes)?;
        Ok(Receipt::new(sha256, bytes))
    }
    /// Re-read and authenticate the entire stored file; export only into a
    /// separately precreated 0700 directory with no preexisting archive.
    pub fn export(&self, key: &str, destination_dir: &Path) -> Result<Receipt> {
        if !digest_key(key) {
            bail!("invalid content identity")
        }
        private_dir(&self.private)?;
        private_dir(destination_dir)?;
        if self.private == destination_dir {
            bail!("source and export dir must differ")
        }
        let stored = self.private.join(format!("{key}.tar"));
        let mut input = open_private_archive(&stored)?;
        let size = input.metadata()?.len();
        let mut temp = NamedTempFile::new_in(destination_dir)?;
        let mut digest = Sha256::new();
        let mut count = 0u64;
        let mut buf = [0u8; 65536];
        loop {
            let n = input.read(&mut buf)?;
            if n == 0 {
                break;
            }
            count = count
                .checked_add(n as u64)
                .context("archive length overflow")?;
            if count > MAX_ARCHIVE_BYTES {
                bail!("archive exceeds cap")
            }
            digest.update(&buf[..n]);
            temp.write_all(&buf[..n])?;
        }
        if count != size || hex::encode(digest.finalize()) != key {
            bail!("escrow source mutated; export withheld")
        }
        temp.as_file_mut().sync_all()?;
        let out = destination_dir.join("archive.tar");
        temp.persist_noclobber(&out).map_err(|e| e.error)?;
        File::open(destination_dir)?.sync_all()?;
        same_digest(&out, key, count)?;
        Ok(Receipt::new(key.to_owned(), count))
    }
}
