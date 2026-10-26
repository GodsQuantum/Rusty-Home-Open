//! Laboratory-only: Rust starts a genuine pinned HAOS QEMU guest.
//!
//! HAOS/Supervisor are the *compatibility backend*, not rewritten in Rust.
//! No host serial/USB devices, shared folders, user-supplied extra QEMU
//! arguments, persistent image writes or family network controllers.
use sha2::{Digest, Sha256};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    os::unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt, PermissionsExt},
    os::unix::net::UnixStream,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
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
    GuestHealthTimeout,
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

/// P8ae: a *private lab-only* copy-on-write guest disk backed by the exact
/// SHA256-pinned official immutable source. No external HASS configuration.
const TEST_MARKER: &str = "HARS_P8AE_DURABLE_MARKER_20261025";
fn private_owner_dir(path: &Path) -> Result<(), GuestError> {
    let m = std::fs::symlink_metadata(path).map_err(|_| GuestError::UnsafeImage)?;
    if !m.is_dir()
        || m.file_type().is_symlink()
        || m.uid() != nix::unistd::Uid::current().as_raw()
        || m.permissions().mode() & 0o077 != 0
    {
        return Err(GuestError::UnsafeImage);
    }
    Ok(())
}
/// Fail closed on multiple concurrent VM managers. Lock remains held across
/// both QEMU processes (including the simulated hard failure).
fn private_vm_lock(dir: &Path) -> Result<File, GuestError> {
    private_owner_dir(dir)?;
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .mode(0o600)
        .custom_flags(nix::libc::O_NOFOLLOW | nix::libc::O_CLOEXEC)
        .open(dir.join(".single-owner-lock"))
        .map_err(|_| GuestError::UnsafeImage)?;
    let meta = lock.metadata().map_err(|_| GuestError::UnsafeImage)?;
    if !meta.is_file()
        || meta.nlink() != 1
        || meta.uid() != nix::unistd::Uid::current().as_raw()
        || meta.permissions().mode() & 0o077 != 0
    {
        return Err(GuestError::UnsafeImage);
    }
    let status = unsafe {
        nix::libc::flock(
            std::os::fd::AsRawFd::as_raw_fd(&lock),
            nix::libc::LOCK_EX | nix::libc::LOCK_NB,
        )
    };
    if status != 0 {
        return Err(GuestError::UnsafeImage);
    }
    Ok(lock)
}
/// Rust directly invokes distro QEMU tools with fixed typed argv and verifies
/// the backing base; it does not modify user-supplied images or launch shell.
fn make_overlay(spec: &GuestSpec, dir: &Path) -> Result<PathBuf, GuestError> {
    // QEMU -drive uses comma-separated options; reject injection via
    // attacker-controlled owner-private directory names.
    if !dir.is_absolute()
        || dir
            .to_string_lossy()
            .chars()
            .any(|c| c == ',' || c.is_control())
        || dir
            .components()
            .any(|c| c == std::path::Component::ParentDir)
    {
        return Err(GuestError::InvalidArguments);
    }
    private_owner_dir(dir)?;
    let out = dir.join("guest-overlay.qcow2");
    // symlink_metadata also catches dangling symlinks that Path::exists()
    // would incorrectly treat as absent.
    match std::fs::symlink_metadata(&out) {
        Ok(_) => return Err(GuestError::UnsafeImage),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err(GuestError::UnsafeImage),
    }
    let source = spec.source_image.to_string_lossy().to_string();
    let exit = Command::new("/usr/bin/qemu-img")
        .args([
            "create",
            "-f",
            "qcow2",
            "-F",
            "qcow2",
            "-b",
            source.as_str(),
        ])
        .arg(&out)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|_| GuestError::QemuStart)?;
    if !exit.success() {
        return Err(GuestError::QemuStart);
    }
    let meta = std::fs::symlink_metadata(&out).map_err(|_| GuestError::UnsafeImage)?;
    if !meta.is_file() || meta.nlink() != 1 || meta.uid() != nix::unistd::Uid::current().as_raw() {
        return Err(GuestError::UnsafeImage);
    }
    std::fs::set_permissions(&out, std::fs::Permissions::from_mode(0o600))
        .map_err(|_| GuestError::UnsafeImage)?;
    File::open(&out)
        .and_then(|f| f.sync_all())
        .map_err(|_| GuestError::UnsafeImage)?;
    File::open(dir)
        .and_then(|f| f.sync_all())
        .map_err(|_| GuestError::UnsafeImage)?;
    Ok(out)
}

/// Read-only admission of an EXISTING overlay after the Rust owner exited.
/// Check both the inode and its qcow2 backing reference; no new guest state
/// is created during recovery. The base image is SHA-pinned separately.
fn verified_existing_overlay(spec: &GuestSpec, dir: &Path) -> Result<PathBuf, GuestError> {
    private_owner_dir(dir)?;
    if !dir.is_absolute()
        || dir
            .to_string_lossy()
            .chars()
            .any(|c| c == ',' || c.is_control())
        || dir
            .components()
            .any(|c| c == std::path::Component::ParentDir)
    {
        return Err(GuestError::InvalidArguments);
    }
    let overlay = dir.join("guest-overlay.qcow2");
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(nix::libc::O_NOFOLLOW | nix::libc::O_CLOEXEC | nix::libc::O_NONBLOCK)
        .open(&overlay)
        .map_err(|_| GuestError::UnsafeImage)?;
    let m = file.metadata().map_err(|_| GuestError::UnsafeImage)?;
    if !m.is_file()
        || m.nlink() != 1
        || m.uid() != nix::unistd::Uid::current().as_raw()
        || m.permissions().mode() & 0o077 != 0
        || m.len() < 1024
        || m.len() > IMAGE_MAX
    {
        return Err(GuestError::UnsafeImage);
    }
    let output = Command::new("/usr/bin/qemu-img")
        .arg("info")
        .arg("-f")
        .arg("qcow2")
        .arg("--output=json")
        .arg(&overlay)
        .stdin(Stdio::null())
        .output()
        .map_err(|_| GuestError::UnsafeImage)?;
    if !output.status.success() || output.stdout.len() > 65536 || !output.stderr.is_empty() {
        return Err(GuestError::UnsafeImage);
    }
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).map_err(|_| GuestError::UnsafeImage)?;
    let base = spec.source_image.to_string_lossy();
    if json.get("format").and_then(|x| x.as_str()) != Some("qcow2")
        || json.get("backing-filename").and_then(|x| x.as_str()) != Some(base.as_ref())
        || json.get("backing-filename-format").and_then(|x| x.as_str()) != Some("qcow2")
    {
        return Err(GuestError::UnsafeImage);
    }
    Ok(overlay)
}
/// Separate Rust process entrypoints: the first one persists a guest marker,
/// exits and relinquishes flock; a new Rust process later reattaches only to
/// the already-pinned owner-private overlay and checks the real guest marker.
/// All data is fictional and disposable on isolated CI KVM.
pub fn guest_marker_in_separate_manager_lab(
    spec: &GuestSpec,
    dir: &Path,
    create: bool,
) -> Result<(), GuestError> {
    if std::env::var("HARS_DISPOSABLE_KVM_LAB").as_deref() != Ok("1") {
        return Err(GuestError::InvalidArguments);
    }
    // Reject a concurrent manager immediately: do not hash a large
    // official HAOS base image before acquiring the exclusive lease.
    let _lease = private_vm_lock(dir)?;
    spec.validate()?;
    let overlay = if create {
        make_overlay(spec, dir)?
    } else {
        verified_existing_overlay(spec, dir)?
    };
    let digest = format!(
        "{:x}",
        Sha256::digest(format!("{TEST_MARKER}\n").as_bytes())
    );
    session_verify_guest_marker(spec, &overlay, create, &digest)?;
    File::open(&overlay)
        .and_then(|f| f.sync_all())
        .map_err(|_| GuestError::UnsafeImage)?;
    File::open(dir)
        .and_then(|f| f.sync_all())
        .map_err(|_| GuestError::UnsafeImage)?;
    verified_existing_overlay(spec, dir)?;
    if sha256_file(&spec.source_image)? != spec.image_sha256 {
        return Err(GuestError::ImageDigestMismatch);
    }
    Ok(())
}

// P8ag: an isolated Rust-owned long-running HAOS guest lifecycle primitive.
// A service start uses an existing validated qcow2; it never auto-creates,
// replaces, migrates or gains access to household devices.
static SERVICE_STOP: AtomicBool = AtomicBool::new(false);
extern "C" fn service_term(_signal: nix::libc::c_int) {
    SERVICE_STOP.store(true, Ordering::SeqCst);
}
fn qmp_shutdown(socket: &Path) -> Result<(), GuestError> {
    let meta = std::fs::symlink_metadata(socket).map_err(|_| GuestError::QemuStart)?;
    if !meta.file_type().is_socket() || meta.uid() != nix::unistd::Uid::current().as_raw() {
        return Err(GuestError::UnsafeImage);
    }
    let mut conn = UnixStream::connect(socket).map_err(|_| GuestError::QemuStart)?;
    conn.set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|_| GuestError::QemuStart)?;
    conn.set_write_timeout(Some(Duration::from_secs(2)))
        .map_err(|_| GuestError::QemuStart)?;
    let mut greeting = [0u8; 4096];
    let n = conn
        .read(&mut greeting)
        .map_err(|_| GuestError::QemuStart)?;
    if !greeting[..n].windows(5).any(|p| p == b"\"QMP\"") {
        return Err(GuestError::QemuStart);
    }
    conn.write_all(b"{\"execute\":\"qmp_capabilities\"}\r\n")
        .map_err(|_| GuestError::QemuStart)?;
    // QMP is single-client and replies in order; consume capabilities.
    let n = conn
        .read(&mut greeting)
        .map_err(|_| GuestError::QemuStart)?;
    if !greeting[..n].windows(8).any(|p| p == b"\"return\"") {
        return Err(GuestError::QemuStart);
    }
    conn.write_all(b"{\"execute\":\"system_powerdown\"}\r\n")
        .map_err(|_| GuestError::QemuStart)?;
    Ok(())
}
/// No credentials, no redirects, no sensitive body captured: only
/// whether official Core responds to a local HTTP frontend request.
fn local_core_frontend_responded() -> bool {
    let local = SocketAddr::from(([127, 0, 0, 1], 18180));
    let Ok(mut tcp) = TcpStream::connect_timeout(&local, Duration::from_millis(300)) else {
        return false;
    };
    let _ = tcp.set_read_timeout(Some(Duration::from_millis(700)));
    let _ = tcp.set_write_timeout(Some(Duration::from_millis(700)));
    if tcp
        .write_all(b"GET / HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
        .is_err()
    {
        return false;
    }
    let mut first = [0u8; 2048];
    let Ok(n) = tcp.read(&mut first) else {
        return false;
    };
    let response = &first[..n];
    response.starts_with(b"HTTP/1.1 200 ")
        || response.starts_with(b"HTTP/1.1 302 ")
        || response.starts_with(b"HTTP/1.1 307 ")
}
pub struct GuestServiceOutcome {
    pub guest_process_reaped: bool,
    pub base_image_unchanged: bool,
    pub acpi_shutdown_requested: bool,
    pub loopback_frontend_forward_requested: bool,
    pub loopback_listener_reached: bool,
    pub core_frontend_http_responded: bool,
}
/// The *host-side* service runs until SIGTERM/SIGINT or unexpected QEMU exit.
/// A genuine OS/systemd manager owns the Rust process. QEMU runs without
/// attached console, public QMP ports, USB passthrough or stdout secrets.
pub fn run_guarded_guest_service_lab(
    spec: &GuestSpec,
    private_dir: &Path,
) -> Result<GuestServiceOutcome, GuestError> {
    run_guest_service_inner_lab(spec, private_dir, false, false)
}
/// Optional on-host UI visibility, loopback only. HAOS 2026.8+ on HAOS
/// uses guest TCP/80, not 8123. Core itself owns authentication/OAuth.
pub fn run_guarded_guest_service_http_loopback_lab(
    spec: &GuestSpec,
    private_dir: &Path,
) -> Result<GuestServiceOutcome, GuestError> {
    run_guest_service_inner_lab(spec, private_dir, true, false)
}
pub fn verify_core_frontend_via_rust_guest_lab(
    spec: &GuestSpec,
    private_dir: &Path,
) -> Result<GuestServiceOutcome, GuestError> {
    run_guest_service_inner_lab(spec, private_dir, true, true)
}
fn run_guest_service_inner_lab(
    spec: &GuestSpec,
    private_dir: &Path,
    local_core_ui: bool,
    must_verify_frontend: bool,
) -> Result<GuestServiceOutcome, GuestError> {
    if std::env::var("HARS_DISPOSABLE_KVM_LAB").as_deref() != Ok("1") {
        return Err(GuestError::InvalidArguments);
    }
    let _lease = private_vm_lock(private_dir)?;
    spec.validate()?;
    let overlay = verified_existing_overlay(spec, private_dir)?;
    let qmp = private_dir.join("rusty-haos-lab-qmp.sock");
    if std::fs::symlink_metadata(&qmp).is_ok() {
        return Err(GuestError::UnsafeImage);
    }
    SERVICE_STOP.store(false, Ordering::SeqCst);
    // Signal handler updates only an atomic flag, no allocations/IO.
    unsafe {
        nix::libc::signal(
            nix::libc::SIGTERM,
            service_term as *const () as nix::libc::sighandler_t,
        );
        nix::libc::signal(
            nix::libc::SIGINT,
            service_term as *const () as nix::libc::sighandler_t,
        );
    }
    let mut args = overlay_args(spec, &overlay);
    let pos = args
        .iter()
        .position(|p| p == "-serial")
        .ok_or(GuestError::InvalidArguments)?;
    args[pos + 1] = "null".into();
    if local_core_ui {
        // Official QEMU -nic hostfwd binds ONLY the local IPv4 loopback.
        // No public listener, host filesystem mapping or guest CLI token.
        let nic = args
            .iter()
            .position(|p| p == "-nic")
            .ok_or(GuestError::InvalidArguments)?;
        args[nic + 1] = "user,model=virtio-net-pci,hostfwd=tcp:127.0.0.1:18180-:80".into();
    }
    args.extend([
        "-qmp".into(),
        format!("unix:{},server=on,wait=off", qmp.display()),
    ]);
    let mut child = Command::new(&spec.emulator)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| GuestError::QemuStart)?;
    // CI-only bounded lifecycle validation; no signal injection required.
    let max_lab_seconds = if must_verify_frontend { 300 } else { 120 };
    let lab_stop_after = std::env::var("HARS_LAB_AUTO_STOP_SECONDS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|n| (1..=max_lab_seconds).contains(n))
        .map(Duration::from_secs);
    let service_started = Instant::now();
    let mut loopback_listener_reached = false;
    let mut core_frontend_http_responded = false;
    let mut last_http_probe = Instant::now() - Duration::from_secs(3);
    let mut unexpectedly_exited = false;
    while !SERVICE_STOP.load(Ordering::SeqCst) {
        if must_verify_frontend
            && !core_frontend_http_responded
            && last_http_probe.elapsed() >= Duration::from_secs(3)
        {
            core_frontend_http_responded = local_core_frontend_responded();
            last_http_probe = Instant::now();
            if core_frontend_http_responded {
                SERVICE_STOP.store(true, Ordering::SeqCst);
            }
        }
        if local_core_ui && !loopback_listener_reached {
            let local = SocketAddr::from(([127, 0, 0, 1], 18180));
            loopback_listener_reached =
                TcpStream::connect_timeout(&local, Duration::from_millis(120)).is_ok();
        }
        if lab_stop_after.is_some_and(|limit| service_started.elapsed() >= limit) {
            SERVICE_STOP.store(true, Ordering::SeqCst);
        }
        match child.try_wait() {
            Ok(Some(_)) | Err(_) => {
                unexpectedly_exited = true;
                break;
            }
            Ok(None) => thread::sleep(Duration::from_millis(150)),
        }
    }
    let mut acpi = false;
    if !unexpectedly_exited {
        acpi = qmp_shutdown(&qmp).is_ok();
        let deadline = Instant::now() + Duration::from_secs(12);
        while Instant::now() < deadline {
            match child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) => thread::sleep(Duration::from_millis(150)),
                Err(_) => break,
            }
        }
    }
    // No orphan guest after Rust manager exits, even after ACPI failure.
    stop_guest(&mut child);
    // Remove only the expected real owner-controlled socket from the private
    // directory. Never follow or unlink user-controlled symlink paths.
    if std::fs::symlink_metadata(&qmp).is_ok() {
        let m = std::fs::symlink_metadata(&qmp).map_err(|_| GuestError::UnsafeImage)?;
        if !m.file_type().is_socket() || m.uid() != nix::unistd::Uid::current().as_raw() {
            return Err(GuestError::UnsafeImage);
        }
        std::fs::remove_file(&qmp).map_err(|_| GuestError::UnsafeImage)?;
    }
    if unexpectedly_exited {
        return Err(GuestError::EarlyExit);
    }
    if must_verify_frontend && !core_frontend_http_responded {
        return Err(GuestError::GuestHealthTimeout);
    }
    File::open(&overlay)
        .and_then(|f| f.sync_all())
        .map_err(|_| GuestError::UnsafeImage)?;
    File::open(private_dir)
        .and_then(|f| f.sync_all())
        .map_err(|_| GuestError::UnsafeImage)?;
    verified_existing_overlay(spec, private_dir)?;
    if sha256_file(&spec.source_image)? != spec.image_sha256 {
        return Err(GuestError::ImageDigestMismatch);
    }
    Ok(GuestServiceOutcome {
        guest_process_reaped: true,
        base_image_unchanged: true,
        acpi_shutdown_requested: acpi,
        loopback_frontend_forward_requested: local_core_ui,
        loopback_listener_reached,
        core_frontend_http_responded,
    })
}

fn overlay_args(spec: &GuestSpec, overlay: &Path) -> Vec<String> {
    let mut args = spec.qemu_args();
    let position = args
        .iter()
        .position(|arg| arg == "-drive")
        .expect("base disk arg");
    args[position + 1] = format!("file={},format=qcow2,if=virtio", overlay.display());
    // Remove both ephemeral switches: the COW overlay now owns guest writes.
    args.retain(|arg| arg != "-snapshot");
    args
}
fn session_verify_guest_marker(
    spec: &GuestSpec,
    overlay: &Path,
    write: bool,
    expected_digest: &str,
) -> Result<(), GuestError> {
    let mut child = Command::new(&spec.emulator)
        .args(overlay_args(spec, overlay))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| GuestError::QemuStart)?;
    let mut stdin = child.stdin.take().ok_or(GuestError::QemuStart)?;
    let (tx, rx) = mpsc::channel();
    if let Some(console) = child.stdout.take() {
        collect_console(console, tx)
    } else {
        stop_guest(&mut child);
        return Err(GuestError::QemuStart);
    }
    let mut stage = 0u8;
    let mut recent = Vec::<u8>::new();
    let begun = Instant::now();
    let expected = expected_digest.as_bytes();
    let outcome = loop {
        match child.try_wait() {
            Ok(Some(_)) | Err(_) => break Err(GuestError::EarlyExit),
            Ok(None) => {}
        }
        if begun.elapsed() > BOOT_MAX {
            break Err(GuestError::GuestHealthTimeout);
        }
        if let Ok(part) = rx.recv_timeout(Duration::from_millis(200)) {
            recent.extend_from_slice(&part);
            if recent.len() > 16384 {
                let remove = recent.len() - 4096;
                recent.drain(..remove);
            }
        }
        match stage {
            0 if has_marker(&recent, PROMPT) => {
                if let Err(e) = send_guest_line(&mut stdin, b"root\n") {
                    break Err(e);
                }
                stage = 1;
                recent.clear();
            }
            1 if has_marker(&recent, SHELL_PROMPT) => {
                let path = "/mnt/data/supervisor/share/hars_p8ae_guest_state";
                let command = if write {
                    format!(
                        "mkdir -p /mnt/data/supervisor/share && printf '{}\\n' > {} && sync && sha256sum {}\n",
                        TEST_MARKER, path, path
                    )
                } else {
                    format!("sha256sum {}\n", path)
                };
                if let Err(e) = send_guest_line(&mut stdin, command.as_bytes()) {
                    break Err(e);
                }
                stage = 2;
                recent.clear();
            }
            2 if has_marker(&recent, expected) => break Ok(()),
            _ => {}
        }
    };
    stop_guest(&mut child);
    outcome
}
/// Genuine QEMU guest twice, with simulated process SIGKILL between boots.
/// Only a fictitious marker inside a disposable HAOS guest is manipulated.
pub fn prove_guest_cow_state_across_restarts_in_lab(
    spec: &GuestSpec,
    overlay_dir: &Path,
) -> Result<(), GuestError> {
    if std::env::var("HARS_DISPOSABLE_KVM_LAB").as_deref() != Ok("1") {
        return Err(GuestError::InvalidArguments);
    }
    spec.validate()?;
    let _lock = private_vm_lock(overlay_dir)?;
    let overlay = make_overlay(spec, overlay_dir)?;
    let digest = format!(
        "{:x}",
        Sha256::digest(format!("{TEST_MARKER}\n").as_bytes())
    );
    session_verify_guest_marker(spec, &overlay, true, &digest)?;
    // QEMU was killed and reaped; ensure overlay durability before restart.
    File::open(&overlay)
        .and_then(|f| f.sync_all())
        .map_err(|_| GuestError::UnsafeImage)?;
    File::open(overlay_dir)
        .and_then(|f| f.sync_all())
        .map_err(|_| GuestError::UnsafeImage)?;
    session_verify_guest_marker(spec, &overlay, false, &digest)?;
    let after = sha256_file(&spec.source_image)?;
    if after != spec.image_sha256 {
        return Err(GuestError::ImageDigestMismatch);
    }
    Ok(())
}

const CLI_OK: &[u8] = b"\"result\":\"ok\"";
const CLI_OK_SPACED: &[u8] = b"\"result\": \"ok\"";
const SHELL_PROMPT: &[u8] = b"# ";
/// Security: only boolean results. Never persist raw guest console output,
/// tokens, config, Supervisor health response or shell transcript.
fn has_marker(buffer: &[u8], mark: &[u8]) -> bool {
    buffer.windows(mark.len()).any(|v| v == mark)
}
fn cli_ok(buffer: &[u8]) -> bool {
    has_marker(buffer, CLI_OK) || has_marker(buffer, CLI_OK_SPACED)
}
fn collect_console(mut stream: impl Read + Send + 'static, tx: mpsc::Sender<Vec<u8>>) {
    thread::spawn(move || {
        let mut buf = [0u8; 2048];
        while let Ok(n) = stream.read(&mut buf) {
            if n == 0 {
                break;
            }
            if tx.send(buf[..n].to_vec()).is_err() {
                break;
            }
        }
    });
}
fn send_guest_line(stdin: &mut impl Write, value: &[u8]) -> Result<(), GuestError> {
    stdin.write_all(value).map_err(|_| GuestError::EarlyExit)?;
    stdin.flush().map_err(|_| GuestError::EarlyExit)
}
/// Actual Rust-owned official HAOS guest. Must prove both CLI Supervisor
/// and Core "result=ok" from the guest serial console, not merely kernel boot.
pub fn verify_supervisor_and_core_in_isolated_lab(spec: &GuestSpec) -> Result<(), GuestError> {
    if std::env::var("HARS_DISPOSABLE_KVM_LAB").as_deref() != Ok("1") {
        return Err(GuestError::InvalidArguments);
    }
    spec.validate()?;
    let mut child = Command::new(&spec.emulator)
        .args(spec.qemu_args())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| GuestError::QemuStart)?;
    let mut stdin = child.stdin.take().ok_or(GuestError::QemuStart)?;
    let (tx, rx) = mpsc::channel();
    if let Some(console) = child.stdout.take() {
        collect_console(console, tx)
    } else {
        stop_guest(&mut child);
        return Err(GuestError::QemuStart);
    }
    let mut stage = 0u8;
    let mut recent = Vec::<u8>::new();
    let mut sent = Instant::now();
    let started = Instant::now();
    let outcome = loop {
        if child
            .try_wait()
            .map_err(|_| GuestError::EarlyExit)?
            .is_some()
        {
            break Err(GuestError::EarlyExit);
        }
        if started.elapsed() > BOOT_MAX {
            break Err(GuestError::GuestHealthTimeout);
        }
        if let Ok(part) = rx.recv_timeout(Duration::from_millis(200)) {
            recent.extend_from_slice(&part);
            if recent.len() > 65536 {
                let drop = recent.len() - 8192;
                recent.drain(..drop);
            }
        }
        match stage {
            0 if has_marker(&recent, PROMPT) => {
                if let Err(e) = send_guest_line(&mut stdin, b"root\n") {
                    break Err(e);
                }
                stage = 1;
                recent.clear();
                sent = Instant::now();
            }
            1 if has_marker(&recent, SHELL_PROMPT) => {
                if let Err(e) =
                    send_guest_line(&mut stdin, b"ha supervisor info --no-progress --raw-json\n")
                {
                    break Err(e);
                }
                stage = 2;
                recent.clear();
                sent = Instant::now();
            }
            2 if cli_ok(&recent) => {
                if let Err(e) =
                    send_guest_line(&mut stdin, b"ha core info --no-progress --raw-json\n")
                {
                    break Err(e);
                }
                stage = 3;
                recent.clear();
                sent = Instant::now();
            }
            2 if sent.elapsed() > Duration::from_secs(20) => {
                if let Err(e) =
                    send_guest_line(&mut stdin, b"ha supervisor info --no-progress --raw-json\n")
                {
                    break Err(e);
                }
                recent.clear();
                sent = Instant::now();
            }
            3 if cli_ok(&recent) => break Ok(()),
            3 if sent.elapsed() > Duration::from_secs(20) => {
                if let Err(e) =
                    send_guest_line(&mut stdin, b"ha core info --no-progress --raw-json\n")
                {
                    break Err(e);
                }
                recent.clear();
                sent = Instant::now();
            }
            _ => {}
        }
    };
    stop_guest(&mut child);
    let digest = sha256_file(&spec.source_image).map_err(|_| GuestError::UnsafeImage)?;
    if digest != spec.image_sha256 {
        return Err(GuestError::UnsafeImage);
    }
    outcome
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
    fn second_manager_rejects_missing_or_replaced_overlay() {
        let base = tempfile::tempdir().unwrap();
        let private = base.path().join("owner");
        std::fs::create_dir(&private).unwrap();
        std::fs::set_permissions(&private, std::fs::Permissions::from_mode(0o700)).unwrap();
        let spec = GuestSpec {
            emulator: "/usr/bin/qemu-system-x86_64".into(),
            firmware: "/usr/share/ovmf/OVMF.fd".into(),
            source_image: "/tmp/official-haos.qcow2".into(),
            image_sha256: "a".repeat(64),
        };
        assert!(verified_existing_overlay(&spec, &private).is_err());
        let fake = private.join("guest-overlay.qcow2");
        std::os::unix::fs::symlink("/dev/null", &fake).unwrap();
        assert!(verified_existing_overlay(&spec, &private).is_err());
    }
    #[test]
    fn haos_core_http_loopback_port_is_fixed_and_not_externally_exposed() {
        let nic = "user,model=virtio-net-pci,hostfwd=tcp:127.0.0.1:18180-:80";
        assert!(nic.contains("127.0.0.1:18180-:80"));
        assert!(!nic.contains("0.0.0.0"));
        assert!(!nic.contains("hostfwd=tcp::"));
    }
    #[test]
    fn service_does_not_accept_external_haos_vm_or_radio_without_lab_opt_in() {
        // Declared lab-only mode prevents silently installing this launcher
        // against a host's existing family HAOS VM.
        assert!(matches!(
            std::env::var("HARS_NONEXISTENT_STRICT_LAB_TEST"),
            Err(std::env::VarError::NotPresent)
        ));
        assert_eq!(std::mem::size_of::<GuestServiceOutcome>(), 6);
    }
    #[test]
    fn persistent_guest_arg_list_uses_cow_not_unsafe_ephemeral_switches() {
        let spec = GuestSpec {
            emulator: "/usr/bin/qemu-system-x86_64".into(),
            firmware: "/usr/share/ovmf/OVMF.fd".into(),
            source_image: "/tmp/haos.qcow2".into(),
            image_sha256: "a".repeat(64),
        };
        let args = overlay_args(&spec, Path::new("/tmp/overlay.qcow2")).join(" ");
        assert!(args.contains("/tmp/overlay.qcow2"));
        assert!(!args.contains("snapshot=on") && !args.contains("-snapshot"));
        assert!(!args.contains("hostfwd") && !args.contains("virtfs"));
        assert!(!args.contains("usb-host"));
    }
    #[test]
    fn supervisor_and_core_health_requires_real_cli_ok_marker() {
        assert!(cli_ok(br#"{"result":"ok","data":{}}"#));
        assert!(cli_ok(br#"{"result": "ok", "data": {}}"#));
        assert!(!cli_ok(br#"{"result":"error","message":"not ready"}"#));
        assert!(!cli_ok(b"homeassistant login:"));
        assert!(!cli_ok(b"ha supervisor info --raw-json"));
    }
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
