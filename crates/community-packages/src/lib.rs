//! Offline structural gate for community packages.
//! Does NOT download, install, execute or certify runtime compatibility.
//! HACS 2026.10 and planned Marketplace 2026.11 have different category support.
pub mod marketplace_transition;
use serde_json::Value;
use std::fs::{self, Metadata};
use std::path::{Component, Path, PathBuf};

const MAX_MANIFEST: u64 = 64 * 1024;
const MAX_PAYLOAD_FILES: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Store {
    HacsLegacy,
    Marketplace2026_11,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Integration,
    Dashboard,
    Theme,
    Template,
    PythonScript,
    AppDaemon,
    SupervisorApp,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evidence {
    pub kind: Kind,
    pub paths: Vec<String>,
    /// Structural inspection cannot authorize execution/install.
    pub runtime_compatible: bool,
    pub activation_authorized: bool,
    pub needs_python: bool,
}
#[derive(Debug)]
pub enum InspectError {
    UnsupportedKind,
    InvalidRepo,
    UnsafePayload,
    InvalidManifest,
    InvalidLayout,
    Io(std::io::Error),
}
impl From<std::io::Error> for InspectError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
fn ordinary_file(p: &Path) -> Result<Metadata, InspectError> {
    let m = fs::symlink_metadata(p)?;
    if !m.is_file() || m.file_type().is_symlink() {
        return Err(InspectError::UnsafePayload);
    }
    Ok(m)
}
fn read_json(p: &Path) -> Result<Value, InspectError> {
    let m = ordinary_file(p)?;
    if m.len() > MAX_MANIFEST {
        return Err(InspectError::InvalidManifest);
    }
    let data = fs::read(p)?;
    serde_json::from_slice(&data).map_err(|_| InspectError::InvalidManifest)
}
fn clean_component(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 120
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-' || c == b'.')
        && s != "."
        && s != ".."
        && !s.starts_with('.')
}
fn clean_repository_entry(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 120
        && s != "."
        && s != ".."
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-' || c == b'.')
}
fn safe_relative(s: &str) -> bool {
    let path = Path::new(s);
    if path.is_absolute() || s.contains('\\') || s.contains('\0') {
        return false;
    }
    path.components().all(|part| match part {
        Component::Normal(name) => name.to_str().is_some_and(clean_component),
        _ => false,
    }) && path.components().count() > 0
}
fn directory(path: &Path) -> Result<(), InspectError> {
    let m = fs::symlink_metadata(path)?;
    if !m.is_dir() || m.file_type().is_symlink() {
        return Err(InspectError::UnsafePayload);
    }
    Ok(())
}
fn validate_tree(root: &Path) -> Result<(), InspectError> {
    directory(root)?;
    let mut stack = vec![(root.to_path_buf(), 0usize)];
    let mut count = 0;
    while let Some((dir, level)) = stack.pop() {
        if level > 12 {
            return Err(InspectError::UnsafePayload);
        }
        for entry in fs::read_dir(&dir)? {
            let e = entry?;
            count += 1;
            if count > MAX_PAYLOAD_FILES {
                return Err(InspectError::UnsafePayload);
            }
            let f = e.file_type()?;
            if f.is_symlink() || (!f.is_file() && !f.is_dir()) {
                return Err(InspectError::UnsafePayload);
            }
            let name = e.file_name();
            let Some(name) = name.to_str() else {
                return Err(InspectError::UnsafePayload);
            };
            if !clean_repository_entry(name) {
                return Err(InspectError::UnsafePayload);
            }
            // A local clone's git database is NOT part of release content.
            if f.is_dir() && name == ".git" {
                continue;
            }
            if f.is_dir() {
                stack.push((e.path(), level + 1));
            }
        }
    }
    Ok(())
}
fn values(root: &Path, store: Store) -> Result<Value, InspectError> {
    let manifest = root.join("hacs.json");
    if manifest.exists() {
        let v = read_json(&manifest)?;
        if !v.is_object() {
            return Err(InspectError::InvalidManifest);
        }
        for k in ["filename", "persistent_directory"] {
            if let Some(s) = v.get(k) {
                let Some(s) = s.as_str() else {
                    return Err(InspectError::InvalidManifest);
                };
                if !safe_relative(s) {
                    return Err(InspectError::InvalidManifest);
                }
            }
        }
        if let Some(v) = v.get("content_in_root")
            && !v.is_boolean()
        {
            return Err(InspectError::InvalidManifest);
        }
        Ok(v)
    } else if store == Store::HacsLegacy {
        Ok(Value::Object(Default::default()))
    } else {
        Err(InspectError::InvalidManifest)
    }
}
fn relative(root: &Path, p: &Path) -> String {
    p.strip_prefix(root)
        .unwrap_or(p)
        .to_string_lossy()
        .replace('\\', "/")
}
fn files(dir: &Path, suffix: &str) -> Result<Vec<PathBuf>, InspectError> {
    directory(dir)?;
    let mut names = Vec::new();
    for e in fs::read_dir(dir)? {
        let e = e?;
        let name = e.file_name().to_string_lossy().into_owned();
        if !clean_repository_entry(&name) {
            return Err(InspectError::UnsafePayload);
        }
        if e.file_type()?.is_symlink() {
            return Err(InspectError::UnsafePayload);
        }
        if e.file_type()?.is_file() && name.ends_with(suffix) {
            names.push(e.path());
        }
    }
    names.sort();
    Ok(names)
}
/// Structural preflight only: creates no files and never executes extensions.
/// HACS/Marketplace support is versioned and fail-closed for unknown categories.
pub fn inspect(
    root: &Path,
    store: Store,
    kind: Kind,
    repo_name: &str,
) -> Result<Evidence, InspectError> {
    if !clean_component(repo_name) {
        return Err(InspectError::InvalidRepo);
    }
    if store == Store::Marketplace2026_11
        && matches!(
            kind,
            Kind::AppDaemon | Kind::PythonScript | Kind::SupervisorApp
        )
    {
        return Err(InspectError::UnsupportedKind);
    }
    if kind == Kind::SupervisorApp {
        return Err(InspectError::UnsupportedKind);
    }
    validate_tree(root)?;
    let opts = values(root, store)?;
    let content_in_root = opts.get("content_in_root").and_then(Value::as_bool) == Some(true);
    let target = match kind {
        Kind::Integration => {
            let integration = if content_in_root {
                root.to_path_buf()
            } else {
                let folder = root.join("custom_components");
                directory(&folder)?;
                let dirs = fs::read_dir(folder)?.collect::<Result<Vec<_>, _>>()?;
                if dirs.len() != 1 || !dirs[0].file_type()?.is_dir() {
                    return Err(InspectError::InvalidLayout);
                }
                dirs[0].path()
            };
            let manifest = read_json(&integration.join("manifest.json"))?;
            let domain = if content_in_root {
                manifest
                    .get("domain")
                    .and_then(Value::as_str)
                    .ok_or(InspectError::InvalidManifest)?
            } else {
                integration
                    .file_name()
                    .and_then(|v| v.to_str())
                    .ok_or(InspectError::InvalidLayout)?
            };
            if !clean_component(domain) {
                return Err(InspectError::InvalidLayout);
            }
            if manifest.get("domain").and_then(Value::as_str) != Some(domain) {
                return Err(InspectError::InvalidManifest);
            }
            for key in ["name", "version", "documentation", "issue_tracker"] {
                if manifest
                    .get(key)
                    .and_then(Value::as_str)
                    .is_none_or(str::is_empty)
                {
                    return Err(InspectError::InvalidManifest);
                }
            }
            integration.join("manifest.json")
        }
        Kind::Dashboard => {
            let file = opts
                .get("filename")
                .and_then(Value::as_str)
                .map(str::to_owned)
                .unwrap_or_else(|| format!("{repo_name}.js"));
            if !safe_relative(&file) || !file.ends_with(".js") {
                return Err(InspectError::InvalidManifest);
            }
            let folders = if content_in_root {
                vec![root.to_path_buf()]
            } else {
                vec![root.join("dist"), root.to_path_buf()]
            };
            folders
                .into_iter()
                .map(|dir| dir.join(&file))
                .find(|p| p.is_file())
                .ok_or(InspectError::InvalidLayout)?
        }
        Kind::Theme => {
            let dir = if content_in_root {
                root.to_path_buf()
            } else {
                root.join("themes")
            };
            files(&dir, ".yaml")?
                .into_iter()
                .next()
                .ok_or(InspectError::InvalidLayout)?
        }
        Kind::Template => {
            let name = opts
                .get("filename")
                .and_then(Value::as_str)
                .ok_or(InspectError::InvalidManifest)?;
            if !safe_relative(name) || !name.ends_with(".jinja") {
                return Err(InspectError::InvalidManifest);
            }
            let file = root.join(name);
            if ordinary_file(&file)?.len() > 5 * 1024 * 1024 {
                return Err(InspectError::InvalidLayout);
            }
            file
        }
        Kind::PythonScript | Kind::AppDaemon => {
            let dir = root.join(if kind == Kind::PythonScript {
                "python_scripts"
            } else {
                "apps"
            });
            files(
                &dir,
                if kind == Kind::PythonScript {
                    ".py"
                } else {
                    ".yaml"
                },
            )?
            .into_iter()
            .next()
            .ok_or(InspectError::InvalidLayout)?
        }
        Kind::SupervisorApp => return Err(InspectError::UnsupportedKind),
    };
    ordinary_file(&target)?;
    Ok(Evidence {
        kind,
        paths: vec![relative(root, &target)],
        runtime_compatible: false,
        activation_authorized: false,
        needs_python: matches!(
            kind,
            Kind::Integration | Kind::PythonScript | Kind::AppDaemon
        ),
    })
}

/// Parse a single published hacs/default catalog list (JSON string array of
/// owner/repository names). Offline only; no HTTP, installation or execution.
pub fn parse_catalog(bytes: &[u8]) -> Result<Vec<String>, InspectError> {
    if bytes.len() > 8 * 1024 * 1024 {
        return Err(InspectError::InvalidManifest);
    }
    let value: Value = serde_json::from_slice(bytes).map_err(|_| InspectError::InvalidManifest)?;
    let entries = value.as_array().ok_or(InspectError::InvalidManifest)?;
    if entries.len() > 50_000 {
        return Err(InspectError::InvalidManifest);
    }
    let mut seen = std::collections::HashSet::new();
    let mut repos = Vec::with_capacity(entries.len());
    for entry in entries {
        let name = entry.as_str().ok_or(InspectError::InvalidRepo)?;
        let mut parts = name.split('/');
        let owner = parts.next().unwrap_or("");
        let repo = parts.next().unwrap_or("");
        if parts.next().is_some() || !clean_component(owner) || !clean_component(repo) {
            return Err(InspectError::InvalidRepo);
        }
        if !seen.insert(name.to_ascii_lowercase()) {
            return Err(InspectError::InvalidRepo);
        }
        repos.push(name.to_owned());
    }
    Ok(repos)
}
#[cfg(test)]
mod catalog_tests {
    use super::*;
    #[test]
    fn future_unknown_community_repositories_accepted_as_names_only() {
        let v =
            parse_catalog(br#"["future-dev/new-device","other-person/lovelace-card"]"#).unwrap();
        assert_eq!(v.len(), 2);
        assert_eq!(v[0], "future-dev/new-device");
    }
    #[test]
    fn catalog_unsafe_entries_and_duplicates_blocked() {
        for content in [
            r#"["alice/ok","Alice/OK"]"#,
            r#"["alice/../../escape"]"#,
            r#"["alice/ok/path"]"#,
            r#"["/repo"]"#,
            r#"{"repo":"alice/ok"}"#,
            r#"[15]"#,
        ] {
            assert!(parse_catalog(content.as_bytes()).is_err(), "{content}");
        }
    }
    #[test]
    fn catalog_empty_list_is_legal_not_proof_of_availability() {
        assert!(parse_catalog(b"[]").unwrap().is_empty());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn dir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }
    fn write(root: &Path, p: &str, content: &str) {
        let file = root.join(p);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, content).unwrap();
    }
    fn meta(root: &Path) {
        write(
            root,
            "hacs.json",
            r#"{"name":"Test","content_in_root":false}"#,
        );
    }
    #[test]
    fn future_integration_has_structure_not_runtime_certification() {
        let d = dir();
        meta(d.path());
        write(
            d.path(),
            "custom_components/abc/manifest.json",
            r#"{"domain":"abc","name":"ABC","version":"1.0.0","documentation":"https://example.invalid","issue_tracker":"https://example.invalid/issues"}"#,
        );
        write(
            d.path(),
            "custom_components/abc/__init__.py",
            "async def async_setup(hass,config): return True",
        );
        let result = inspect(
            d.path(),
            Store::Marketplace2026_11,
            Kind::Integration,
            "abc",
        )
        .unwrap();
        assert!(result.needs_python);
        assert!(!result.runtime_compatible && !result.activation_authorized);
        assert_eq!(result.paths, vec!["custom_components/abc/manifest.json"]);
    }
    #[test]
    fn future_dashboard_theme_template() {
        let d = dir();
        meta(d.path());
        write(d.path(), "dist/lovelace-light.js", "export default {}");
        write(d.path(), "themes/dark.yaml", "dark: {}");
        write(d.path(), "button.jinja", "{{ value }}");
        assert!(
            inspect(
                d.path(),
                Store::Marketplace2026_11,
                Kind::Dashboard,
                "lovelace-light"
            )
            .is_ok()
        );
        assert!(inspect(d.path(), Store::Marketplace2026_11, Kind::Theme, "anything").is_ok());
        assert!(
            inspect(
                d.path(),
                Store::Marketplace2026_11,
                Kind::Template,
                "anything"
            )
            .is_err()
        );
        write(d.path(), "hacs.json", r#"{"filename":"button.jinja"}"#);
        assert!(
            inspect(
                d.path(),
                Store::Marketplace2026_11,
                Kind::Template,
                "anything"
            )
            .is_ok()
        );
    }
    #[test]
    fn old_types_refused_in_new_marketplace() {
        let d = dir();
        meta(d.path());
        write(d.path(), "python_scripts/ex.py", "print('test')");
        assert!(inspect(d.path(), Store::HacsLegacy, Kind::PythonScript, "abc").is_ok());
        assert!(matches!(
            inspect(
                d.path(),
                Store::Marketplace2026_11,
                Kind::PythonScript,
                "abc"
            ),
            Err(InspectError::UnsupportedKind)
        ));
        assert!(matches!(
            inspect(d.path(), Store::Marketplace2026_11, Kind::AppDaemon, "abc"),
            Err(InspectError::UnsupportedKind)
        ));
        assert!(matches!(
            inspect(d.path(), Store::HacsLegacy, Kind::SupervisorApp, "abc"),
            Err(InspectError::UnsupportedKind)
        ));
    }
    #[test]
    fn normal_hidden_repository_metadata_is_not_executable_payload() {
        let d = dir();
        meta(d.path());
        write(d.path(), ".github/workflows/check.yml", "name: inert");
        write(d.path(), ".gitignore", "target/");
        write(d.path(), ".git/HEAD", "ref: refs/heads/main");
        write(d.path(), "themes/quiet.yaml", "quiet: {}");
        let result = inspect(d.path(), Store::Marketplace2026_11, Kind::Theme, "quiet");
        assert!(
            result.is_ok(),
            "safe metadata was erroneously rejected: {result:?}"
        );
        assert!(!result.unwrap().activation_authorized);
    }
    #[test]
    fn malicious_manifests_and_symlinks_denied() {
        let d = dir();
        meta(d.path());
        write(d.path(), "themes/my.yaml", "safe: true");
        write(d.path(), "hacs.json", r#"{"filename":"../outside.js"}"#);
        assert!(inspect(d.path(), Store::Marketplace2026_11, Kind::Theme, "repo").is_err());
        meta(d.path());
        std::os::unix::fs::symlink("my.yaml", d.path().join("themes/link.yaml")).unwrap();
        assert!(matches!(
            inspect(d.path(), Store::Marketplace2026_11, Kind::Theme, "repo"),
            Err(InspectError::UnsafePayload)
        ));
    }
    #[test]
    fn missing_manifest_or_duplicate_integration_denied() {
        let d = dir();
        write(
            d.path(),
            "custom_components/abc/manifest.json",
            r#"{"domain":"abc"}"#,
        );
        assert!(
            inspect(
                d.path(),
                Store::Marketplace2026_11,
                Kind::Integration,
                "abc"
            )
            .is_err()
        );
        meta(d.path());
        assert!(
            inspect(
                d.path(),
                Store::Marketplace2026_11,
                Kind::Integration,
                "abc"
            )
            .is_err()
        );
        write(
            d.path(),
            "custom_components/def/manifest.json",
            r#"{"domain":"def"}"#,
        );
        assert!(matches!(
            inspect(
                d.path(),
                Store::Marketplace2026_11,
                Kind::Integration,
                "abc"
            ),
            Err(InspectError::InvalidLayout)
        ));
    }
    #[test]
    fn content_in_root_future_custom_integration() {
        let d = dir();
        write(d.path(), "hacs.json", r#"{"content_in_root":true}"#);
        write(
            d.path(),
            "manifest.json",
            r#"{"domain":"future_device","name":"Future Device","version":"1.0.0","documentation":"https://example.invalid","issue_tracker":"https://example.invalid/issues"}"#,
        );
        write(
            d.path(),
            "__init__.py",
            "async def async_setup(hass,config): return True",
        );
        let evidence = inspect(
            d.path(),
            Store::Marketplace2026_11,
            Kind::Integration,
            "other-repository",
        )
        .unwrap();
        assert_eq!(evidence.paths, vec!["manifest.json"]);
        assert!(evidence.needs_python);
        assert!(!evidence.runtime_compatible && !evidence.activation_authorized);
    }
    #[test]
    fn future_unseen_repos_not_claimed_compatible() {
        let d = dir();
        meta(d.path());
        assert!(
            inspect(
                d.path(),
                Store::Marketplace2026_11,
                Kind::Integration,
                "future-integration"
            )
            .is_err()
        );
        assert!(
            inspect(
                d.path(),
                Store::Marketplace2026_11,
                Kind::Dashboard,
                "future-card"
            )
            .is_err()
        );
        assert!(
            inspect(
                d.path(),
                Store::Marketplace2026_11,
                Kind::Template,
                "future-template"
            )
            .is_err()
        );
        assert!(
            inspect(
                d.path(),
                Store::Marketplace2026_11,
                Kind::Theme,
                "future-theme"
            )
            .is_err()
        );
    }
}
