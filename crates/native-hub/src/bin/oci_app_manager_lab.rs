//! P8ar: lab-only *actual* rootless native systemd/Podman OCI App manager.
//! No root, no household HAOS, no user device access, no production API.
use hars_native_hub::oci_quadlet::{AppNetwork, OciApp, QuadletUnits, bind_native_secret};
use std::{
    env,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    os::unix::{
        fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
        io::AsRawFd,
    },
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::Duration,
};

fn denied() -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, "lab manager refused")
}
fn bad() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, "invalid OCI app operation")
}
fn sane_slug(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.as_bytes()[0].is_ascii_lowercase()
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}
fn owner_dir(p: &Path) -> io::Result<()> {
    let uid = unsafe { nix::libc::geteuid() };
    let m = fs::symlink_metadata(p)?;
    if !m.file_type().is_dir() || m.uid() != uid || m.mode() & 0o022 != 0 {
        return Err(denied());
    }
    Ok(())
}
fn installed_file(p: &Path) -> io::Result<()> {
    let m = fs::symlink_metadata(p)?;
    let uid = unsafe { nix::libc::geteuid() };
    if !m.file_type().is_file() || m.uid() != uid || m.mode() & 0o022 != 0 {
        return Err(denied());
    }
    Ok(())
}
fn units_dir() -> io::Result<PathBuf> {
    let home = PathBuf::from(env::var_os("HOME").ok_or_else(denied)?);
    if !home.is_absolute() {
        return Err(denied());
    }
    let mut dir = home;
    owner_dir(&dir)?;
    for part in [".config", "containers", "systemd"] {
        dir = dir.join(part);
        match fs::symlink_metadata(&dir) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                fs::create_dir(&dir)?;
                fs::set_permissions(&dir, fs::Permissions::from_mode(0o700))?;
            }
            Err(e) => return Err(e),
            Ok(_) => (),
        }
        owner_dir(&dir)?;
    }
    Ok(dir)
}
fn cmd(bin: &str, args: &[&str]) -> io::Result<()> {
    let ok = Command::new(bin)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?
        .success();
    if ok {
        Ok(())
    } else {
        Err(io::Error::other("native system service refused operation"))
    }
}
fn systemctl(args: &[&str]) -> io::Result<()> {
    cmd("/usr/bin/systemctl", args)
}
fn podman(args: &[&str]) -> io::Result<()> {
    cmd("/usr/bin/podman", args)
}
fn write_private_new(path: &Path, data: &str) -> io::Result<()> {
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    f.write_all(data.as_bytes())?;
    f.sync_all()
}
fn emit_files(dir: &Path, u: &QuadletUnits) -> io::Result<()> {
    let container = dir.join(&u.container_name);
    let volume = dir.join(&u.volume_name);
    if container.exists() || volume.exists() {
        return Err(denied());
    }
    write_private_new(&container, &u.container)?;
    if let Err(e) = write_private_new(&volume, &u.volume) {
        fs::remove_file(container)?;
        return Err(e);
    }
    File::open(dir)?.sync_all()?;
    Ok(())
}
fn unit_paths(dir: &Path, slug: &str) -> io::Result<(PathBuf, PathBuf, String)> {
    if !sane_slug(slug) {
        return Err(bad());
    }
    let a = dir.join(format!("rh-{slug}.container"));
    let b = dir.join(format!("rh-{slug}-data.volume"));
    Ok((a, b, format!("rh-{slug}.service")))
}
fn require_installed(dir: &Path, slug: &str) -> io::Result<String> {
    let (a, b, unit) = unit_paths(dir, slug)?;
    installed_file(&a)?;
    installed_file(&b)?;
    Ok(unit)
}
fn checked_health_cmd(cmd: &str) -> io::Result<String> {
    if cmd.is_empty() || cmd.len() > 128 || cmd.trim() != cmd || cmd.split(' ').count() > 6 {
        return Err(bad());
    }
    for token in cmd.split(' ') {
        if token.is_empty()
            || token.contains("..")
            || !token
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._/-".contains(&b))
        {
            return Err(denied());
        }
    }
    Ok(format!(
        "HealthCmd={cmd}\nHealthInterval=15s\nHealthTimeout=5s\nHealthRetries=2\nHealthOnFailure=kill\n"
    ))
}
fn with_health(container: &str, cmd: &str) -> io::Result<String> {
    let config = checked_health_cmd(cmd)?;
    if container.matches("Pull=never\n").count() != 1 || container.contains("HealthCmd=") {
        return Err(denied());
    }
    Ok(container.replacen("Pull=never\n", &format!("Pull=never\n{config}"), 1))
}
#[derive(Default)]
struct InstallExtras<'a> {
    health: Option<&'a str>,
    data_destination: Option<&'a str>,
    secret_name: Option<&'a str>,
}
fn install(
    dir: &Path,
    slug: &str,
    image: &str,
    uid: u32,
    network: AppNetwork,
    extras: InstallExtras<'_>,
) -> io::Result<()> {
    if uid == 0 || uid > 65533 {
        return Err(bad());
    }
    let decl = OciApp {
        slug,
        image,
        memory_mib: 256,
        pids_limit: 128,
        run_uid: Some(uid),
        network,
    };
    let mut q = if let Some(path) = extras.data_destination {
        decl.render_at(path).map_err(|_| bad())?
    } else {
        decl.render().map_err(|_| bad())?
    };
    if let Some(healthcmd) = extras.health {
        q.container = with_health(&q.container, healthcmd)?;
    }
    if let Some(name) = extras.secret_name {
        bind_native_secret(&mut q, name).map_err(|_| bad())?;
    }
    // Flock is acquired BEFORE checking or writing descriptors: concurrent
    // installers and install/remove/start/switch may never race on a slug.
    let _guard = lock_app_install(dir, slug)?;
    // Bind only a previously created native secret in this same rootless store.
    // No creating, reading, printing, or marshalling secret content here.
    if let Some(name) = extras.secret_name {
        podman(&["secret", "exists", name])?;
    }
    // Strictly no pull or mutable tags. OCI digest must already be available.
    podman(&["image", "exists", image])?;
    let (a, b, _) = unit_paths(dir, slug)?;
    emit_files(dir, &q)?;
    if let Err(e) = systemctl(&["--user", "daemon-reload"]) {
        // Avoid silently retaining half-installed App descriptors.
        fs::remove_file(a)?;
        fs::remove_file(b)?;
        return Err(e);
    }
    Ok(())
}
fn remove(dir: &Path, slug: &str) -> io::Result<()> {
    let _guard = lock_app(dir, slug)?;
    let unit = require_installed(dir, slug)?;
    let (a, b, _) = unit_paths(dir, slug)?;
    if fs::symlink_metadata(a.with_extension("rollback")).is_ok() {
        return Err(denied());
    }
    systemctl(&["--user", "stop", &unit])?;
    fs::remove_file(a)?;
    fs::remove_file(b)?;
    File::open(dir)?.sync_all()?;
    systemctl(&["--user", "daemon-reload"])?;
    // No volume deletion: App data survives uninstall by default.
    Ok(())
}
// A short exclusive per-App advisory lock serializes separate manager invocations.
fn lock_app(dir: &Path, slug: &str) -> io::Result<File> {
    lock_app_inner(dir, slug, true)
}
// Install must acquire the same per-App lock BEFORE the descriptor exists.
// Never unlink the lock when an App is removed: doing so would admit two
// independent inode locks during a concurrent remove/reinstall transition.
fn lock_app_install(dir: &Path, slug: &str) -> io::Result<File> {
    lock_app_inner(dir, slug, false)
}
fn lock_app_inner(dir: &Path, slug: &str, require_existing: bool) -> io::Result<File> {
    let (a, _, _) = unit_paths(dir, slug)?;
    let lock = a.with_extension("lock");
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .mode(0o600)
        .custom_flags(nix::libc::O_NOFOLLOW)
        .open(lock)?;
    let m = file.metadata()?;
    if !m.is_file() || m.uid() != unsafe { nix::libc::geteuid() } || m.mode() & 0o077 != 0 {
        return Err(denied());
    }
    let code =
        unsafe { nix::libc::flock(file.as_raw_fd(), nix::libc::LOCK_EX | nix::libc::LOCK_NB) };
    if code != 0 {
        return Err(denied());
    }
    // Hold lock throughout validation; check for the real unit only after
    // fencing other processes, closing the install/remove TOCTOU window.
    if require_existing {
        installed_file(&a)?;
    }
    Ok(file)
}
fn extract_image(content: &str) -> io::Result<&str> {
    let mut matches = content.lines().filter_map(|s| s.strip_prefix("Image="));
    let img = matches.next().ok_or_else(bad)?;
    if matches.next().is_some() || !content.contains("Pull=never\n") {
        return Err(denied());
    }
    Ok(img)
}
fn validate_digest(slug: &str, image: &str) -> io::Result<()> {
    let app = OciApp {
        slug,
        image,
        memory_mib: 256,
        pids_limit: 128,
        run_uid: Some(999),
        network: AppNetwork::Offline,
    };
    app.render().map(|_| ()).map_err(|_| bad())
}
fn replace_image(content: &str, expected: &str, new_image: &str) -> io::Result<String> {
    if extract_image(content)? != expected {
        return Err(denied());
    }
    let prev = format!("Image={expected}\n");
    if content.matches(&prev).count() != 1 {
        return Err(denied());
    }
    Ok(content.replacen(&prev, &format!("Image={new_image}\n"), 1))
}
fn replace_private_atomic(dir: &Path, target: &Path, bytes: &str) -> io::Result<()> {
    let stage = target.with_extension("pending");
    // Reject existing stage, including symlink. Never replace arbitrary paths.
    write_private_new(&stage, bytes)?;
    if let Err(e) = fs::rename(&stage, target) {
        let _ = fs::remove_file(stage);
        return Err(e);
    }
    File::open(dir)?.sync_all()
}
fn require_health(slug: &str, content: &str) -> io::Result<()> {
    if !content.lines().any(|line| line.starts_with("HealthCmd=")) {
        return Ok(());
    }
    let name = format!("rh-{slug}");
    // Native Podman application-specific healthcheck, not systemd process
    // state. Bounded retries permit image startup before checking.
    for _ in 0..30 {
        if podman(&["healthcheck", "run", &name]).is_ok() {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(400));
    }
    Err(io::Error::other(
        "application health failed; rollback required",
    ))
}
fn restart_from_unit(unit: &str) -> io::Result<()> {
    systemctl(&["--user", "daemon-reload"])?;
    systemctl(&["--user", "start", unit])?;
    systemctl(&["--user", "is-active", "--quiet", unit])
}
fn switch_image(dir: &Path, slug: &str, expected: &str, new_image: &str) -> io::Result<()> {
    let _guard = lock_app(dir, slug)?;
    validate_digest(slug, expected)?;
    validate_digest(slug, new_image)?;
    if new_image == expected {
        return Err(bad());
    }
    podman(&["image", "exists", new_image])?;
    let (path, _, unit) = unit_paths(dir, slug)?;
    installed_file(&path)?;
    let old = fs::read_to_string(&path)?;
    let updated = replace_image(&old, expected, new_image)?;
    let rollback = path.with_extension("rollback");
    if fs::symlink_metadata(&rollback).is_ok() {
        return Err(denied());
    }
    // If stop fails, the old application remains untouched.
    systemctl(&["--user", "stop", &unit])?;
    write_private_new(&rollback, &old)?;
    if let Err(e) = replace_private_atomic(dir, &path, &updated)
        .and_then(|_| restart_from_unit(&unit))
        .and_then(|_| require_health(slug, &updated))
    {
        // Recover the exact old digest and restart original if the new
        // container fails systemd startup. Keep rollback file for diagnosis.
        replace_private_atomic(dir, &path, &old)?;
        restart_from_unit(&unit)?;
        return Err(e);
    }
    Ok(())
}
fn rollback_image(dir: &Path, slug: &str) -> io::Result<()> {
    let _guard = lock_app(dir, slug)?;
    let (path, _, unit) = unit_paths(dir, slug)?;
    installed_file(&path)?;
    let rollback = path.with_extension("rollback");
    installed_file(&rollback)?;
    let current = fs::read_to_string(&path)?;
    let previous = fs::read_to_string(&rollback)?;
    validate_digest(slug, extract_image(&previous)?)?;
    validate_digest(slug, extract_image(&current)?)?;
    podman(&["image", "exists", extract_image(&previous)?])?;
    // The rollback descriptor is a copy of the exact previous unit; enforce
    // no modified network, permissions, or capabilities during rollback.
    if current.replace(extract_image(&current)?, extract_image(&previous)?) != previous {
        return Err(denied());
    }
    systemctl(&["--user", "stop", &unit])?;
    if let Err(e) =
        replace_private_atomic(dir, &path, &previous).and_then(|_| restart_from_unit(&unit))
    {
        replace_private_atomic(dir, &path, &current)?;
        restart_from_unit(&unit)?;
        return Err(e);
    }
    fs::remove_file(&rollback)?;
    File::open(dir)?.sync_all()?;
    Ok(())
}
fn execute() -> io::Result<()> {
    // Two explicit controls: only ephemeral GitHub CI; no Cloud9 or any host.
    if env::var("GITHUB_ACTIONS").as_deref() != Ok("true")
        || env::var("HARS_DISPOSABLE_OCI_LAB").as_deref() != Ok("1")
        || unsafe { nix::libc::geteuid() } == 0
    {
        return Err(denied());
    }
    let args = env::args().collect::<Vec<_>>();
    if args.len() < 3 {
        return Err(bad());
    }
    let dir = units_dir()?;
    match args[1].as_str() {
        "--install-lab" if args.len() == 5 || args.len() == 7 => {
            let uid = args[4].parse::<u32>().map_err(|_| bad())?;
            let net = if args.len() == 7 {
                if args[5] != "--local-tcp" {
                    return Err(bad());
                }
                let (h, c) = args[6].split_once(':').ok_or_else(bad)?;
                AppNetwork::LoopbackTcp {
                    host_port: h.parse::<u16>().map_err(|_| bad())?,
                    container_port: c.parse::<u16>().map_err(|_| bad())?,
                }
            } else {
                AppNetwork::Offline
            };
            install(&dir, &args[2], &args[3], uid, net, InstallExtras::default())
        }
        "--start-lab" if args.len() == 3 => {
            let _guard = lock_app(&dir, &args[2])?;
            let unit = require_installed(&dir, &args[2])?;
            systemctl(&["--user", "start", &unit])
        }
        "--stop-lab" if args.len() == 3 => {
            let _guard = lock_app(&dir, &args[2])?;
            let unit = require_installed(&dir, &args[2])?;
            systemctl(&["--user", "stop", &unit])
        }
        "--install-health-lab" if args.len() == 6 => {
            let uid = args[4].parse::<u32>().map_err(|_| bad())?;
            install(
                &dir,
                &args[2],
                &args[3],
                uid,
                AppNetwork::Offline,
                InstallExtras {
                    health: Some(&args[5]),
                    ..Default::default()
                },
            )
        }
        "--install-path-lab" if args.len() == 6 || args.len() == 7 => {
            let uid = args[4].parse::<u32>().map_err(|_| bad())?;
            let network = if args.len() == 7 {
                let (host, container) = args[6].split_once(':').ok_or_else(bad)?;
                AppNetwork::LoopbackTcp {
                    host_port: host.parse::<u16>().map_err(|_| bad())?,
                    container_port: container.parse::<u16>().map_err(|_| bad())?,
                }
            } else {
                AppNetwork::Offline
            };
            install(
                &dir,
                &args[2],
                &args[3],
                uid,
                network,
                InstallExtras {
                    data_destination: Some(&args[5]),
                    ..Default::default()
                },
            )
        }
        "--install-secret-lab" if args.len() == 6 => {
            let uid = args[4].parse::<u32>().map_err(|_| bad())?;
            install(
                &dir,
                &args[2],
                &args[3],
                uid,
                AppNetwork::Offline,
                InstallExtras {
                    secret_name: Some(&args[5]),
                    ..Default::default()
                },
            )
        }
        "--switch-image-lab" if args.len() == 5 => switch_image(&dir, &args[2], &args[3], &args[4]),
        "--rollback-image-lab" if args.len() == 3 => rollback_image(&dir, &args[2]),
        "--remove-lab" if args.len() == 3 => remove(&dir, &args[2]),
        _ => Err(bad()),
    }
}
fn main() {
    if execute().is_err() {
        eprintln!("P8AR_NATIVE_GENERIC_OCI_MANAGER_REFUSED_OR_FAILED");
        std::process::exit(2)
    }
    println!("P8AR_NATIVE_GENERIC_OCI_MANAGER_OPERATION_SUCCEEDED");
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn names_have_no_path_escape() {
        for ok in ["redis", "mealie", "zigbee2mqtt", "test-name-12"] {
            assert!(sane_slug(ok))
        }
        for bad in [
            "",
            "A",
            "../../etc",
            "a/b",
            "a.b",
            "a\n[Install]",
            "%u",
            "-bad",
            "x=1",
        ] {
            assert!(!sane_slug(bad))
        }
    }
    #[test]
    fn new_install_writes_both_manifests_no_overwrite_and_preserves_private_mode() {
        let temp = tempfile::tempdir().unwrap();
        let image = format!("docker.io/library/redis@sha256:{}", "a".repeat(64));
        let u = OciApp {
            slug: "redis",
            image: &image,
            memory_mib: 256,
            pids_limit: 128,
            run_uid: Some(999),
            network: AppNetwork::Offline,
        }
        .render()
        .unwrap();
        emit_files(temp.path(), &u).unwrap();
        assert!(temp.path().join("rh-redis.container").exists());
        assert!(temp.path().join("rh-redis-data.volume").exists());
        assert_eq!(
            fs::metadata(temp.path().join("rh-redis.container"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        assert!(emit_files(temp.path(), &u).is_err());
    }
    #[test]
    fn pinned_digest_switch_only_changes_image_key_not_isolation_or_volumes() {
        let a = format!("docker.io/library/redis@sha256:{}", "a".repeat(64));
        let b = format!("docker.io/library/redis@sha256:{}", "b".repeat(64));
        let old = OciApp {
            slug: "redis",
            image: &a,
            memory_mib: 256,
            pids_limit: 128,
            run_uid: Some(999),
            network: AppNetwork::Offline,
        }
        .render()
        .unwrap()
        .container;
        let newer = replace_image(&old, &a, &b).unwrap();
        assert!(newer.contains(&format!("Image={b}")));
        assert_eq!(newer.replace(&b, &a), old);
        assert!(newer.contains("Network=none"));
        assert!(newer.contains("DropCapability=all"));
        assert!(newer.contains("Volume=rh-redis-data.volume:/data:U"));
        assert!(replace_image(&newer, &a, &b).is_err());
        assert!(validate_digest("redis", "docker.io/library/redis:latest").is_err());
    }
    #[test]
    fn operator_healthcheck_is_safe_no_shell_and_stays_in_existing_container() {
        let image = format!("docker.io/library/redis@sha256:{}", "a".repeat(64));
        let original = OciApp {
            slug: "redis",
            image: &image,
            memory_mib: 256,
            pids_limit: 128,
            run_uid: Some(999),
            network: AppNetwork::Offline,
        }
        .render()
        .unwrap()
        .container;
        let healthy = with_health(&original, "redis-cli ping").unwrap();
        assert!(healthy.contains("HealthCmd=redis-cli ping\n"));
        assert!(healthy.contains("HealthTimeout=5s"));
        assert!(healthy.contains("HealthOnFailure=kill"));
        assert!(healthy.contains("Network=none"));
        for bad in [
            "",
            "redis-cli; rm -rf /data",
            "redis-cli $(env)",
            "redis-cli\nping",
            "redis-cli && false",
            " ../bin/false",
            "../../etc/passwd",
        ] {
            assert!(with_health(&original, bad).is_err(), "{bad}");
        }
    }
    #[test]
    fn installation_fences_other_processes_and_retains_single_lock_inode() {
        let temp = tempfile::tempdir().unwrap();
        let guard = lock_app_install(temp.path(), "concurrent-app").unwrap();
        let meta = temp
            .path()
            .join("rh-concurrent-app.lock")
            .metadata()
            .unwrap();
        assert_eq!(meta.permissions().mode() & 0o777, 0o600);
        // flock on separate opens conflicts even within a single process.
        assert!(lock_app_install(temp.path(), "concurrent-app").is_err());
        assert!(lock_app(temp.path(), "concurrent-app").is_err());
        drop(guard);
        let later = lock_app_install(temp.path(), "concurrent-app").unwrap();
        assert_eq!(
            temp.path()
                .join("rh-concurrent-app.lock")
                .metadata()
                .unwrap()
                .ino(),
            meta.ino()
        );
        drop(later);
        let lock_path = temp.path().join("rh-fake.lock");
        std::os::unix::fs::symlink("/etc/passwd", &lock_path).unwrap();
        assert!(lock_app_install(temp.path(), "fake").is_err());
    }
    #[test]
    fn installed_start_stop_and_image_upgrade_share_the_same_lock() {
        let temp = tempfile::tempdir().unwrap();
        let container = temp.path().join("rh-redis.container");
        let volume = temp.path().join("rh-redis-data.volume");
        fs::write(&container, "[Container]\nImage=test\n").unwrap();
        fs::write(&volume, "[Volume]\n").unwrap();
        // Respect the managers owner-private unit contract even when the
        // developer or CI uses a group-writable (0002) umask.
        fs::set_permissions(&container, fs::Permissions::from_mode(0o600)).unwrap();
        fs::set_permissions(&volume, fs::Permissions::from_mode(0o600)).unwrap();
        let lock = lock_app(temp.path(), "redis").unwrap();
        assert!(lock_app(temp.path(), "redis").is_err());
        assert!(lock_app_install(temp.path(), "redis").is_err());
        drop(lock);
        assert!(lock_app(temp.path(), "redis").is_ok());
    }
    #[test]
    fn fake_or_duplicate_image_configuration_fails_closed() {
        assert!(extract_image("[Container]\nImage=a\nImage=b\nPull=never\n").is_err());
        assert!(extract_image("[Container]\nImage=a\n").is_err());
    }
    #[test]
    fn symlink_is_never_trusted_as_installed_app() {
        let temp = tempfile::tempdir().unwrap();
        let unit = temp.path().join("rh-redis.container");
        std::os::unix::fs::symlink("/etc/passwd", unit).unwrap();
        let vol = temp.path().join("rh-redis-data.volume");
        fs::write(vol, "[Volume]").unwrap();
        assert!(require_installed(temp.path(), "redis").is_err());
    }
}
