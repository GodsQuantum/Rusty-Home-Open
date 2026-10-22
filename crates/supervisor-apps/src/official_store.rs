//! Read-only native Rust *client model* of official Supervisor Store API.
//! Official Python Supervisor remains the sole owner of repository/app lifecycle.
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ApiVersion {
    V1,
    V2,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Repo {
    pub slug: String,
    pub name: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct App {
    pub slug: String,
    pub repository: String,
    pub name: String,
    pub available: bool,
    pub installed: bool,
    pub update_available: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Catalog {
    pub api_version: ApiVersion,
    pub repositories: Vec<Repo>,
    pub apps: Vec<App>,
    pub official_supervisor_owner: bool,
    pub activation_authorized: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ContractError {
    InvalidPayload,
    InvalidVersion,
    UnsupportedPayload,
    TooLarge,
    Duplicate,
}
/// Parse a bounded, *untrusted* official Supervisor /store V1 or V2 response.
/// Network authorization and signed host identity must be verified by a future
/// caller; parsing a fixture alone does not establish Supervisor authenticity.
pub fn parse_store_response(bytes: &[u8]) -> Result<Catalog, ContractError> {
    use serde_json::Value;
    use std::collections::HashSet;
    if bytes.len() > 8 * 1024 * 1024 {
        return Err(ContractError::TooLarge);
    }
    // Reuse the strict parser from our existing offline admission crate:
    // serde_json::Value by itself silently accepts duplicate object keys.
    let envelope = serde_json::from_slice::<super::NoDuplicates>(bytes)
        .map_err(|_| ContractError::InvalidPayload)?
        .0;
    let top = envelope.as_object().ok_or(ContractError::InvalidPayload)?;
    let data = if let Some(content) = top.get("data") {
        if top.get("result").and_then(Value::as_str) != Some("ok") {
            return Err(ContractError::InvalidPayload);
        }
        content.as_object().ok_or(ContractError::InvalidPayload)?
    } else {
        if top.contains_key("result") {
            return Err(ContractError::InvalidPayload);
        }
        top
    };
    let api_version = match (data.contains_key("apps"), data.contains_key("addons")) {
        (true, false) => ApiVersion::V2,
        (false, true) => ApiVersion::V1,
        _ => return Err(ContractError::InvalidVersion),
    };
    let key = match api_version {
        ApiVersion::V1 => "addons",
        ApiVersion::V2 => "apps",
    };
    let apps_json = data
        .get(key)
        .and_then(Value::as_array)
        .ok_or(ContractError::InvalidPayload)?;
    let repos_json = data
        .get("repositories")
        .and_then(Value::as_array)
        .ok_or(ContractError::InvalidPayload)?;
    if apps_json.len() > 50_000 || repos_json.len() > 2048 {
        return Err(ContractError::TooLarge);
    }
    fn ident(s: &str) -> bool {
        !s.is_empty()
            && s.len() <= 120
            && s.bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
    }
    fn safe_name(s: &str) -> bool {
        !s.is_empty() && s.len() <= 256 && !s.chars().any(char::is_control)
    }
    fn string_field<'a>(
        object: &'a serde_json::Map<String, Value>,
        field: &str,
        verify: fn(&str) -> bool,
    ) -> Result<&'a str, ContractError> {
        let value = object
            .get(field)
            .and_then(Value::as_str)
            .ok_or(ContractError::InvalidPayload)?;
        if !verify(value) {
            return Err(ContractError::InvalidPayload);
        }
        Ok(value)
    }
    let mut repos = Vec::with_capacity(repos_json.len());
    let mut seen_repos = HashSet::new();
    for item in repos_json {
        let object = item.as_object().ok_or(ContractError::InvalidPayload)?;
        let slug = string_field(object, "slug", ident)?.to_string();
        if !seen_repos.insert(slug.to_ascii_lowercase()) {
            return Err(ContractError::Duplicate);
        }
        repos.push(Repo {
            slug,
            name: string_field(object, "name", safe_name)?.to_string(),
        });
    }
    let mut apps = Vec::with_capacity(apps_json.len());
    let mut seen_apps = HashSet::new();
    for item in apps_json {
        let object = item.as_object().ok_or(ContractError::InvalidPayload)?;
        let slug = string_field(object, "slug", ident)?.to_string();
        if !seen_apps.insert(slug.to_ascii_lowercase()) {
            return Err(ContractError::Duplicate);
        }
        let bool_field = |key: &str| -> Result<bool, ContractError> {
            object
                .get(key)
                .and_then(Value::as_bool)
                .ok_or(ContractError::InvalidPayload)
        };
        apps.push(App {
            slug,
            repository: string_field(object, "repository", ident)?.to_string(),
            name: string_field(object, "name", safe_name)?.to_string(),
            available: bool_field("available")?,
            installed: bool_field("installed")?,
            update_available: bool_field("update_available")?,
        });
    }
    Ok(Catalog {
        api_version,
        repositories: repos,
        apps,
        official_supervisor_owner: true,
        activation_authorized: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    const V1: &str = r#"{"result":"ok","data":{"addons":[{"slug":"core_mosquitto","repository":"core","name":"Mosquitto","available":true,"installed":true,"update_available":false,"version_latest":"6.6.0"},{"slug":"a8b1c2d3_future","repository":"a8b1c2d3","name":"Unknown Future App","available":true,"installed":false,"update_available":true}],"repositories":[{"slug":"core","name":"Official","source":"https://github.com/home-assistant/addons"},{"slug":"a8b1c2d3","name":"Third party","source":"https://example.invalid/user:secret@repo"}]}}"#;
    const V2: &str = r#"{"result":"ok","data":{"apps":[{"slug":"core_mosquitto","repository":"core","name":"Mosquitto","available":true,"installed":true,"update_available":false,"version_latest":"6.6.0"},{"slug":"a8b1c2d3_future","repository":"a8b1c2d3","name":"Unknown Future App","available":true,"installed":false,"update_available":true}],"repositories":[{"slug":"core","name":"Official"},{"slug":"a8b1c2d3","name":"Third party"}]}}"#;
    #[test]
    fn accepts_upstream_supervisor_v1_addons_and_v2_apps_with_same_entries() {
        let legacy = parse_store_response(V1.as_bytes()).unwrap();
        let modern = parse_store_response(V2.as_bytes()).unwrap();
        assert_eq!(legacy.api_version, ApiVersion::V1);
        assert_eq!(modern.api_version, ApiVersion::V2);
        assert_eq!(legacy.apps, modern.apps);
        assert_eq!(legacy.repositories, modern.repositories);
        assert_eq!(modern.apps[1].slug, "a8b1c2d3_future");
        assert!(modern.official_supervisor_owner && !modern.activation_authorized);
    }
    #[test]
    fn allows_direct_payload_and_denies_negative_supervisor_response() {
        let payload = br#"{"apps":[],"repositories":[]}"#;
        assert_eq!(
            parse_store_response(payload).unwrap().api_version,
            ApiVersion::V2
        );
        assert!(matches!(
            parse_store_response(br#"{"result":"error","data":{"apps":[],"repositories":[]}}"#),
            Err(ContractError::InvalidPayload)
        ));
    }
    #[test]
    fn fails_closed_on_mixed_unversioned_or_empty_store() {
        for bad in [
            br#"{"apps":[],"addons":[],"repositories":[]}"#.as_slice(),
            br#"{"repositories":[]}"#,
            br#"{"apps":[]}"#,
            br#"{"addons":{},"repositories":[]}"#,
            br#"{"data":{"apps":[],"repositories":[]}}"#,
            br#"[]"#,
        ] {
            assert!(parse_store_response(bad).is_err());
        }
    }
    #[test]
    fn rejects_duplicate_or_invalid_app_identifiers() {
        for bad in [
            V2.replace("a8b1c2d3_future", "core_mosquitto"),
            V2.replace("a8b1c2d3_future", "../../dangerous"),
            V2.replace("a8b1c2d3_future", ""),
            V2.replace("\"available\":true", "\"available\":\"true\""),
            V2.replace("\"repository\":\"core\"", "\"repository\":true"),
        ] {
            assert!(parse_store_response(bad.as_bytes()).is_err(), "{bad}");
        }
    }
    #[test]
    fn does_not_copy_arbitrary_upstream_urls_or_secrets_to_ui() {
        let m = parse_store_response(V1.as_bytes()).unwrap();
        let exported = serde_json::to_string(&m).unwrap();
        assert!(!exported.contains("secret"));
        assert!(!exported.contains("https://"));
        assert!(!m.activation_authorized);
    }
    #[test]
    fn rejects_oversize_or_malformed_json_and_duplicate_keys() {
        let data = vec![b'x'; 8 * 1024 * 1024 + 1];
        assert_eq!(parse_store_response(&data), Err(ContractError::TooLarge));
        for bad in [
            br#"not json"#.as_slice(),
            br#"{"apps":[],"apps":[{}],"repositories":[]}"#,
            br#"{"apps":[{"slug":"x","slug":"y","repository":"core","name":"X","available":true,"installed":false}],"repositories":[]}"#,
        ] {assert!(parse_store_response(bad).is_err());}
    }
}
