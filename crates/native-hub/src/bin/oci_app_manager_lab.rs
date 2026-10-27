//! P8ar: lab-only *actual* rootless native systemd/Podman OCI App manager.
//! No root, no household HAOS, no user device access, no production API.
use hars_native_hub::oci_quadlet::{
    AppNetwork, OciApp, QuadletUnits, bind_native_nested_existing_volume,
    bind_native_runtime_tmpfs, bind_native_secret, bind_native_service_dependency,
    bind_native_sibling_existing_volume,
};
use std::{
    collections::HashSet,
    env,
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
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
// A single rootless-user graph lock covers BOTH provider removal and
// dependent installation. Keep its inode forever to avoid split-brain locks.
fn lock_dependency_graph(dir: &Path) -> io::Result<File> {
    let path = dir.join(".rh-dependency-graph.lock");
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .mode(0o600)
        .custom_flags(nix::libc::O_NOFOLLOW)
        .open(path)?;
    let m = file.metadata()?;
    if !m.is_file() || m.uid() != unsafe { nix::libc::geteuid() } || m.mode() & 0o077 != 0 {
        return Err(denied());
    }
    let code =
        unsafe { nix::libc::flock(file.as_raw_fd(), nix::libc::LOCK_EX | nix::libc::LOCK_NB) };
    if code != 0 {
        return Err(denied());
    }
    Ok(file)
}

fn installed_dependency(dir: &Path, slug: &str) -> io::Result<Option<String>> {
    require_installed(dir, slug)?;
    let (path, _, _) = unit_paths(dir, slug)?;
    let content = fs::read_to_string(path)?;
    let header = format!("[Unit]\nDescription=Rusty Home isolated OCI App {slug} (lab only)\n");
    if !content.starts_with(&header)
        || content.matches("[Unit]\n").count() != 1
        || content.matches("\n[Container]\n").count() != 1
    {
        return Err(denied());
    }
    let mut binds = Vec::new();
    let mut after = Vec::new();
    for line in content.lines() {
        if let Some(value) = line.strip_prefix("BindsTo=") {
            binds.push(value);
        }
        if let Some(value) = line.strip_prefix("After=") {
            after.push(value);
        }
        if line.starts_with("Requires=") || line.starts_with("Wants=") {
            return Err(denied());
        }
    }
    if binds.is_empty() && after.is_empty() {
        return Ok(None);
    }
    if binds.len() != 1 || after.len() != 1 || binds[0] != after[0] {
        return Err(denied());
    }
    let name = binds[0]
        .strip_prefix("rh-")
        .and_then(|x| x.strip_suffix(".service"))
        .ok_or_else(denied)?;
    if !sane_slug(name) || name == slug {
        return Err(denied());
    }
    Ok(Some(name.to_string()))
}

fn walk_dependency_chain(dir: &Path, root: &str, blocked: Option<&str>) -> io::Result<()> {
    let mut seen = HashSet::new();
    let mut current = Some(root.to_string());
    while let Some(slug) = current {
        if !seen.insert(slug.clone()) || blocked == Some(slug.as_str()) {
            return Err(denied());
        }
        current = installed_dependency(dir, &slug)?;
    }
    Ok(())
}

fn validate_dependency_chain(dir: &Path, slug: &str, provider: &str) -> io::Result<()> {
    if !sane_slug(slug) || !sane_slug(provider) || slug == provider {
        return Err(bad());
    }
    walk_dependency_chain(dir, provider, Some(slug))
}

fn provider_has_dependents(dir: &Path, provider: &str) -> io::Result<bool> {
    if !sane_slug(provider) {
        return Err(bad());
    }
    let mut has_dependent = false;
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let filename = entry.file_name();
        let filename = filename.to_str().ok_or_else(denied)?;
        if let Some(slug) = filename
            .strip_prefix("rh-")
            .and_then(|name| name.strip_suffix(".container"))
        {
            if !sane_slug(slug) {
                return Err(denied());
            }
            // Refuse corrupt/cyclic graphs before touching ANY service.
            walk_dependency_chain(dir, slug, None)?;
            if slug != provider && installed_dependency(dir, slug)?.as_deref() == Some(provider) {
                has_dependent = true;
            }
        }
    }
    Ok(has_dependent)
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
// Podman >=5 native readiness. Do not apply to the legacy 4.9 health
// path: Ubuntu 24.04's Quadlet generator does not support Notify=healthy.
fn with_health_readiness(container: &str, cmd: &str) -> io::Result<String> {
    let health = with_health(container, cmd)?;
    if health.matches("HealthOnFailure=kill\n").count() != 1 || health.contains("Notify=") {
        return Err(denied());
    }
    Ok(health.replacen(
        "HealthOnFailure=kill\n",
        "HealthOnFailure=kill\nNotify=healthy\n",
        1,
    ))
}
#[derive(Default)]
struct InstallExtras<'a> {
    health: Option<&'a str>,
    health_readiness: bool,
    data_destination: Option<&'a str>,
    secret_name: Option<&'a str>,
    runtime_tmpfs: Option<&'a str>,
    nested_volume: Option<(&'a str, &'a str)>,
    resources: Option<(u16, u16)>,
    sibling_volume: Option<(&'a str, &'a str)>,
    dependency: Option<&'a str>,
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
    let (memory_mib, pids_limit) = extras.resources.unwrap_or((256, 128));
    // Bound explicitly requested lab resources, retain unchanged secure
    // production-profile defaults for ALL existing applications.
    if !(256..=2048).contains(&memory_mib) || !(128..=512).contains(&pids_limit) {
        return Err(bad());
    }
    let decl = OciApp {
        slug,
        image,
        memory_mib,
        pids_limit,
        run_uid: Some(uid),
        network,
    };
    let mut q = if let Some(path) = extras.data_destination {
        decl.render_at(path).map_err(|_| bad())?
    } else {
        decl.render().map_err(|_| bad())?
    };
    if extras.health_readiness && extras.health.is_none() {
        return Err(bad());
    }
    if let Some(healthcmd) = extras.health {
        q.container = if extras.health_readiness {
            with_health_readiness(&q.container, healthcmd)?
        } else {
            with_health(&q.container, healthcmd)?
        };
    }
    if let Some(name) = extras.secret_name {
        bind_native_secret(&mut q, name).map_err(|_| bad())?;
    }
    if let Some(path) = extras.runtime_tmpfs {
        bind_native_runtime_tmpfs(&mut q, path).map_err(|_| bad())?;
    }
    if let Some((volume, child)) = extras.nested_volume {
        let parent = extras.data_destination.ok_or_else(bad)?;
        // Reuse existing native container path validation, in addition to
        // requiring the *specific* parent and child topology below.
        decl.render_at(child).map_err(|_| bad())?;
        bind_native_nested_existing_volume(&mut q, slug, parent, volume, child)
            .map_err(|_| bad())?;
    }
    if let Some((volume, sibling)) = extras.sibling_volume {
        let data_path = extras.data_destination.ok_or_else(bad)?;
        decl.render_at(sibling).map_err(|_| bad())?;
        bind_native_sibling_existing_volume(&mut q, slug, data_path, volume, sibling)
            .map_err(|_| bad())?;
    }
    // Flock is acquired BEFORE checking or writing descriptors: concurrent
    // installers and install/remove/start/switch may never race on a slug.
    let _graph_guard = lock_dependency_graph(dir)?;
    ensure_no_pair_journal(dir)?;
    let _guard = lock_app_install(dir, slug)?;
    if let Some(provider) = extras.dependency {
        validate_dependency_chain(dir, slug, provider)?;
        bind_native_service_dependency(&mut q, slug, provider).map_err(|_| bad())?;
    }
    // Bind only a previously created native secret in this same rootless store.
    // No creating, reading, printing, or marshalling secret content here.
    if let Some(name) = extras.secret_name {
        podman(&["secret", "exists", name])?;
    }
    // This volume must already exist in this exact rootless Podman store.
    // Never let the engine silently create an anonymous image VOLUME child.
    if let Some((volume, _)) = extras.nested_volume {
        podman(&["volume", "exists", volume])?;
    }
    if let Some((volume, _)) = extras.sibling_volume {
        podman(&["volume", "exists", volume])?;
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

// P8cr: bounded atomic pair installation (descriptors only). Not a
// database/runtime transaction or HAOS migration.
fn pair_units(
    provider: &str,
    dependent: &str,
    provider_image: &str,
    dependent_image: &str,
    uid: u32,
) -> io::Result<(QuadletUnits, QuadletUnits)> {
    if provider == dependent
        || !sane_slug(provider)
        || !sane_slug(dependent)
        || uid == 0
        || uid > 65533
    {
        return Err(bad());
    }
    let make = |slug, image| {
        OciApp {
            slug,
            image,
            memory_mib: 256,
            pids_limit: 128,
            run_uid: Some(uid),
            network: AppNetwork::Offline,
        }
        .render()
        .map_err(|_| bad())
    };
    let a = make(provider, provider_image)?;
    let mut b = make(dependent, dependent_image)?;
    bind_native_service_dependency(&mut b, dependent, provider).map_err(|_| bad())?;
    Ok((a, b))
}

// All-or-none publication under the graph and per-App locks. A failed
// systemd reload rolls back all four new descriptors and reloads again.
// Pre-existing files, containers and persistent volumes are never removed.
fn publish_pair<F>(dir: &Path, a: &QuadletUnits, b: &QuadletUnits, reload: F) -> io::Result<()>
where
    F: Fn() -> io::Result<()>,
{
    let entries = [
        (dir.join(&a.container_name), a.container.as_str()),
        (dir.join(&a.volume_name), a.volume.as_str()),
        (dir.join(&b.container_name), b.container.as_str()),
        (dir.join(&b.volume_name), b.volume.as_str()),
    ];
    for (path, _) in &entries {
        match fs::symlink_metadata(path) {
            Ok(_) => return Err(denied()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => (),
            Err(e) => return Err(e),
        }
    }
    let mut written = Vec::new();
    let result = (|| {
        for (path, data) in &entries {
            // Track the inode immediately after create_new succeeds, even
            // if a later write/fsync fails with ENOSPC or EIO.
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(path)?;
            written.push(path.clone());
            file.write_all(data.as_bytes())?;
            file.sync_all()?;
        }
        File::open(dir)?.sync_all()?;
        reload()
    })();
    if let Err(original) = result {
        for path in written.iter().rev() {
            fs::remove_file(path)?;
        }
        File::open(dir)?.sync_all()?;
        reload()?;
        return Err(original);
    }
    Ok(())
}
fn install_pair(
    dir: &Path,
    provider: &str,
    provider_image: &str,
    dependent: &str,
    dependent_image: &str,
    uid: u32,
) -> io::Result<()> {
    let (a, b) = pair_units(provider, dependent, provider_image, dependent_image, uid)?;
    let _graph_guard = lock_dependency_graph(dir)?;
    ensure_no_pair_journal(dir)?;
    let mut names = [provider, dependent];
    names.sort_unstable();
    let _locks = [
        lock_app_install(dir, names[0])?,
        lock_app_install(dir, names[1])?,
    ];
    podman(&["image", "exists", provider_image])?;
    podman(&["image", "exists", dependent_image])?;
    publish_pair(dir, &a, &b, || systemctl(&["--user", "daemon-reload"]))
}

// P8cs: a single durable intent per rootless user. This journal is written
// and fsynced BEFORE the first native systemd start. A crashed coordinator
// leaves the pair fenced until an explicit, conservative stop-only recovery.
// No runtime data, volumes, or pre-existing services are ever deleted.
const PAIR_START_JOURNAL: &str = ".rh-start-pair.journal";
fn pair_journal_path(dir: &Path) -> PathBuf {
    dir.join(PAIR_START_JOURNAL)
}
fn ensure_no_pair_journal(dir: &Path) -> io::Result<()> {
    match fs::symlink_metadata(pair_journal_path(dir)) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        _ => Err(denied()),
    }
}
fn read_pair_journal(dir: &Path) -> io::Result<(String, String)> {
    let mut file = OpenOptions::new()
        .read(true)
        .custom_flags(nix::libc::O_NOFOLLOW)
        .open(pair_journal_path(dir))?;
    let m = file.metadata()?;
    if !m.is_file()
        || m.uid() != unsafe { nix::libc::geteuid() }
        || m.mode() & 0o077 != 0
        || m.len() > 150
    {
        return Err(denied());
    }
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    let mut lines = content.lines();
    if lines.next() != Some("P8CS1") {
        return Err(denied());
    }
    let provider = lines.next().ok_or_else(denied)?;
    let child = lines.next().ok_or_else(denied)?;
    if lines.next().is_some()
        || !content.ends_with('\n')
        || !sane_slug(provider)
        || !sane_slug(child)
        || provider == child
        || installed_dependency(dir, child)?.as_deref() != Some(provider)
    {
        return Err(denied());
    }
    require_installed(dir, provider)?;
    Ok((provider.to_owned(), child.to_owned()))
}
fn pair_locks<'a>(dir: &Path, provider: &'a str, child: &'a str) -> io::Result<[File; 2]> {
    if provider == child || !sane_slug(provider) || !sane_slug(child) {
        return Err(bad());
    }
    let mut names = [provider, child];
    names.sort_unstable();
    Ok([lock_app(dir, names[0])?, lock_app(dir, names[1])?])
}
fn pair_active(slug: &str) -> io::Result<bool> {
    if !sane_slug(slug) {
        return Err(bad());
    }
    Ok(systemctl(&[
        "--user",
        "is-active",
        "--quiet",
        &format!("rh-{slug}.service"),
    ])
    .is_ok())
}
fn recover_pair_locked(dir: &Path, provider: &str, child: &str) -> io::Result<()> {
    let (recorded_provider, recorded_child) = read_pair_journal(dir)?;
    if (recorded_provider.as_str(), recorded_child.as_str()) != (provider, child) {
        return Err(denied());
    }
    // Always stop dependent first; if either stop fails, keep the durable
    // journal to prevent a later invocation claiming a successful rollback.
    let child_stop = systemctl(&["--user", "stop", &format!("rh-{child}.service")]);
    let provider_stop = systemctl(&["--user", "stop", &format!("rh-{provider}.service")]);
    child_stop?;
    provider_stop?;
    if pair_active(provider)? || pair_active(child)? {
        return Err(io::Error::other("pair still active after recovery"));
    }
    fs::remove_file(pair_journal_path(dir))?;
    File::open(dir)?.sync_all()
}
fn start_pair(dir: &Path, provider: &str, child: &str) -> io::Result<()> {
    let _graph = lock_dependency_graph(dir)?;
    ensure_no_pair_journal(dir)?;
    let _locks = pair_locks(dir, provider, child)?;
    require_installed(dir, provider)?;
    if installed_dependency(dir, child)?.as_deref() != Some(provider) {
        return Err(denied());
    }
    // Never stop services that were active before this invocation.
    if pair_active(provider)? || pair_active(child)? {
        return Err(denied());
    }
    let journal = pair_journal_path(dir);
    write_private_new(&journal, &format!("P8CS1\n{provider}\n{child}\n"))?;
    File::open(dir)?.sync_all()?;
    let start = (|| {
        systemctl(&["--user", "start", &format!("rh-{provider}.service")])?;
        if !pair_active(provider)? {
            return Err(io::Error::other("provider did not become active"));
        }
        // Only a disposable GitHub runner can trigger this deliberate
        // coordinator SIGKILL. Recovery must occur in a separate process.
        if env::var("HARS_P8CS_KILL_AFTER_PROVIDER").as_deref() == Ok("1")
            && env::var("GITHUB_ACTIONS").as_deref() == Ok("true")
        {
            unsafe { nix::libc::kill(nix::libc::getpid(), nix::libc::SIGKILL) };
        }
        systemctl(&["--user", "start", &format!("rh-{child}.service")])?;
        if !pair_active(provider)? || !pair_active(child)? {
            return Err(io::Error::other(
                "pair failed post-start health of systemd units",
            ));
        }
        Ok(())
    })();
    if let Err(original) = start {
        recover_pair_locked(dir, provider, child)?;
        return Err(original);
    }
    fs::remove_file(journal)?;
    File::open(dir)?.sync_all()
}
fn recover_pair(dir: &Path) -> io::Result<()> {
    let _graph = lock_dependency_graph(dir)?;
    let (provider, child) = read_pair_journal(dir)?;
    let _locks = pair_locks(dir, &provider, &child)?;
    recover_pair_locked(dir, &provider, &child)
}

fn remove(dir: &Path, slug: &str) -> io::Result<()> {
    let _graph_guard = lock_dependency_graph(dir)?;
    ensure_no_pair_journal(dir)?;
    let _guard = lock_app(dir, slug)?;
    if provider_has_dependents(dir, slug)? {
        return Err(denied());
    }
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
    let _graph_guard = lock_dependency_graph(dir)?;
    ensure_no_pair_journal(dir)?;
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
    let _graph_guard = lock_dependency_graph(dir)?;
    ensure_no_pair_journal(dir)?;
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
    if args.len() < 2 {
        return Err(bad());
    }
    let dir = units_dir()?;
    match args[1].as_str() {
        "--start-pair-transaction-lab" if args.len() == 4 => start_pair(&dir, &args[2], &args[3]),
        "--recover-start-pair-lab" if args.len() == 2 => recover_pair(&dir),
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
        "--install-pair-dependent-lab" if args.len() == 7 => {
            let uid = args[6].parse::<u32>().map_err(|_| bad())?;
            install_pair(&dir, &args[2], &args[3], &args[4], &args[5], uid)
        }
        "--install-dependent-lab" if args.len() == 6 => {
            let uid = args[4].parse::<u32>().map_err(|_| bad())?;
            install(
                &dir,
                &args[2],
                &args[3],
                uid,
                AppNetwork::Offline,
                InstallExtras {
                    dependency: Some(&args[5]),
                    ..Default::default()
                },
            )
        }
        "--start-lab" if args.len() == 3 => {
            let _graph_guard = lock_dependency_graph(&dir)?;
            ensure_no_pair_journal(&dir)?;
            let _guard = lock_app(&dir, &args[2])?;
            let unit = require_installed(&dir, &args[2])?;
            systemctl(&["--user", "start", &unit])
        }
        "--stop-lab" if args.len() == 3 => {
            let _graph_guard = lock_dependency_graph(&dir)?;
            ensure_no_pair_journal(&dir)?;
            let _guard = lock_app(&dir, &args[2])?;
            let unit = require_installed(&dir, &args[2])?;
            systemctl(&["--user", "stop", &unit])
        }
        "--install-health-dependent-ready-lab" if args.len() == 7 => {
            let uid = args[4].parse::<u32>().map_err(|_| bad())?;
            install(
                &dir,
                &args[2],
                &args[3],
                uid,
                AppNetwork::Offline,
                InstallExtras {
                    health: Some(&args[5]),
                    health_readiness: true,
                    dependency: Some(&args[6]),
                    ..Default::default()
                },
            )
        }
        "--install-health-ready-lab" if args.len() == 6 => {
            let uid = args[4].parse::<u32>().map_err(|_| bad())?;
            install(
                &dir,
                &args[2],
                &args[3],
                uid,
                AppNetwork::Offline,
                InstallExtras {
                    health: Some(&args[5]),
                    health_readiness: true,
                    ..Default::default()
                },
            )
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
        "--install-path-resources-lab" if args.len() == 8 || args.len() == 10 => {
            let uid = args[4].parse::<u32>().map_err(|_| bad())?;
            let memory_mib = args[6].parse::<u16>().map_err(|_| bad())?;
            let pids_limit = args[7].parse::<u16>().map_err(|_| bad())?;
            install(
                &dir,
                &args[2],
                &args[3],
                uid,
                AppNetwork::Offline,
                InstallExtras {
                    data_destination: Some(&args[5]),
                    resources: Some((memory_mib, pids_limit)),
                    sibling_volume: (args.len() == 10).then(|| (&args[8][..], &args[9][..])),
                    ..Default::default()
                },
            )
        }
        "--install-dual-path-lab" if args.len() == 8 => {
            let uid = args[4].parse::<u32>().map_err(|_| bad())?;
            install(
                &dir,
                &args[2],
                &args[3],
                uid,
                AppNetwork::Offline,
                InstallExtras {
                    data_destination: Some(&args[5]),
                    nested_volume: Some((&args[6], &args[7])),
                    ..Default::default()
                },
            )
        }
        "--install-path-tmpfs-lab" if args.len() == 7 => {
            let uid = args[4].parse::<u32>().map_err(|_| bad())?;
            install(
                &dir,
                &args[2],
                &args[3],
                uid,
                AppNetwork::Offline,
                InstallExtras {
                    data_destination: Some(&args[5]),
                    runtime_tmpfs: Some(&args[6]),
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
    fn durable_pair_journal_validates_identity_and_refuses_tampering() {
        let dir = tempfile::tempdir().unwrap();
        let digest = format!("docker.io/library/redis@sha256:{}", "a".repeat(64));
        let (a, b) =
            pair_units("durable-provider", "durable-child", &digest, &digest, 999).unwrap();
        publish_pair(dir.path(), &a, &b, || Ok(())).unwrap();
        let journal = pair_journal_path(dir.path());
        assert!(ensure_no_pair_journal(dir.path()).is_ok());
        write_private_new(&journal, "P8CS1\ndurable-provider\ndurable-child\n").unwrap();
        assert!(ensure_no_pair_journal(dir.path()).is_err());
        assert_eq!(
            read_pair_journal(dir.path()).unwrap(),
            ("durable-provider".into(), "durable-child".into())
        );
        assert_eq!(
            fs::metadata(&journal).unwrap().permissions().mode() & 0o777,
            0o600
        );
        fs::write(&journal, "P8CS1\ndurable-child\ndurable-provider\n").unwrap();
        assert!(read_pair_journal(dir.path()).is_err());
        fs::write(&journal, "P8CS1\ndurable-provider\ndurable-child\nextra\n").unwrap();
        assert!(read_pair_journal(dir.path()).is_err());
        fs::write(&journal, "P8CS1\ndurable-provider\ndurable-child\n").unwrap();
        fs::set_permissions(&journal, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(read_pair_journal(dir.path()).is_err());
        fs::remove_file(&journal).unwrap();
        std::os::unix::fs::symlink("/etc/passwd", &journal).unwrap();
        assert!(read_pair_journal(dir.path()).is_err());
        assert!(ensure_no_pair_journal(dir.path()).is_err());
        fs::remove_file(journal).unwrap();
        assert!(ensure_no_pair_journal(dir.path()).is_ok());
    }
    #[test]
    fn atomic_pair_real_filesystem_and_reload_failure() {
        let dir = tempfile::tempdir().unwrap();
        let digest = format!("docker.io/library/redis@sha256:{}", "a".repeat(64));
        let (a, b) = pair_units("pair-provider", "pair-child", &digest, &digest, 999).unwrap();
        assert!(b.container.contains("BindsTo=rh-pair-provider.service"));
        assert!(a.container.contains("Network=none"));
        assert!(a.container.contains("ReadOnly=true"));
        assert!(pair_units("same", "same", &digest, &digest, 999).is_err());
        assert!(pair_units("one", "two", &digest, &digest, 0).is_err());
        assert!(pair_units("one", "two", "redis:latest", &digest, 999).is_err());
        let collision = dir.path().join(&b.volume_name);
        fs::write(&collision, "DO-NOT-OVERWRITE").unwrap();
        assert!(publish_pair(dir.path(), &a, &b, || Ok(())).is_err());
        assert_eq!(fs::read_to_string(&collision).unwrap(), "DO-NOT-OVERWRITE");
        assert!(!dir.path().join(&a.container_name).exists());
        fs::remove_file(collision).unwrap();
        let calls = std::cell::Cell::new(0);
        assert!(
            publish_pair(dir.path(), &a, &b, || {
                calls.set(calls.get() + 1);
                if calls.get() == 1 {
                    Err(io::Error::other("injected reload failure"))
                } else {
                    Ok(())
                }
            })
            .is_err()
        );
        assert_eq!(calls.get(), 2);
        for name in [
            &a.container_name,
            &a.volume_name,
            &b.container_name,
            &b.volume_name,
        ] {
            assert!(!dir.path().join(name).exists());
        }
        publish_pair(dir.path(), &a, &b, || Ok(())).unwrap();
        for name in [
            &a.container_name,
            &a.volume_name,
            &b.container_name,
            &b.volume_name,
        ] {
            assert_eq!(
                fs::metadata(dir.path().join(name))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }
        assert!(publish_pair(dir.path(), &a, &b, || Ok(())).is_err());
    }

    #[test]
    fn native_dependency_chain_and_provider_removal_are_fenced() {
        let dir = tempfile::tempdir().unwrap();
        let image = format!("docker.io/library/redis@sha256:{}", "a".repeat(64));
        for slug in ["provider", "consumer", "leaf"] {
            let mut u = OciApp {
                slug,
                image: &image,
                memory_mib: 256,
                pids_limit: 128,
                run_uid: Some(999),
                network: AppNetwork::Offline,
            }
            .render()
            .unwrap();
            if slug == "consumer" {
                bind_native_service_dependency(&mut u, slug, "provider").unwrap();
            }
            if slug == "leaf" {
                bind_native_service_dependency(&mut u, slug, "consumer").unwrap();
            }
            emit_files(dir.path(), &u).unwrap();
        }
        validate_dependency_chain(dir.path(), "newapp", "leaf").unwrap();
        assert!(provider_has_dependents(dir.path(), "provider").unwrap());
        assert!(provider_has_dependents(dir.path(), "consumer").unwrap());
        assert!(!provider_has_dependents(dir.path(), "leaf").unwrap());
        assert!(validate_dependency_chain(dir.path(), "provider", "leaf").is_err());
        assert!(validate_dependency_chain(dir.path(), "missing", "absent").is_err());
        let provider = dir.path().join("rh-provider.container");
        let old = fs::read_to_string(&provider).unwrap();
        let corrupted = old.replace(
            " (lab only)\n",
            " (lab only)\nBindsTo=rh-leaf.service\nAfter=rh-leaf.service\n",
        );
        fs::write(&provider, corrupted).unwrap();
        assert!(validate_dependency_chain(dir.path(), "newapp", "leaf").is_err());
        assert!(provider_has_dependents(dir.path(), "provider").is_err());
    }

    #[test]
    fn graph_lock_refuses_concurrent_edit_and_symlink() {
        let dir = tempfile::tempdir().unwrap();
        let guard = lock_dependency_graph(dir.path()).unwrap();
        assert!(lock_dependency_graph(dir.path()).is_err());
        drop(guard);
        assert!(lock_dependency_graph(dir.path()).is_ok());
        fs::remove_file(dir.path().join(".rh-dependency-graph.lock")).unwrap();
        std::os::unix::fs::symlink("/etc/passwd", dir.path().join(".rh-dependency-graph.lock"))
            .unwrap();
        assert!(lock_dependency_graph(dir.path()).is_err());
    }
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
        assert!(!healthy.contains("Notify=healthy"));
        let ready = with_health_readiness(&original, "redis-cli ping").unwrap();
        assert!(ready.contains("Notify=healthy\n"));
        assert!(ready.contains("HealthCmd=redis-cli ping\n"));
        assert!(with_health_readiness(&ready, "redis-cli ping").is_err());
        assert!(!original.contains("Notify=healthy"));
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
