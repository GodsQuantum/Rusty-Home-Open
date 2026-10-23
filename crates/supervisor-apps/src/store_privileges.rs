//! Strict, value-free and vendor-neutral Store privileges review.
//! This is a read-only *security preflight*, NOT a permission grant or
//! promise that an App is safe. Unknown/missing metadata is never zero-risk.
use crate::app_detail::{InfoError, store_detail_path, valid_slug};
use crate::official_store::ApiVersion;
use serde::Serialize;
use serde_json::Value;

const PRIVILEGES: [&str; 9] = [
    "auth_api",
    "docker_api",
    "full_access",
    "hassio_api",
    "homeassistant_api",
    "host_network",
    "host_pid",
    "ingress",
    "protected",
];
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskReview {
    Elevated,
    Unknown,
    Inspect,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AppPrivileges {
    pub slug: String,
    pub security_rating: Option<u8>,
    /// Only trusted, fixed permission names, never raw vendor fields or values.
    pub granted: Vec<String>,
    pub unknown: Vec<String>,
    pub apparmor: Option<String>,
    pub role: Option<String>,
    pub review: RiskReview,
    pub read_only: bool,
    pub activation_authorized: bool,
}
pub fn privilege_path(version: ApiVersion, slug: &str) -> Result<String, InfoError> {
    store_detail_path(version, slug)
}
fn opt_boolean(
    data: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<Option<bool>, InfoError> {
    match data.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(v)) => Ok(Some(*v)),
        _ => Err(InfoError::InvalidPayload),
    }
}
fn classify_string(
    data: &serde_json::Map<String, Value>,
    key: &str,
    allowed: &[&str],
) -> Result<Option<String>, InfoError> {
    match data.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(v)) => {
            if v.len() > 128 || v.chars().any(char::is_control) {
                return Err(InfoError::InvalidPayload);
            }
            if allowed.contains(&v.as_str()) {
                Ok(Some(v.clone()))
            } else {
                Ok(Some("unrecognized".to_owned()))
            }
        }
        _ => Err(InfoError::InvalidPayload),
    }
}
pub fn parse_store_privileges(raw: &[u8], slug: &str) -> Result<AppPrivileges, InfoError> {
    if !valid_slug(slug) {
        return Err(InfoError::UnsafeSlug);
    }
    if raw.len() > 512 * 1024 {
        return Err(InfoError::TooLarge);
    }
    let root = serde_json::from_slice::<crate::NoDuplicates>(raw)
        .map_err(|_| InfoError::InvalidPayload)?
        .0;
    if root.get("result").and_then(Value::as_str) != Some("ok") {
        return Err(InfoError::InvalidPayload);
    }
    let data = root
        .get("data")
        .and_then(Value::as_object)
        .ok_or(InfoError::InvalidPayload)?;
    if data.get("slug").and_then(Value::as_str) != Some(slug) {
        return Err(InfoError::MismatchedSlug);
    }
    let security_rating = match data.get("rating") {
        None | Some(Value::Null) => None,
        Some(Value::Number(n)) => {
            let x = n.as_u64().ok_or(InfoError::InvalidPayload)?;
            if !(1..=8).contains(&x) {
                return Err(InfoError::InvalidPayload);
            }
            Some(x as u8)
        }
        _ => return Err(InfoError::InvalidPayload),
    };
    let mut granted = Vec::new();
    let mut unknown = Vec::new();
    for field in PRIVILEGES {
        match opt_boolean(data, field)? {
            Some(true) => granted.push(field.to_owned()),
            Some(false) => {}
            None => unknown.push(field.to_owned()),
        }
    }
    let apparmor = classify_string(
        data,
        "apparmor",
        &["disable", "disabled", "default", "profile"],
    )?;
    let role = classify_string(
        data,
        "hassio_role",
        &["default", "homeassistant", "manager", "admin"],
    )?;
    let privileged = granted.iter().any(|x| x != "ingress" && x != "protected")
        || matches!(security_rating, Some(1..=3))
        || matches!(apparmor.as_deref(), Some("disable" | "disabled"))
        || matches!(role.as_deref(), Some("manager" | "admin"));
    let incomplete = security_rating.is_none()
        || !unknown.is_empty()
        || apparmor.is_none()
        || role.is_none()
        || apparmor.as_deref() == Some("unrecognized")
        || role.as_deref() == Some("unrecognized");
    // Even an empty-list review is NOT an installation permit.
    let review = if privileged {
        RiskReview::Elevated
    } else if incomplete {
        RiskReview::Unknown
    } else {
        RiskReview::Inspect
    };
    Ok(AppPrivileges {
        slug: slug.to_owned(),
        security_rating,
        granted,
        unknown,
        apparmor,
        role,
        review,
        read_only: true,
        activation_authorized: false,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn wire(slug: &str, fields: Value) -> Vec<u8> {
        let mut d = fields.as_object().unwrap().clone();
        d.insert("slug".into(), json!(slug));
        d.insert(
            "options".into(),
            json!({"access_token":"DO_NOT_EXPOSE_PRIVATE"}),
        );
        d.insert("schema".into(), json!({"password":"password"}));
        d.insert("devices".into(), json!(["/dev/SENSITIVE_DEVICE"]));
        json!({"result":"ok","data":d}).to_string().into_bytes()
    }
    #[test]
    fn detects_vendor_risk_and_never_returns_config_secrets() {
        let body = wire(
            "unknown_vendor_522",
            json!({"rating":2,
      "docker_api":true,"full_access":true,"host_network":true,
      "hassio_api":true,"hassio_role":"manager","apparmor":"disable",
      "webui":"http://private-token.com","auto_update":true}),
        );
        let x = parse_store_privileges(&body, "unknown_vendor_522").unwrap();
        assert_eq!(x.review, RiskReview::Elevated);
        assert!(x.granted.contains(&"docker_api".into()));
        assert!(x.unknown.contains(&"auth_api".into()));
        assert!(!x.activation_authorized && x.read_only);
        let text = serde_json::to_string(&x).unwrap();
        for secret in [
            "DO_NOT_EXPOSE_PRIVATE",
            "/dev/SENSITIVE_DEVICE",
            "private-token.com",
            "options",
            "schema",
            "webui",
        ] {
            assert!(!text.contains(secret), "{secret}");
        }
    }
    #[test]
    fn unknown_metadata_is_not_mistaken_for_safe() {
        let x = parse_store_privileges(&wire("vendor", json!({})), "vendor").unwrap();
        assert_eq!(x.review, RiskReview::Unknown);
        assert_eq!(x.unknown.len(), 9);
        let complete = wire(
            "vendor",
            json!({"rating":8,"apparmor":"default","hassio_role":"default",
      "auth_api":false,"docker_api":false,"full_access":false,"hassio_api":false,
      "homeassistant_api":false,"host_network":false,"host_pid":false,"ingress":false,
      "protected":true}),
        );
        let x = parse_store_privileges(&complete, "vendor").unwrap();
        assert_eq!(x.review, RiskReview::Inspect);
        assert!(x.granted.contains(&"protected".to_owned()));
    }
    #[test]
    fn reject_invalid_and_duplicate_vendor_security_values() {
        for details in [
            json!({"rating":0}),
            json!({"rating":9}),
            json!({"rating":-1}),
            json!({"rating":"8"}),
            json!({"docker_api":"yes"}),
            json!({"host_pid":1}),
            json!({"hassio_role":7}),
            json!({"apparmor":"broken\nHeader"}),
            json!({"rating":1.3}),
        ] {
            assert!(parse_store_privileges(&wire("vendor", details), "vendor").is_err());
        }
        assert!(
            parse_store_privileges(
                br#"{"result":"ok","data":{"slug":"vendor","docker_api":false,"docker_api":true}}"#,
                "vendor"
            )
            .is_err()
        );
        assert_eq!(
            parse_store_privileges(&wire("wrong", json!({})), "vendor"),
            Err(InfoError::MismatchedSlug)
        );
        assert!(parse_store_privileges(&vec![b'a'; 524289], "vendor").is_err());
        for slug in ["../vendor", "a%2Fb", "x?redirect=http://evil", ""] {
            assert_eq!(
                privilege_path(ApiVersion::V1, slug),
                Err(InfoError::UnsafeSlug)
            );
        }
    }
}
