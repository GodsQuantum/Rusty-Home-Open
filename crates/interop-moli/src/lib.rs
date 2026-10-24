//! Experimental bridge using actual upstream Moli OS types, not a rewrite.
//! Read-only projection; does not own HA's entity state or control devices.
//! Preserve original payloads: Moli typed Value only represents JSON scalars.

use moli_core::{DeviceId, InstanceId, PointId, Value as MoliValue};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HaEntity {
    pub entity_id: String,
    pub platform: String,
    pub config_entry_id: Option<String>,
    pub unique_id: Option<String>,
    pub state: Value,
    #[serde(default)]
    pub attributes: Map<String, Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MoliProjection {
    pub original: HaEntity,
    pub device_id: DeviceId,
    pub point_id: PointId,
    pub scalar: MoliValue,
    pub stable_identity: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ProjectionError {
    InvalidEntityId,
    InvalidPlatform,
}

fn domain(id: &str) -> Result<&str, ProjectionError> {
    let (domain, name) = id.split_once('.').ok_or(ProjectionError::InvalidEntityId)?;
    if domain.is_empty()
        || name.is_empty()
        || domain.bytes().any(|b| !b.is_ascii_lowercase() && b != b'_')
        || name.chars().any(char::is_control)
    {
        return Err(ProjectionError::InvalidEntityId);
    }
    Ok(domain)
}

/// Hash a structured, injective input tuple before upstream DeviceId.
/// Unencoded native ids "a/b" and "a_b" collide because upstream replaces
/// slashes with underscores. SHA-256 bounds PointId size even for long IDs.
/// The original tuple is retained for import provenance and verification;
/// a 256-bit digest has negligible but not mathematically zero collision risk.
pub fn project(e: &HaEntity) -> Result<MoliProjection, ProjectionError> {
    let dom = domain(&e.entity_id)?;
    if e.platform.is_empty() || e.platform.chars().any(char::is_control) {
        return Err(ProjectionError::InvalidPlatform);
    }
    let stable = e.config_entry_id.as_ref().is_some_and(|id| !id.is_empty())
        && e.unique_id.as_ref().is_some_and(|id| !id.is_empty());
    let key = if stable {
        serde_json::json!(["stable-v1", dom, e.platform, e.config_entry_id, e.unique_id])
    } else {
        serde_json::json!(["unstable-v1", e.entity_id, e.platform])
    };
    let bytes = serde_json::to_vec(&key).expect("known JSON string/array construction");
    let native = format!("sha256:{}", hex::encode(Sha256::digest(bytes)));
    let device_id = DeviceId::new(&InstanceId::from("ha-compat"), &native);
    let point_id = PointId::new(&device_id, "state");
    Ok(MoliProjection {
        original: e.clone(),
        device_id,
        point_id,
        scalar: MoliValue::from_json(&e.state),
        stable_identity: stable,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashSet;

    fn fixture(name: &str, id: &str) -> HaEntity {
        HaEntity {
            entity_id: name.to_owned(),
            platform: "mock".into(),
            config_entry_id: Some("synthetic-entry".into()),
            unique_id: Some(id.to_owned()),
            state: json!("21.5"),
            attributes: Map::from_iter([(
                "measurement".to_string(),
                json!({"unit":"C","values":[1,2]}),
            )]),
        }
    }

    #[test]
    fn uses_real_moli_id_and_point_types() {
        let p = project(&fixture("sensor.temperature", "abc")).unwrap();
        let (device, key) = p.point_id.split().unwrap();
        assert_eq!(device, p.device_id);
        assert_eq!(key, "state");
        assert_eq!(p.device_id.instance().as_str(), "ha-compat");
        assert!(p.stable_identity);
    }

    #[test]
    fn slash_underscore_dont_collide() {
        let slash = project(&fixture("sensor.test", "a/b")).unwrap();
        let underscore = project(&fixture("sensor.test", "a_b")).unwrap();
        assert_ne!(slash.device_id, underscore.device_id);
    }

    #[test]
    fn display_entity_rename_preserves_stable_id() {
        let mut a = fixture("light.livingroom", "native/123");
        let before = project(&a).unwrap();
        a.entity_id = "light.lounge".into();
        assert_eq!(before.point_id, project(&a).unwrap().point_id);
    }

    #[test]
    fn json_payload_retains_compound_structure() {
        let mut e = fixture("sensor.large", "json");
        e.state = json!({"nested":[1,true,null],"value":2});
        let result = project(&e).unwrap();
        assert_eq!(result.original.state, e.state);
        assert!(matches!(result.scalar, MoliValue::Text(_)));
        assert_eq!(result.original.attributes, e.attributes);
    }

    #[test]
    fn unstable_identity_is_flagged() {
        let mut a = fixture("sensor.temporary", "id");
        a.unique_id = None;
        let first = project(&a).unwrap();
        assert!(!first.stable_identity);
        a.entity_id = "sensor.renamed".into();
        assert_ne!(first.point_id, project(&a).unwrap().point_id);
    }

    #[test]
    fn long_unique_id_generates_bounded_native_point_id() {
        let e = fixture("sensor.long_id", &"z".repeat(10000));
        let p = project(&e).unwrap();
        assert!(p.point_id.as_str().len() < 256);
        assert!(p.stable_identity);
    }

    #[test]
    fn simulated_1232_entities_remain_distinct() {
        let mut ids = HashSet::new();
        for i in 0..1232 {
            let e = fixture(&format!("sensor.fixture_{i}"), &format!("device/{i}"));
            assert!(ids.insert(project(&e).unwrap().point_id));
        }
        assert_eq!(ids.len(), 1232);
    }

    #[test]
    fn invalid_ids_rejected() {
        let mut e = fixture("invalid", "id");
        assert_eq!(project(&e), Err(ProjectionError::InvalidEntityId));
        e.entity_id = "sensor.value".into();
        e.platform = "".into();
        assert_eq!(project(&e), Err(ProjectionError::InvalidPlatform));
    }
}

/// Opt-in private read-only HA entity registry import staging, no device writes.
pub mod registry_stage;
