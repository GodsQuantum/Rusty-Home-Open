//! Strict offline subset of Home Assistant MQTT discovery, no broker or device IO.
//! See https://www.home-assistant.io/integrations/mqtt (2026-10-19).
use serde_json::{Map, Value};
use std::collections::HashSet;
const MAX_BYTES: usize = 65_536;
const MAX_COMPONENTS: usize = 32;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Sensor {
    pub platform: String,
    pub unique_id: String,
    pub state_topic: String,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Intent {
    Remove {
        id: String,
    },
    Migrate {
        id: String,
    },
    Preview {
        id: String,
        device_id: Option<String>,
        sensors: Vec<Sensor>,
    },
}
#[derive(Debug, PartialEq, Eq)]
pub enum Rejection {
    BadTopic,
    BadPayload,
    Unsupported,
    MissingIdentity,
    BadStateTopic,
    DuplicateIdentity,
    TooLarge,
    NotRetained,
}
fn unsafe_chars(s: &str) -> bool {
    s.chars()
        .any(|c| matches!(c, '+' | '#' | '\0' | '\r' | '\n'))
}
fn simple_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 100
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
}
fn parse_topic(topic: &str) -> Result<(&str, String, bool), Rejection> {
    if topic.len() > 512 || unsafe_chars(topic) {
        return Err(Rejection::BadTopic);
    }
    let parts: Vec<_> = topic.split('/').collect();
    if !(parts.len() == 4 || parts.len() == 5)
        || parts[0] != "homeassistant"
        || parts.last() != Some(&"config")
        || !parts[2..parts.len() - 1].iter().all(|s| simple_id(s))
    {
        return Err(Rejection::BadTopic);
    }
    let platform = parts[1];
    let device = platform == "device";
    if device && parts.len() != 4 {
        return Err(Rejection::BadTopic);
    }
    if !device && !matches!(platform, "sensor" | "binary_sensor") {
        return Err(Rejection::Unsupported);
    }
    Ok((platform, parts[2..parts.len() - 1].join("/"), device))
}
fn prop<'a>(
    m: &'a Map<String, Value>,
    long: &str,
    short: &str,
) -> Result<Option<&'a Value>, Rejection> {
    if m.contains_key(long) && m.contains_key(short) {
        return Err(Rejection::BadPayload);
    }
    Ok(m.get(long).or_else(|| m.get(short)))
}
fn required(value: Option<&Value>) -> Result<&str, Rejection> {
    let s = value
        .and_then(Value::as_str)
        .ok_or(Rejection::MissingIdentity)?;
    if s.is_empty() || s.len() > 256 || s.chars().any(char::is_control) {
        return Err(Rejection::MissingIdentity);
    }
    Ok(s)
}
fn state_topic(
    obj: &Map<String, Value>,
    shared: Option<&Map<String, Value>>,
) -> Result<String, Rejection> {
    let own = prop(obj, "state_topic", "stat_t")?;
    let inherited = shared
        .map(|m| prop(m, "state_topic", "stat_t"))
        .transpose()?
        .flatten();
    let s = required(own.or(inherited)).map_err(|_| Rejection::BadStateTopic)?;
    if s.len() > 512 || unsafe_chars(s) || s.starts_with('~') || s.ends_with('~') {
        // MQTT base-topic expansion is not yet implemented: refuse, do not guess.
        return Err(Rejection::BadStateTopic);
    }
    Ok(s.to_owned())
}
fn dev_id(obj: &Map<String, Value>) -> Result<String, Rejection> {
    let d = prop(obj, "device", "dev")?
        .and_then(Value::as_object)
        .ok_or(Rejection::MissingIdentity)?;
    let id = prop(d, "identifiers", "ids")?.ok_or(Rejection::MissingIdentity)?;
    let s = match id {
        Value::String(s) => s.as_str(),
        Value::Array(a) if a.len() == 1 => a[0].as_str().ok_or(Rejection::MissingIdentity)?,
        _ => return Err(Rejection::MissingIdentity),
    };
    if s.trim().is_empty() || s.len() > 256 || s.chars().any(char::is_control) {
        return Err(Rejection::MissingIdentity);
    }
    let _ = required(d.get("name"))?;
    Ok(s.to_owned())
}
fn sensor(
    m: &Map<String, Value>,
    shared: Option<&Map<String, Value>>,
    platform: &str,
) -> Result<Sensor, Rejection> {
    if !matches!(platform, "sensor" | "binary_sensor") {
        return Err(Rejection::Unsupported);
    }
    Ok(Sensor {
        platform: platform.to_owned(),
        unique_id: required(prop(m, "unique_id", "uniq_id")?)?.to_owned(),
        state_topic: state_topic(m, shared)?,
    })
}
/// This interprets metadata only; it DOES NOT install, subscribe or activate.
pub fn preview(topic: &str, payload: &[u8], retained: bool) -> Result<Intent, Rejection> {
    let (platform, id, device) = parse_topic(topic)?;
    if payload.len() > MAX_BYTES {
        return Err(Rejection::TooLarge);
    }
    if payload.is_empty() {
        if !retained {
            return Err(Rejection::NotRetained);
        }
        return Ok(Intent::Remove { id });
    }
    let payload: Value = serde_json::from_slice(payload).map_err(|_| Rejection::BadPayload)?;
    let obj = payload.as_object().ok_or(Rejection::BadPayload)?;
    if obj.len() == 1 && obj.get("migrate_discovery") == Some(&Value::Bool(true)) {
        return Ok(Intent::Migrate { id });
    }
    if obj.contains_key("migrate_discovery") {
        return Err(Rejection::BadPayload);
    }
    if device {
        let device_id = dev_id(obj)?;
        let origin = prop(obj, "origin", "o")?
            .and_then(Value::as_object)
            .ok_or(Rejection::MissingIdentity)?;
        let _ = required(origin.get("name"))?;
        let cmps = prop(obj, "components", "cmps")?
            .and_then(Value::as_object)
            .ok_or(Rejection::BadPayload)?;
        if cmps.is_empty() || cmps.len() > MAX_COMPONENTS {
            return Err(Rejection::BadPayload);
        }
        let mut ids = HashSet::new();
        let mut sensors = Vec::new();
        for (key, component) in cmps {
            if !simple_id(key) {
                return Err(Rejection::BadPayload);
            }
            let m = component.as_object().ok_or(Rejection::BadPayload)?;
            let kind = required(prop(m, "platform", "p")?)?;
            let val = sensor(m, Some(obj), kind)?;
            if !ids.insert(val.unique_id.clone()) {
                return Err(Rejection::DuplicateIdentity);
            }
            sensors.push(val);
        }
        Ok(Intent::Preview {
            id,
            device_id: Some(device_id),
            sensors,
        })
    } else {
        let value = sensor(obj, None, platform)?;
        let device_id = if prop(obj, "device", "dev")?.is_some() {
            Some(dev_id(obj)?)
        } else {
            None
        };
        Ok(Intent::Preview {
            id,
            device_id,
            sensors: vec![value],
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn official_single_sensor_shape() {
        let p=br#"{"dev":{"ids":"ea334450945afc","name":"Kitchen"},"o":{"name":"bla2mqtt"},"unique_id":"temp01ae_t","state_topic":"sensorBedroom/state"}"#;
        let Intent::Preview {
            id,
            device_id,
            sensors,
        } = preview("homeassistant/sensor/kitchen/temp/config", p, true).unwrap()
        else {
            panic!("not preview")
        };
        assert_eq!(id, "kitchen/temp");
        assert_eq!(device_id.as_deref(), Some("ea334450945afc"));
        assert_eq!(sensors[0].unique_id, "temp01ae_t");
    }
    #[test]
    fn official_device_sensors_shape() {
        let p=br#"{"dev":{"ids":"ea334450945afc","name":"Kitchen"},"o":{"name":"bla2mqtt"},"cmps":{"first":{"p":"sensor","unique_id":"temp01ae_t"},"second":{"p":"binary_sensor","unique_id":"motion01"}},"state_topic":"sensorBedroom/state"}"#;
        let Intent::Preview { sensors, .. } =
            preview("homeassistant/device/room123/config", p, true).unwrap()
        else {
            panic!("wrong intent")
        };
        assert_eq!(sensors.len(), 2);
        assert_eq!(sensors[1].state_topic, "sensorBedroom/state");
    }
    #[test]
    fn retained_removal_and_migration_never_activate() {
        assert_eq!(
            preview("homeassistant/sensor/test/config", b"", true),
            Ok(Intent::Remove { id: "test".into() })
        );
        assert_eq!(
            preview("homeassistant/sensor/test/config", b"", false),
            Err(Rejection::NotRetained)
        );
        assert_eq!(
            preview(
                "homeassistant/sensor/test/config",
                br#"{"migrate_discovery":true}"#,
                false
            ),
            Ok(Intent::Migrate { id: "test".into() })
        );
    }
    #[test]
    fn malformed_topics_and_unsupported_kinds_rejected() {
        for t in [
            "homeassistant/sensor/+/config",
            "homeassistant/sensor/name",
            "homeassistant/sensor/node/../config",
            "homeassistant/device/n/o/config",
            "wrong/sensor/id/config",
        ] {
            assert!(preview(t, b"", true).is_err(), "{t}");
        }
        assert_eq!(
            preview("homeassistant/light/kitchen/config", b"{}", true),
            Err(Rejection::Unsupported)
        );
    }
    #[test]
    fn dangerous_topics_and_conflicting_aliases_rejected() {
        for p in [
            r#"{"unique_id":"a","uniq_id":"b","state_topic":"sensor/a"}"#,
            r#"{"unique_id":"a","state_topic":"sensor/#"}"#,
            r#"{"unique_id":"a","state_topic":"~/abc"}"#,
            r#"{"unique_id":"a","state_topic":"sensor/a","migrate_discovery":false}"#,
            r#"[]"#,
        ] {
            assert!(
                preview("homeassistant/sensor/test/config", p.as_bytes(), true).is_err(),
                "{p}"
            );
        }
    }
    #[test]
    fn duplicate_device_component_or_missing_origin_rejected() {
        for p in [
            r#"{"dev":{"ids":"x","name":"Lab"},"cmps":{"a":{"p":"sensor","unique_id":"x","state_topic":"lab/a"}}}"#,
            r#"{"dev":{"ids":"x","name":"Lab"},"o":{"name":"lab"},"cmps":{"a":{"p":"sensor","unique_id":"x"},"b":{"p":"sensor","unique_id":"x"}},"state_topic":"lab/a"}"#,
            r#"{"dev":{"ids":"x","name":"Lab"},"o":{"name":"lab"},"cmps":{"a":{"p":"light","unique_id":"x","state_topic":"lab/a"}}}"#,
        ] {
            assert!(
                preview("homeassistant/device/lab/config", p.as_bytes(), true).is_err(),
                "{p}"
            );
        }
    }
    #[test]
    fn bounded_payload_and_preview_only() {
        assert_eq!(
            preview(
                "homeassistant/sensor/test/config",
                &vec![b'x'; MAX_BYTES + 1],
                false
            ),
            Err(Rejection::TooLarge)
        );
        assert!(matches!(
            preview(
                "homeassistant/sensor/test/config",
                br#"{"unique_id":"ok","state_topic":"lab/ok"}"#,
                false
            ),
            Ok(Intent::Preview { .. })
        ));
    }
}
