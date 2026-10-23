//! Typed, redacted details for any official Supervisor App, not a brand list.
use crate::official_store::ApiVersion;
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AppDetail {
    pub slug: String,
    pub name: String,
    pub repository: Option<String>,
    pub version: Option<String>,
    pub latest_version: Option<String>,
    pub state: Option<String>,
    pub available: Option<bool>,
    pub update_available: Option<bool>,
    pub system_managed: Option<bool>,
    pub ingress: Option<bool>,
    pub official_supervisor_owner: bool,
    pub activation_authorized: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InfoError {
    UnsafeSlug,
    TooLarge,
    InvalidPayload,
    MismatchedSlug,
}

pub fn valid_slug(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 120
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
}
pub fn detail_path(version: ApiVersion, slug: &str) -> Result<String, InfoError> {
    if !valid_slug(slug) {
        return Err(InfoError::UnsafeSlug);
    }
    Ok(match version {
        ApiVersion::V1 => format!("/addons/{slug}/info"),
        ApiVersion::V2 => format!("/v2/apps/{slug}/info"),
    })
}
fn optional_str(
    data: &serde_json::Map<String, Value>,
    key: &str,
    max: usize,
) -> Result<Option<String>, InfoError> {
    let Some(value) = data.get(key) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let value = value.as_str().ok_or(InfoError::InvalidPayload)?;
    if value.len() > max || value.chars().any(char::is_control) {
        return Err(InfoError::InvalidPayload);
    }
    Ok(Some(value.to_owned()))
}
fn optional_bool(
    data: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<Option<bool>, InfoError> {
    let Some(value) = data.get(key) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    value.as_bool().map(Some).ok_or(InfoError::InvalidPayload)
}
/// Parse only safe public fields. Never copy options, schemas, ingress tokens,
/// host addresses, API credentials, or device paths from the untrusted source.
pub fn parse_app_detail(bytes: &[u8], expected_slug: &str) -> Result<AppDetail, InfoError> {
    if !valid_slug(expected_slug) {
        return Err(InfoError::UnsafeSlug);
    }
    if bytes.len() > 512 * 1024 {
        return Err(InfoError::TooLarge);
    }
    let root = serde_json::from_slice::<crate::NoDuplicates>(bytes)
        .map_err(|_| InfoError::InvalidPayload)?
        .0;
    if root.get("result").and_then(Value::as_str) != Some("ok") {
        return Err(InfoError::InvalidPayload);
    }
    let data = root
        .get("data")
        .and_then(Value::as_object)
        .ok_or(InfoError::InvalidPayload)?;
    let slug = data
        .get("slug")
        .and_then(Value::as_str)
        .ok_or(InfoError::InvalidPayload)?;
    if !valid_slug(slug) || slug != expected_slug {
        return Err(InfoError::MismatchedSlug);
    }
    let name = optional_str(data, "name", 256)?
        .filter(|s| !s.is_empty())
        .ok_or(InfoError::InvalidPayload)?;
    Ok(AppDetail {
        slug: slug.into(),
        name,
        repository: optional_str(data, "repository", 120)?,
        version: optional_str(data, "version", 120)?,
        latest_version: optional_str(data, "version_latest", 120)?,
        state: optional_str(data, "state", 120)?,
        available: optional_bool(data, "available")?,
        update_available: optional_bool(data, "update_available")?,
        system_managed: optional_bool(data, "system_managed")?,
        ingress: optional_bool(data, "ingress")?,
        official_supervisor_owner: true,
        activation_authorized: false,
    })
}
