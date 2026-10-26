//! Read-only Moli Hub adapter for prevalidated, synthetic MQTT discovery.
//! No entity state is guessed from a discovery config. No writeable points.
//! The upstream DriverCtx enforces instance ownership of DeviceIds.
use crate::mqtt_discovery::Intent;
use anyhow::Result;
use moli_core::{Access, Device, DeviceId, Kind, PointSpec, Semantic, Value};

/// Input already limited to an isolated, read-only MQTT test namespace.
/// This never carries actuator commands or synthetic state guesses.
pub enum DiscoveryEvent {
    Config(Intent),
    SensorState { topic: String, payload: Vec<u8> },
}

fn state_value(platform: &str, payload: &[u8]) -> Option<Value> {
    if payload.is_empty() || payload.len() > 128 {
        return None;
    }
    let raw = std::str::from_utf8(payload).ok()?;
    if raw.chars().any(char::is_control) {
        return None;
    }
    match platform {
        "sensor" => Some(Value::Text(raw.into())),
        "binary_sensor" => match raw {
            "ON" => Some(Value::Bool(true)),
            "OFF" => Some(Value::Bool(false)),
            _ => None,
        },
        _ => None,
    }
}
use moli_runtime::{BoxFuture, Driver, DriverCtx};
use std::collections::{HashMap, HashSet};
use tokio::sync::{Mutex, mpsc};

pub struct DiscoveryDriver {
    events: Mutex<mpsc::Receiver<DiscoveryEvent>>,
}
impl DiscoveryDriver {
    pub fn new(events: mpsc::Receiver<DiscoveryEvent>) -> Self {
        Self {
            events: Mutex::new(events),
        }
    }
}
fn safe_sensor_id(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= 100
        && text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}
impl Driver for DiscoveryDriver {
    fn kind(&self) -> &'static str {
        "hars-mqtt-discovery-lab"
    }
    fn run<'a>(&'a self, ctx: &'a mut DriverCtx) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            ctx.ready();
            let mut rx = self.events.lock().await;
            // Per-config registered devices with exact topics and type; never
            // bind an MQTT payload to a device that has not been discovered.
            let mut configs: HashMap<String, Vec<(DeviceId, String, String, String)>> =
                HashMap::new();
            let mut globally_known: HashSet<String> = HashSet::new();
            while let Some(msg) = rx.recv().await {
                match msg {
                    DiscoveryEvent::Config(Intent::Preview { id, sensors, .. }) => {
                        if sensors.iter().any(|s| !safe_sensor_id(&s.unique_id)) {
                            continue;
                        }
                        if configs.contains_key(&id) {
                            // Same config may update, but our LAB deliberately
                            // fails closed instead of replacing a device.
                            continue;
                        }
                        if sensors
                            .iter()
                            .any(|s| globally_known.contains(&s.unique_id))
                        {
                            continue;
                        }
                        let mut registered = Vec::new();
                        for sensor in sensors {
                            let device_id = ctx.device_id(&sensor.unique_id);
                            let kind = if sensor.platform == "binary_sensor" {
                                Kind::Binary
                            } else {
                                Kind::Text
                            };
                            ctx.upsert_device(Device {
                                id: device_id.clone(),
                                instance: ctx.instance().clone(),
                                native_name: sensor.unique_id.clone().into(),
                                manufacturer: None,
                                model: None,
                                description: Some("Synthetic MQTT discovery preview only".into()),
                                native_room: None,
                                members: vec![],
                                points: vec![PointSpec {
                                    key: "state".into(),
                                    label: "State".into(),
                                    kind,
                                    access: Access {
                                        read: true,
                                        write: false,
                                    },
                                    unit: None,
                                    semantic: Semantic::Other,
                                }],
                            });
                            globally_known.insert(sensor.unique_id.clone());
                            registered.push((
                                device_id,
                                sensor.state_topic,
                                sensor.platform,
                                sensor.unique_id,
                            ));
                        }
                        configs.insert(id, registered);
                    }
                    DiscoveryEvent::Config(Intent::Remove { id } | Intent::Migrate { id }) => {
                        if let Some(devices) = configs.remove(&id) {
                            for (device, _, _, unique_id) in devices {
                                // Release only IDs owned by this removed config.
                                // A fresh retained discovery can re-register them,
                                // but cannot inherit any of their previous state.
                                globally_known.remove(&unique_id);
                                ctx.remove_device(&device);
                            }
                        }
                    }
                    DiscoveryEvent::SensorState { topic, payload } => {
                        if !topic.starts_with("lab/readonly/") {
                            continue;
                        }
                        for sensors in configs.values() {
                            for (device, state_topic, platform, _) in sensors {
                                if state_topic == &topic
                                    && let Some(value) = state_value(platform, &payload)
                                {
                                    ctx.set_state(device, "state", value);
                                }
                            }
                        }
                    }
                }
            }
            Ok(())
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn strict_sensor_payloads_cannot_inject_untyped_or_control_values() {
        assert_eq!(state_value("binary_sensor", b"ON"), Some(Value::Bool(true)));
        assert_eq!(
            state_value("binary_sensor", b"OFF"),
            Some(Value::Bool(false))
        );
        assert_eq!(
            state_value("sensor", b"22.1"),
            Some(Value::Text("22.1".into()))
        );
        for bad in [b"".as_slice(), b"on", b"true", b"ON\n", b"\xff"] {
            assert_eq!(state_value("binary_sensor", bad), None);
        }
        assert_eq!(state_value("sensor", &[b'X'; 129]), None);
        assert_eq!(state_value("light", b"ON"), None);
    }
    #[test]
    fn no_path_or_ambiguous_id_in_synthetic_registry() {
        assert!(safe_sensor_id("temp_23"));
        assert!(!safe_sensor_id("../escape"));
        assert!(!safe_sensor_id("device/other"));
        assert!(!safe_sensor_id(""));
        assert!(!safe_sensor_id("éclairage"));
    }
}

#[cfg(test)]
mod p2j_tombstone_readd_tests {
    use super::*;
    use crate::mqtt_discovery::Sensor;
    use moli_core::{InstanceId, PointId, Value};
    use moli_runtime::{Hub, HubOptions, spawn_driver};
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::mpsc;
    use tokio_util::sync::CancellationToken;

    async fn wait_devices(hub: &Hub, count: usize) {
        for _ in 0..150 {
            if hub.snapshot().devices.len() == count {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("Moli never converged to {count} devices");
    }

    async fn wait_state(hub: &Hub, point: &PointId, expected: &str) {
        for _ in 0..150 {
            if hub.state(point).map(|s| s.value) == Some(Value::Text(expected.into())) {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("Moli never reached expected new state");
    }

    #[tokio::test]
    async fn live_tombstone_readd_same_id_clears_state_and_old_topic() {
        let hub = Hub::new(HubOptions::default()).unwrap();
        let cancel = CancellationToken::new();
        let (tx, rx) = mpsc::channel(16);
        let worker = spawn_driver(
            &hub,
            InstanceId::from("hars-p2j-isolated"),
            Arc::new(DiscoveryDriver::new(rx)),
            cancel.clone(),
        );
        let config = |topic: &str| Intent::Preview {
            id: "p2j_probe".into(),
            device_id: None,
            sensors: vec![Sensor {
                platform: "sensor".into(),
                unique_id: "lab_p2j_probe".into(),
                state_topic: topic.into(),
            }],
        };
        tx.send(DiscoveryEvent::Config(config("lab/readonly/old")))
            .await
            .unwrap();
        wait_devices(&hub, 1).await;
        let id = hub.snapshot().devices[0].device.id.clone();
        let point = PointId::new(&id, "state");
        tx.send(DiscoveryEvent::SensorState {
            topic: "lab/readonly/old".into(),
            payload: b"21.5".to_vec(),
        })
        .await
        .unwrap();
        wait_state(&hub, &point, "21.5").await;
        tx.send(DiscoveryEvent::Config(Intent::Remove {
            id: "p2j_probe".into(),
        }))
        .await
        .unwrap();
        wait_devices(&hub, 0).await;
        assert!(hub.state(&point).is_none(), "tombstone must clear state");
        tx.send(DiscoveryEvent::SensorState {
            topic: "lab/readonly/old".into(),
            payload: b"99".to_vec(),
        })
        .await
        .unwrap();
        tx.send(DiscoveryEvent::Config(config("lab/readonly/new")))
            .await
            .unwrap();
        wait_devices(&hub, 1).await;
        assert!(
            hub.state(&point).is_none(),
            "re-add must start without state"
        );
        tx.send(DiscoveryEvent::SensorState {
            topic: "lab/readonly/old".into(),
            payload: b"88".to_vec(),
        })
        .await
        .unwrap();
        tokio::time::sleep(Duration::from_millis(80)).await;
        assert!(
            hub.state(&point).is_none(),
            "old topic cannot update new identity"
        );
        tx.send(DiscoveryEvent::SensorState {
            topic: "lab/readonly/new".into(),
            payload: b"31.2".to_vec(),
        })
        .await
        .unwrap();
        wait_state(&hub, &point, "31.2").await;
        assert!(
            hub.snapshot().devices[0]
                .device
                .points
                .iter()
                .all(|p| !p.access.write)
        );
        cancel.cancel();
        drop(tx);
        tokio::time::timeout(Duration::from_secs(3), worker)
            .await
            .unwrap()
            .unwrap();
    }
}
