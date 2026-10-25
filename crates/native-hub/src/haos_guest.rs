//! Laboratory-only: Rust starts a genuine pinned HAOS QEMU guest.
//!
//! HAOS/Supervisor are the *compatibility backend*, not rewritten in Rust.
//! No host serial/USB devices, shared folders, user-supplied extra QEMU
//! arguments, persistent image writes or family network controllers.
use sha2::{Digest, Sha256};
use std::{
    fs::OpenOptions,
    io::Read,
    os::unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

const IMAGE_MAX: u64 = 20 * 1024 * 1024 * 1024;
const BOOT_MAX: Duration = Duration::from_secs(540);
const PROMPT: &[u8] = b"homeassistant login:";

#[derive(Debug, PartialEq, Eq)]
pub enum GuestError {
    UnsafeImage,
    ImageDigestMismatch,
    InvalidArguments,
    MissingKvm,
    QemuStart,
    EarlyExit,
    BootTimeout,
}
impl std::fmt::Display for GuestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for GuestError {}

pub struct GuestSpec {
    pub emulator: PathBuf,
    pub firmware: PathBuf,
    pub source_image: PathBuf,
    pub image_sha256: String,
}
pub fn sha256_file(path: &Path) -> Result<String, GuestError> {
    let mut file = OpenOptions::new()
        .read(true)
        .custom_flags(nix::libc::O_NOFOLLOW | nix::libc::O_CLOEXEC | nix::libc::O_NONBLOCK)
        .open(path)
        .map_err(|_| GuestError::UnsafeImage)?;
    let meta = file.metadata().map_err(|_| GuestError::UnsafeImage)?;
    if !meta.is_file()
        || meta.nlink() != 1
        || meta.len() < 1024
        || meta.len() > IMAGE_MAX
        || meta.uid() != nix::unistd::Uid::current().as_raw()
        || meta.permissions().mode() & 0o077 != 0
    {
        return Err(GuestError::UnsafeImage);
    }
    let mut digest = Sha256::new();
    let mut block = [0u8; 65536];
    loop {
        let n = file.read(&mut block).map_err(|_| GuestError::UnsafeImage)?;
        if n == 0 {
            break;
        }
        digest.update(&block[..n]);
    }
    let after = file.metadata().map_err(|_| GuestError::UnsafeImage)?;
    if meta.len() != after.len()
        || meta.mtime() != after.mtime()
        || meta.mtime_nsec() != after.mtime_nsec()
    {
        return Err(GuestError::UnsafeImage);
    }
    Ok(format!("{:x}", digest.finalize()))
}
fn check_path(path: &Path, expect_exec: bool) -> Result<(), GuestError> {
    if !path.is_absolute() {
        return Err(GuestError::InvalidArguments);
    }
    let meta = std::fs::symlink_metadata(path).map_err(|_| GuestError::InvalidArguments)?;
    if !meta.is_file()
        || meta.file_type().is_symlink()
        || meta.nlink() != 1
        || (expect_exec && meta.permissions().mode() & 0o111 == 0)
    {
        return Err(GuestError::InvalidArguments);
    }
    Ok(())
}
impl GuestSpec {
    pub fn validate(&self) -> Result<(), GuestError> {
        if self.emulator != Path::new("/usr/bin/qemu-system-x86_64")
            || self.firmware != Path::new("/usr/share/ovmf/OVMF.fd")
            || !self.source_image.is_absolute()
            || self
                .source_image
                .to_string_lossy()
                .chars()
                .any(|c| c == ',' || c.is_control())
            || self
                .source_image
                .components()
                .any(|c| c == std::path::Component::ParentDir)
            || self.source_image == self.firmware
            || self.source_image == self.emulator
            || self.image_sha256.len() != 64
            || !self
                .image_sha256
                .bytes()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        {
            return Err(GuestError::InvalidArguments);
        }
        check_path(&self.emulator, true)?;
        check_path(&self.firmware, false)?;
        let dev = std::fs::metadata("/dev/kvm").map_err(|_| GuestError::MissingKvm)?;
        if !dev.file_type().is_char_device() {
            return Err(GuestError::MissingKvm);
        }
        let found = sha256_file(&self.source_image)?;
        if found != self.image_sha256 {
            return Err(GuestError::ImageDigestMismatch);
        }
        Ok(())
    }
    /// Fixed upstream-like KVM arguments, no host devices, no public ports,
    /// ephemeral -snapshot, no tap/bridge/hardware passthrough.
    /// Networking uses QEMU user-mode NIC for disposable official Core boot.
    pub fn qemu_args(&self) -> Vec<String> {
        vec![
            "-machine".into(),
            "pc".into(),
            "-cpu".into(),
            "qemu64".into(),
            "-accel".into(),
            "kvm".into(),
            "-m".into(),
            "3072".into(),
            "-smp".into(),
            "2".into(),
            "-bios".into(),
            self.firmware.to_string_lossy().into(),
            "-drive".into(),
            format!(
                "file={},format=qcow2,if=virtio,snapshot=on",
                self.source_image.display()
            ),
            "-snapshot".into(),
            "-nic".into(),
            "user,model=virtio-net-pci".into(),
            "-display".into(),
            "none".into(),
            "-serial".into(),
            "stdio".into(),
            "-monitor".into(),
            "none".into(),
            "-no-reboot".into(),
        ]
    }
}
fn watch_console(mut stream: impl Read + Send + 'static, tx: mpsc::Sender<()>) {
    thread::spawn(move || {
        let mut recent = Vec::<u8>::new();
        let mut buffer = [0u8; 2048];
        while let Ok(count) = stream.read(&mut buffer) {
            if count == 0 {
                return;
            }
            recent.extend_from_slice(&buffer[..count]);
            if recent.windows(PROMPT.len()).any(|x| x == PROMPT) {
                let _ = tx.send(());
                return;
            }
            if recent.len() > 4096 {
                let drop = recent.len() - 256;
                recent.drain(..drop);
            }
        }
    });
}
fn stop_guest(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}
pub fn boot_guest_in_isolated_lab(spec: &GuestSpec) -> Result<(), GuestError> {
    if std::env::var("HARS_DISPOSABLE_KVM_LAB").as_deref() != Ok("1") {
        return Err(GuestError::InvalidArguments);
    }
    spec.validate()?;
    let mut child = Command::new(&spec.emulator)
        .args(spec.qemu_args())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| GuestError::QemuStart)?;
    let (tx, rx) = mpsc::channel();
    if let Some(console) = child.stdout.take() {
        watch_console(console, tx)
    } else {
        stop_guest(&mut child);
        return Err(GuestError::QemuStart);
    }
    let start = Instant::now();
    let outcome = loop {
        if rx.try_recv().is_ok() {
            break Ok(());
        }
        if child
            .try_wait()
            .map_err(|_| GuestError::EarlyExit)?
            .is_some()
        {
            break Err(GuestError::EarlyExit);
        }
        if start.elapsed() > BOOT_MAX {
            break Err(GuestError::BootTimeout);
        }
        thread::sleep(Duration::from_millis(200));
    };
    stop_guest(&mut child);
    // Source image unchanged: -snapshot is never allowed to mutate qcow2.
    if let Ok(check) = sha256_file(&spec.source_image) {
        if check != spec.image_sha256 {
            return Err(GuestError::UnsafeImage);
        }
    } else {
        return Err(GuestError::UnsafeImage);
    }
    outcome
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn immutable_guest_arguments_prevent_host_and_radio_takeover() {
        let spec = GuestSpec {
            emulator: "/usr/bin/qemu-system-x86_64".into(),
            firmware: "/usr/share/ovmf/OVMF.fd".into(),
            source_image: "/tmp/synthetic_haos.qcow2".into(),
            image_sha256: "0".repeat(64),
        };
        let args = spec.qemu_args().join(" ");
        assert!(args.contains("-snapshot"));
        assert!(args.contains("-monitor none"));
        assert!(args.contains("-display none"));
        assert!(args.contains("snapshot=on"));
        assert!(!args.contains("-usbdevice"));
        assert!(!args.contains("hostfwd"));
        assert!(!args.contains("-netdev tap"));
        assert!(!args.contains("virtfs"));
    }
    #[test]
    fn image_requires_single_owner_private_file_no_symlink() {
        let dir = tempfile::tempdir().unwrap();
        let safe = dir.path().join("safe.qcow2");
        std::fs::write(&safe, vec![12u8; 4096]).unwrap();
        std::fs::set_permissions(&safe, std::fs::Permissions::from_mode(0o600)).unwrap();
        let expected = sha256_file(&safe).unwrap();
        assert_eq!(expected.len(), 64);
        let symlink = dir.path().join("shortcut");
        std::os::unix::fs::symlink(&safe, &symlink).unwrap();
        assert!(sha256_file(&symlink).is_err());
        std::fs::set_permissions(&safe, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(sha256_file(&safe).is_err());
    }
    #[test]
    fn refuse_fake_emulator_paths_and_malformed_image_digest() {
        let mut spec = GuestSpec {
            emulator: "/bin/sh".into(),
            firmware: "/usr/share/ovmf/OVMF.fd".into(),
            source_image: "/tmp/example".into(),
            image_sha256: "0".repeat(64),
        };
        assert!(matches!(spec.validate(), Err(GuestError::InvalidArguments)));
        spec.emulator = "/usr/bin/qemu-system-x86_64".into();
        spec.image_sha256 = "../../bad".into();
        assert!(matches!(spec.validate(), Err(GuestError::InvalidArguments)));
    }
}
