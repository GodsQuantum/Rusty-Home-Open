//! Generic offline Home Assistant Supervisor App repo inspector.
//! App names/slugs are unbounded by an allowlist; no install/start/socket IO.
#[cfg(feature = "managed-apps-preview")]
pub mod audit_lab;
#[cfg(feature = "managed-apps-preview")]
pub mod managed_apps;
#[cfg(feature = "persistent-session-lab")]
pub mod persistent_session_lab;
#[cfg(feature = "managed-apps-preview")]
pub mod rbac_lab;
#[cfg(feature = "managed-apps-preview")]
pub mod session_lab;

#[cfg(feature = "core-user-live-lab")]
pub mod core_user_live_lab;
#[cfg(feature = "core-user-snapshot-lab")]
pub mod core_user_snapshot_lab;
#[cfg(feature = "ingress-identity-lab")]
pub mod ingress_identity_lab;

pub mod catalog_filter;
pub mod official_store;
pub mod official_transport;
use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Number, Value};
use std::collections::{BTreeSet, HashSet};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

const MAX_FILE: u64 = 65_536;
const MAX_APPS: usize = 64;
const MAX_VISITED: usize = 8192;
const MAX_DEPTH: usize = 12;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Repository {
    pub name: String,
    pub apps: Vec<AppEvidence>,
    /// Structural inspection must never grant container execution permissions.
    pub activation_authorized: bool,
}
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AppEvidence {
    pub slug: String,
    pub version: String,
    pub path: String,
    pub architectures: Vec<String>,
    /// Image name is intentionally not exposed; metadata classification only.
    pub image_source: String,
    pub requirements: Vec<String>,
    /// Additional flags may be valid in future Supervisor, but need explicit review.
    pub unresolved_fields: Vec<String>,
    pub runtime_compatible: bool,
    pub activation_authorized: bool,
}
#[derive(Debug, PartialEq, Eq)]
pub enum InspectError {
    UnsafePath,
    InvalidManifest,
    DuplicateSlug,
    TooLarge,
    MissingRepository,
    Io(String),
}
impl From<std::io::Error> for InspectError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e.to_string())
    }
}

/// Whole-document duplicate-key rejection, including nested options.
/// serde_json::Value alone silently overwrites duplicate JSON object keys.
struct NoDuplicates(Value);
impl<'de> Deserialize<'de> for NoDuplicates {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct StrictVisitor;
        impl<'de> Visitor<'de> for StrictVisitor {
            type Value = NoDuplicates;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a JSON/YAML value without duplicate mapping keys")
            }
            fn visit_bool<E: de::Error>(self, v: bool) -> Result<Self::Value, E> {
                Ok(NoDuplicates(Value::Bool(v)))
            }
            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
                Ok(NoDuplicates(Value::Number(v.into())))
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
                Ok(NoDuplicates(Value::Number(v.into())))
            }
            fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
                let num = Number::from_f64(v).ok_or_else(|| E::custom("non-finite value"))?;
                Ok(NoDuplicates(Value::Number(num)))
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(NoDuplicates(Value::String(v.to_string())))
            }
            fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
                Ok(NoDuplicates(Value::String(v)))
            }
            fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(NoDuplicates(Value::Null))
            }
            fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(NoDuplicates(Value::Null))
            }
            fn visit_some<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
                NoDuplicates::deserialize(d)
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut a: A) -> Result<Self::Value, A::Error> {
                let mut result = Vec::new();
                while let Some(item) = a.next_element::<NoDuplicates>()? {
                    if result.len() >= 4096 {
                        return Err(de::Error::custom("too many YAML list elements"));
                    }
                    result.push(item.0);
                }
                Ok(NoDuplicates(Value::Array(result)))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut a: A) -> Result<Self::Value, A::Error> {
                let mut map = Map::new();
                while let Some((key, item)) = a.next_entry::<String, NoDuplicates>()? {
                    if map.len() >= 4096 {
                        return Err(de::Error::custom("too many YAML keys"));
                    }
                    if map.insert(key, item.0).is_some() {
                        return Err(de::Error::custom("duplicate YAML/JSON mapping key"));
                    }
                }
                Ok(NoDuplicates(Value::Object(map)))
            }
        }
        deserializer.deserialize_any(StrictVisitor)
    }
}

fn safe_component(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 100
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-' | b'.'))
        && s != "."
        && s != ".."
        && !s.starts_with('.')
}
fn safe_text(s: &str) -> bool {
    !s.is_empty() && s.len() <= 256 && !s.chars().any(char::is_control)
}
fn regular_file(path: &Path) -> Result<u64, InspectError> {
    let m = fs::symlink_metadata(path)?;
    if m.file_type().is_symlink() || !m.is_file() {
        return Err(InspectError::UnsafePath);
    }
    if m.len() > MAX_FILE {
        return Err(InspectError::TooLarge);
    }
    Ok(m.len())
}
fn read_map(path: &Path) -> Result<Map<String, Value>, InspectError> {
    regular_file(path)?;
    let bytes = fs::read(path)?;
    let value = if path.extension().and_then(|x| x.to_str()) == Some("json") {
        serde_json::from_slice::<NoDuplicates>(&bytes)
            .map_err(|_| InspectError::InvalidManifest)?
            .0
    } else {
        serde_yaml_ng::from_slice::<NoDuplicates>(&bytes)
            .map_err(|_| InspectError::InvalidManifest)?
            .0
    };
    value
        .as_object()
        .cloned()
        .ok_or(InspectError::InvalidManifest)
}
fn field<'a>(obj: &'a Map<String, Value>, key: &str) -> Result<&'a str, InspectError> {
    let s = obj
        .get(key)
        .and_then(Value::as_str)
        .ok_or(InspectError::InvalidManifest)?;
    if !safe_text(s) {
        return Err(InspectError::InvalidManifest);
    }
    Ok(s)
}
fn boolean(obj: &Map<String, Value>, key: &str) -> Result<bool, InspectError> {
    match obj.get(key) {
        None => Ok(false),
        Some(Value::Bool(b)) => Ok(*b),
        _ => Err(InspectError::InvalidManifest),
    }
}
fn array_strings(obj: &Map<String, Value>, key: &str) -> Result<Vec<String>, InspectError> {
    let values = obj
        .get(key)
        .and_then(Value::as_array)
        .ok_or(InspectError::InvalidManifest)?;
    if values.is_empty() || values.len() > 16 {
        return Err(InspectError::InvalidManifest);
    }
    values
        .iter()
        .map(|x| {
            let s = x.as_str().ok_or(InspectError::InvalidManifest)?;
            if !safe_component(s) {
                return Err(InspectError::InvalidManifest);
            }
            Ok(s.to_string())
        })
        .collect()
}
fn classified_capabilities(
    config: &Map<String, Value>,
) -> Result<(Vec<String>, Vec<String>), InspectError> {
    let known: HashSet<&str> = [
        "name",
        "version",
        "slug",
        "description",
        "arch",
        "image",
        "url",
        "machine",
        "homeassistant",
        "startup",
        "boot",
        "stage",
        "init",
        "timeout",
        "options",
        "schema",
        "ingress",
        "ingress_port",
        "ingress_entry",
        "ingress_stream",
        "panel_icon",
        "panel_title",
        "panel_admin",
        "ports",
        "ports_description",
        "webui",
        "watchdog",
        "host_network",
        "map",
        "devices",
        "uart",
        "usb",
        "gpio",
        "udev",
        "devicetree",
        "video",
        "audio",
        "kernel_modules",
        "full_access",
        "privileged",
        "apparmor",
        "host_dbus",
        "host_pid",
        "host_uts",
        "host_ipc",
        "realtime",
        "journald",
        "ulimits",
        "homeassistant_api",
        "hassio_api",
        "hassio_role",
        "auth_api",
        "docker_api",
        "services",
        "discovery",
        "backup",
        "backup_pre",
        "backup_post",
        "backup_exclude",
        "snapshot",
        "snapshot_pre",
        "snapshot_post",
        "snapshot_exclude",
        "auto_uart",
        "advanced",
        "codenotary",
        "environment",
        "breaking_versions",
        "timeout",
        "ingress_panel",
        "codenotary",
        "gpio",
        "s6_overlay",
        "legacy",
        "init",
    ]
    .into_iter()
    .collect();
    let mut req = BTreeSet::<String>::new();
    let mut unknown = BTreeSet::<String>::new();
    for k in config.keys() {
        if !known.contains(k.as_str()) {
            // Never include values of unknown fields: could contain passwords.
            unknown.insert(if safe_component(k) {
                k.clone()
            } else {
                "nonstandard_extension".into()
            });
        }
    }
    for key in [
        "usb",
        "uart",
        "gpio",
        "udev",
        "devicetree",
        "video",
        "audio",
        "kernel_modules",
        "full_access",
        "docker_api",
        "host_dbus",
        "host_pid",
        "host_uts",
        "host_ipc",
        "host_network",
        "homeassistant_api",
        "hassio_api",
        "auth_api",
        "realtime",
        "journald",
    ] {
        if boolean(config, key)? {
            req.insert(key.to_string());
        }
    }
    if config
        .get("devices")
        .is_some_and(|v| v.as_array().is_some_and(|a| !a.is_empty()))
    {
        req.insert("hardware_devices".into());
    }
    if config
        .get("privileged")
        .is_some_and(|v| v.as_array().is_some_and(|a| !a.is_empty()))
    {
        req.insert("linux_capabilities".into());
    }
    if boolean(config, "hassio_api")? {
        let role = config
            .get("hassio_role")
            .and_then(Value::as_str)
            .unwrap_or("default");
        req.insert(format!(
            "supervisor_api:{}",
            if matches!(
                role,
                "default" | "homeassistant" | "backup" | "manager" | "admin"
            ) {
                role
            } else {
                "unknown_role"
            }
        ));
    }
    if config
        .get("ports")
        .is_some_and(|v| v.as_object().is_some_and(|m| !m.is_empty()))
    {
        req.insert("published_ports".into());
    }
    if boolean(config, "ingress")? {
        req.insert("ingress".into());
    }
    if config
        .get("services")
        .is_some_and(|v| v.as_array().is_some_and(|a| !a.is_empty()))
    {
        req.insert("services".into());
    }
    if config
        .get("discovery")
        .is_some_and(|v| v.as_array().is_some_and(|a| !a.is_empty()))
    {
        req.insert("discovery".into());
    }
    match config.get("backup").and_then(Value::as_str) {
        Some("cold") => {
            req.insert("backup:cold".into());
        }
        Some("hot") | None => {}
        Some(_) => {
            req.insert("backup:unknown".into());
        }
    }
    if let Some(Value::Array(mappings)) = config.get("map") {
        if mappings.len() > 64 {
            return Err(InspectError::InvalidManifest);
        }
        for entry in mappings {
            match entry {
                Value::Object(m) => {
                    let source = m
                        .get("type")
                        .and_then(Value::as_str)
                        .ok_or(InspectError::InvalidManifest)?;
                    if !safe_component(source) {
                        return Err(InspectError::InvalidManifest);
                    }
                    let writable = match m.get("read_only") {
                        Some(Value::Bool(b)) => !b,
                        None => false, // Supervisor defaults to read-only except app-private /data.
                        _ => return Err(InspectError::InvalidManifest),
                    };
                    req.insert(format!("mount:{}", source));
                    if writable {
                        req.insert(format!("writable_mount:{}", source));
                    }
                    if source == "data" {
                        req.insert("writable_mount:data".into());
                    }
                }
                Value::String(old) => {
                    // Legacy "share:rw" format: accepted for inventory but requires adapter.
                    let (ty, mode) = old.split_once(':').unwrap_or((old, "ro"));
                    if !safe_component(ty) {
                        return Err(InspectError::InvalidManifest);
                    }
                    req.insert(format!("mount:{}", ty));
                    if mode == "rw" {
                        req.insert(format!("writable_mount:{}", ty));
                    }
                    req.insert("legacy_mount_format".into());
                }
                _ => return Err(InspectError::InvalidManifest),
            }
        }
    } else if config.contains_key("map") {
        return Err(InspectError::InvalidManifest);
    }
    Ok((req.into_iter().collect(), unknown.into_iter().collect()))
}
fn inspect_app(root: &Path, path: &Path) -> Result<AppEvidence, InspectError> {
    let config = read_map(path)?;
    let slug = field(&config, "slug")?.to_string();
    if !safe_component(&slug) {
        return Err(InspectError::InvalidManifest);
    }
    field(&config, "name")?;
    field(&config, "description")?;
    let version = field(&config, "version")?.to_string();
    let architectures = array_strings(&config, "arch")?;
    let image_source = if config.contains_key("image") {
        field(&config, "image")?;
        "published_image".to_string()
    } else {
        let dockerfile = path
            .parent()
            .ok_or(InspectError::InvalidManifest)?
            .join("Dockerfile");
        regular_file(&dockerfile)?;
        "dockerfile_build".to_string()
    };
    let (mut requirements, mut unresolved_fields) = classified_capabilities(&config)?;
    // Architecture matching must be verified per actual host at install time.
    if architectures
        .iter()
        .any(|a| !matches!(a.as_str(), "amd64" | "aarch64" | "armv7" | "armhf" | "i386"))
    {
        unresolved_fields.push("unknown_architecture".into());
        unresolved_fields.sort();
        unresolved_fields.dedup();
    }
    if image_source == "dockerfile_build" {
        requirements.push("build_on_target".into());
        requirements.sort();
    }
    let path = path
        .strip_prefix(root)
        .map_err(|_| InspectError::UnsafePath)?
        .to_str()
        .ok_or(InspectError::UnsafePath)?
        .replace('\\', "/");
    Ok(AppEvidence {
        slug,
        version,
        path,
        architectures,
        image_source,
        requirements,
        unresolved_fields,
        runtime_compatible: false,
        activation_authorized: false,
    })
}
pub fn inspect_repository(root: &Path) -> Result<Repository, InspectError> {
    let m = fs::symlink_metadata(root)?;
    if !m.is_dir() || m.file_type().is_symlink() {
        return Err(InspectError::UnsafePath);
    }
    let repo_file = ["repository.yaml", "repository.yml", "repository.json"]
        .iter()
        .map(|p| root.join(p))
        .filter(|p| p.exists())
        .collect::<Vec<_>>();
    if repo_file.is_empty() {
        return Err(InspectError::MissingRepository);
    }
    if repo_file.len() != 1 {
        return Err(InspectError::InvalidManifest);
    }
    let meta = read_map(&repo_file[0])?;
    let name = field(&meta, "name")?.to_string();
    let mut apps = Vec::new();
    let mut names = HashSet::new();
    let mut stack: Vec<(PathBuf, usize)> = vec![(root.to_path_buf(), 0)];
    let mut visited = 0usize;
    while let Some((dir, depth)) = stack.pop() {
        if depth > MAX_DEPTH {
            return Err(InspectError::TooLarge);
        }
        let mut entries = fs::read_dir(dir)?.collect::<Result<Vec<_>, _>>()?;
        entries.sort_by_key(|a| a.file_name());
        for entry in entries {
            visited += 1;
            if visited > MAX_VISITED {
                return Err(InspectError::TooLarge);
            }
            let filename = entry.file_name();
            let key = filename.to_str().ok_or(InspectError::UnsafePath)?;
            // Supervisor recursively skips hidden directories and rootfs.
            if key.starts_with('.') || key == "rootfs" {
                continue;
            }
            if !safe_component(key) {
                return Err(InspectError::UnsafePath);
            }
            let ftype = entry.file_type()?;
            if ftype.is_symlink() || (!ftype.is_file() && !ftype.is_dir()) {
                return Err(InspectError::UnsafePath);
            }
            if ftype.is_dir() {
                stack.push((entry.path(), depth + 1));
                continue;
            }
            if !matches!(key, "config.yaml" | "config.yml" | "config.json") {
                continue;
            }
            if apps.len() >= MAX_APPS {
                return Err(InspectError::TooLarge);
            }
            let app = inspect_app(root, &entry.path())?;
            if !names.insert(app.slug.to_ascii_lowercase()) {
                return Err(InspectError::DuplicateSlug);
            }
            if apps.iter().any(|other: &AppEvidence| {
                Path::new(&other.path).parent() == Path::new(&app.path).parent()
            }) {
                return Err(InspectError::InvalidManifest);
            }
            apps.push(app);
        }
    }
    apps.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(Repository {
        name,
        apps,
        activation_authorized: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    fn make() -> tempfile::TempDir {
        let d = tempfile::tempdir().unwrap();
        fs::write(d.path().join("repository.yaml"), "name: Dynamic Apps\n").unwrap();
        d
    }
    fn write(root: &Path, sub: &str, content: &str) {
        let p = root.join(sub);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, content).unwrap();
    }
    const SIMPLE: &str = "name: Future timer\nversion: '1.2.0'\nslug: future_timer\ndescription: Unknown app\narch: [amd64, aarch64]\nimage: ghcr.io/future/timer\nstartup: once\nbackup: cold\n";
    const BRIDGE: &str = "name: Unknown camera bridge\nversion: '2026.10'\nslug: open_camera\ndescription: Unknown vendor\narch: [amd64]\nimage: ghcr.io/unknown/cam\ningress: true\ningress_port: 8080\nmap:\n  - type: media\n    read_only: false\nusb: true\nfull_access: true\ndocker_api: true\nhassio_api: true\nhassio_role: admin\nports:\n  9988/tcp: 9988\nsome_future_setting: true\n";
    #[test]
    fn arbitrary_nested_json_and_yaml_with_no_slug_allowlist() {
        let d = make();
        write(d.path(), "group/deep/config.yaml", SIMPLE);
        write(
            d.path(),
            "nested/other/config.json",
            r#"{"name":"New","version":"5.0","slug":"entirely_new","description":"future","arch":["aarch64"],"image":"ghcr.io/new/unknown"}"#,
        );
        let found = inspect_repository(d.path()).unwrap();
        assert_eq!(found.name, "Dynamic Apps");
        assert_eq!(found.apps.len(), 2);
        assert!(found.apps.iter().any(|app| app.slug == "future_timer"));
        assert!(found.apps.iter().any(|app| app.slug == "entirely_new"));
        assert!(
            found
                .apps
                .iter()
                .all(|x| !x.activation_authorized && !x.runtime_compatible)
        );
        assert!(!found.activation_authorized);
        assert!(
            found.apps[0]
                .requirements
                .contains(&"backup:cold".to_string())
        );
    }
    #[test]
    fn unknown_and_dangerous_capabilities_reported_not_authorized() {
        let d = make();
        write(d.path(), "vendor/bridge/config.yml", BRIDGE);
        let a = inspect_repository(d.path()).unwrap().apps.remove(0);
        for req in [
            "usb",
            "full_access",
            "docker_api",
            "supervisor_api:admin",
            "writable_mount:media",
            "ingress",
            "published_ports",
        ] {
            assert!(a.requirements.contains(&req.to_string()), "missing {req}");
        }
        assert!(
            a.unresolved_fields
                .contains(&"some_future_setting".to_string())
        );
        assert!(!a.activation_authorized);
    }
    #[test]
    fn duplicate_slug_in_separate_folders_is_denied() {
        let d = make();
        write(d.path(), "one/config.yaml", SIMPLE);
        write(d.path(), "two/config.yaml", SIMPLE);
        assert!(matches!(
            inspect_repository(d.path()),
            Err(InspectError::DuplicateSlug)
        ));
    }
    #[test]
    fn symlinked_configs_and_directories_refused() {
        let d = make();
        write(d.path(), "valid/config.yaml", SIMPLE);
        std::os::unix::fs::symlink("valid", d.path().join("link")).unwrap();
        assert!(matches!(
            inspect_repository(d.path()),
            Err(InspectError::UnsafePath)
        ));
        fs::remove_file(d.path().join("link")).unwrap();
        std::os::unix::fs::symlink("valid/config.yaml", d.path().join("config.yaml")).unwrap();
        assert!(matches!(
            inspect_repository(d.path()),
            Err(InspectError::UnsafePath)
        ));
        assert!(matches!(
            inspect_repository(tempfile::tempdir().unwrap().path()),
            Err(InspectError::MissingRepository)
        ));
    }
    #[test]
    fn hidden_and_rootfs_skipped() {
        let d = make();
        write(d.path(), "real/config.yaml", SIMPLE);
        write(d.path(), "rootfs/fake/config.yaml", BRIDGE);
        write(d.path(), ".git/nested/config.yaml", BRIDGE);
        let found = inspect_repository(d.path()).unwrap();
        assert_eq!(found.apps.len(), 1);
    }
    #[test]
    fn unsafe_duplicate_json_keys_and_bad_types_denied() {
        let d = make();
        write(
            d.path(),
            "bad/config.json",
            r#"{"name":"A","version":"1","slug":"dupe","slug":"changed","description":"x","arch":["amd64"],"image":"ghcr.io/a/b"}"#,
        );
        assert!(matches!(
            inspect_repository(d.path()),
            Err(InspectError::InvalidManifest)
        ));
        write(
            d.path(),
            "bad/config.json",
            r#"{"name":"A","version":1.0,"slug":"timer","description":"x","arch":["amd64"],"image":"ghcr.io/a/b"}"#,
        );
        assert!(matches!(
            inspect_repository(d.path()),
            Err(InspectError::InvalidManifest)
        ));
        write(d.path(), "bad/config.json", &" ".repeat(70_000));
        assert!(matches!(
            inspect_repository(d.path()),
            Err(InspectError::TooLarge)
        ));
    }
}
