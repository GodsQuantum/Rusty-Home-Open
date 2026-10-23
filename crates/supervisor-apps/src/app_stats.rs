//! Read-only typed resource metrics from the official Supervisor installed-App API.
//! No credentials, App options, device identifiers or log contents are exposed.
use crate::app_detail::valid_slug;
use crate::official_store::ApiVersion;
use serde::Serialize;
use serde_json::{Map, Value};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AppStats {
    pub slug: String,
    pub cpu_percent: Option<f64>,
    pub memory_usage: u64,
    pub memory_limit: u64,
    pub memory_percent: Option<f64>,
    pub network_rx: Option<u64>,
    pub network_tx: Option<u64>,
    pub read_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatsError {
    UnsafeSlug,
    TooLarge,
    InvalidPayload,
}

pub fn stats_path(version: ApiVersion, slug: &str) -> Result<String, StatsError> {
    if !valid_slug(slug) {
        return Err(StatsError::UnsafeSlug);
    }
    Ok(match version {
        ApiVersion::V1 => format!("/addons/{slug}/stats"),
        ApiVersion::V2 => format!("/v2/apps/{slug}/stats"),
    })
}

fn optional_percentage(data: &Map<String, Value>, key: &str) -> Result<Option<f64>, StatsError> {
    match data.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => {
            let n = value.as_f64().ok_or(StatsError::InvalidPayload)?;
            if !n.is_finite() || !(0.0..=100_000.0).contains(&n) {
                return Err(StatsError::InvalidPayload);
            }
            Ok(Some(n))
        }
    }
}

fn optional_bytes(data: &Map<String, Value>, key: &str) -> Result<Option<u64>, StatsError> {
    match data.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value.as_u64().map(Some).ok_or(StatsError::InvalidPayload),
    }
}

/// Parse a current authenticated Supervisor response. Only numeric telemetry
/// fields are allowlisted, unknown and potentially sensitive fields discarded.
pub fn parse_app_stats(bytes: &[u8], slug: &str) -> Result<AppStats, StatsError> {
    if !valid_slug(slug) {
        return Err(StatsError::UnsafeSlug);
    }
    if bytes.len() > 64 * 1024 {
        return Err(StatsError::TooLarge);
    }
    let response = serde_json::from_slice::<crate::NoDuplicates>(bytes)
        .map_err(|_| StatsError::InvalidPayload)?
        .0;
    if response.get("result").and_then(Value::as_str) != Some("ok") {
        return Err(StatsError::InvalidPayload);
    }
    let data = response
        .get("data")
        .and_then(Value::as_object)
        .ok_or(StatsError::InvalidPayload)?;
    let memory_usage = data
        .get("memory_usage")
        .and_then(Value::as_u64)
        .ok_or(StatsError::InvalidPayload)?;
    let memory_limit = data
        .get("memory_limit")
        .and_then(Value::as_u64)
        .ok_or(StatsError::InvalidPayload)?;
    Ok(AppStats {
        slug: slug.to_owned(),
        cpu_percent: optional_percentage(data, "cpu_percent")?,
        memory_usage,
        memory_limit,
        memory_percent: optional_percentage(data, "memory_percent")?,
        network_rx: optional_bytes(data, "network_rx")?,
        network_tx: optional_bytes(data, "network_tx")?,
        read_only: true,
    })
}
