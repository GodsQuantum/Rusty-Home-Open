//! P8ar: lab-only *actual* rootless native systemd/Podman OCI App manager.
//! No root, no household HAOS, no user device access, no production API.
use hars_native_hub::oci_quadlet::{OciApp, QuadletUnits};
use std::{
    env,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    process::{Command, Stdio},
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
fn install(dir: &Path, slug: &str, image: &str, uid: u32) -> io::Result<()> {
    if uid == 0 || uid > 65533 {
        return Err(bad());
    }
    let decl = OciApp {
        slug,
        image,
        memory_mib: 256,
        pids_limit: 128,
        run_uid: Some(uid),
    };
    let q = decl.render().map_err(|_| bad())?;
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
    let unit = require_installed(dir, slug)?;
    systemctl(&["--user", "stop", &unit])?;
    let (a, b, _) = unit_paths(dir, slug)?;
    fs::remove_file(a)?;
    fs::remove_file(b)?;
    File::open(dir)?.sync_all()?;
    systemctl(&["--user", "daemon-reload"])?;
    // No volume deletion: App data survives uninstall by default.
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
        "--install-lab" if args.len() == 5 => {
            let uid = args[4].parse::<u32>().map_err(|_| bad())?;
            install(&dir, &args[2], &args[3], uid)
        }
        "--start-lab" if args.len() == 3 => {
            let unit = require_installed(&dir, &args[2])?;
            systemctl(&["--user", "start", &unit])
        }
        "--stop-lab" if args.len() == 3 => {
            let unit = require_installed(&dir, &args[2])?;
            systemctl(&["--user", "stop", &unit])
        }
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
    fn symlink_is_never_trusted_as_installed_app() {
        let temp = tempfile::tempdir().unwrap();
        let unit = temp.path().join("rh-redis.container");
        std::os::unix::fs::symlink("/etc/passwd", unit).unwrap();
        let vol = temp.path().join("rh-redis-data.volume");
        fs::write(vol, "[Volume]").unwrap();
        assert!(require_installed(temp.path(), "redis").is_err());
    }
}
