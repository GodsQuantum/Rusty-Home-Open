//! LAB ONLY: real MQTT 3.1.1 transport over an isolated, broker-local
//! namespace. No command topics, no credentials, no production MQTT access.
use crate::mqtt_discovery::{self, Intent};
use crate::mqtt_hub::{DiscoveryDriver, DiscoveryEvent};
use anyhow::{Result, bail};
use moli_core::{InstanceId, PointId, Value};
use moli_runtime::{Hub, HubOptions, spawn_driver};
use rumqttc::{AsyncClient, Broker, Event, Incoming, MqttOptions, PublishOptions, QoS};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

#[derive(Serialize, Debug, Default)]
pub struct Report {
    pub received: usize,
    /// Only real MQTT state publications, not discovery configurations.
    pub state_packets: usize,
    pub moli_states: usize,
    pub moli_binary_true: usize,
    pub previews: usize,
    pub removes: usize,
    pub migrations: usize,
    pub rejected: usize,
    pub retained: usize,
    pub registry_entries: usize,
    pub moli_registered_devices: usize,
    pub moli_native_adapter: bool,
    pub mqtt_broker_isolated: bool,
    pub read_only: bool,
    pub production_ready: bool,
}
/// Fail closed if invoked from isolated Linux lab/host or a Docker network with interfaces.
pub(crate) fn require_network_none() -> Result<()> {
    // Docker may expose pseudo sysfs entries even under network=none.
    // /proc/self/net/dev is scoped to the actual process network namespace.
    let interfaces = fs::read_to_string("/proc/self/net/dev")?;
    let names = interfaces
        .lines()
        .skip(2)
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.split(':').next().unwrap_or("").trim())
        .collect::<Vec<_>>();
    if names.as_slice() != ["lo"] {
        bail!("MQTT lab requires an isolated namespace with only loopback")
    }
    let routes = fs::read_to_string("/proc/net/route")?;
    if routes.lines().skip(1).any(|line| !line.trim().is_empty()) {
        bail!("MQTT lab forbids any non-loopback route")
    }
    Ok(())
}
fn options(identity: &str) -> MqttOptions {
    let mut opts = MqttOptions::new(identity, Broker::tcp("127.0.0.1", 1883));
    opts.set_keep_alive(5);
    opts.set_clean_session(true);
    opts
}
fn handle(
    report: &mut Report,
    reg: &mut HashMap<String, usize>,
    topic: &str,
    payload: &[u8],
    retain: bool,
) {
    report.received += 1;
    if retain {
        report.retained += 1;
    }
    match mqtt_discovery::preview(topic, payload, retain) {
        Ok(Intent::Preview { id, sensors, .. }) => {
            if let Some(existing) = reg.get(&id)
                && *existing != sensors.len()
            {
                report.rejected += 1;
                return;
            }
            reg.insert(id, sensors.len());
            report.previews += 1;
        }
        Ok(Intent::Remove { id }) => {
            reg.remove(&id);
            report.removes += 1;
        }
        Ok(Intent::Migrate { id }) => {
            reg.remove(&id);
            report.migrations += 1;
        }
        Err(_) => report.rejected += 1,
    }
    report.registry_entries = reg.len();
}
pub async fn observe(expected: usize) -> Result<Report> {
    observe_with_ready(expected, false).await
}

/// Deterministic lab handshake for a separate publisher process. This only
/// confirms SUBACK, never completion or authority of the retained replay.
pub async fn observe_with_ready(expected: usize, announce_suback: bool) -> Result<Report> {
    observe_inner(expected, 0, announce_suback, false).await
}

/// Strict read-only state-topic lab, without a real home automation broker.
pub async fn observe_sensor_values(
    expected_configs: usize,
    expected_states: usize,
) -> Result<Report> {
    if expected_states == 0 || expected_states > 32 {
        bail!("state count must be within 1..32")
    }
    observe_inner(expected_configs, expected_states, false, true).await
}

async fn observe_inner(
    expected: usize,
    expected_states: usize,
    announce_suback: bool,
    announce_states: bool,
) -> Result<Report> {
    require_network_none()?;
    if expected == 0 || expected > 50 {
        bail!("expected count 1..50 required")
    }
    let (client, mut loopback) = AsyncClient::builder(options("hars-p2b-receiver"))
        .capacity(64)
        .build();
    client
        .subscribe("homeassistant/#", QoS::AtLeastOnce)
        .await?;
    let mut report = Report {
        mqtt_broker_isolated: true,
        read_only: true,
        ..Report::default()
    };
    let mut reg = HashMap::new();
    let mut subscribed: HashSet<String> = HashSet::new();
    let mut subacks = 0usize;
    let mut states_announced = false;
    let hub = Hub::new(HubOptions::default())?;
    let cancel = CancellationToken::new();
    let (tx, rx) = mpsc::channel(64);
    let worker = spawn_driver(
        &hub,
        InstanceId::from("hars-mqtt-lab"),
        Arc::new(DiscoveryDriver::new(rx)),
        cancel.clone(),
    );
    let deadline = tokio::time::Instant::now() + Duration::from_secs(12);
    while report.received < expected || report.state_packets < expected_states {
        let remain = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remain.is_zero() {
            bail!(
                "timed out: expected {expected} configs and {expected_states} states, got {} and {}",
                report.received,
                report.state_packets
            )
        }
        match tokio::time::timeout(remain, loopback.poll()).await {
            Ok(Ok(Event::Incoming(Incoming::SubAck(ack)))) => {
                if ack.return_codes.is_empty()
                    || ack
                        .return_codes
                        .contains(&rumqttc::SubscribeReasonCode::Failure)
                {
                    bail!("isolated broker refused an MQTT sensor subscription")
                }
                subacks += 1;
                if announce_suback && subacks == 1 {
                    use std::io::Write;
                    println!("LAB_MQTT_SUBSCRIBED");
                    std::io::stdout().flush()?;
                }
            }
            Ok(Ok(Event::Incoming(Incoming::Publish(p)))) => {
                if let Ok(topic) = std::str::from_utf8(p.topic.as_ref()) {
                    if topic.starts_with("homeassistant/") {
                        let rejected_before = report.rejected;
                        handle(&mut report, &mut reg, topic, p.payload.as_ref(), p.retain);
                        if report.rejected == rejected_before
                            && let Ok(intent) =
                                mqtt_discovery::preview(topic, p.payload.as_ref(), p.retain)
                        {
                            if let Intent::Preview { sensors, .. } = &intent
                                && (sensors.len() > 16 || reg.values().sum::<usize>() > 64)
                            {
                                bail!("too many synthetic MQTT sensors")
                            }
                            if expected_states > 0
                                && let Intent::Preview { sensors, .. } = &intent
                            {
                                for sensor in sensors {
                                    let state_topic = &sensor.state_topic;
                                    // Additional lab boundary: only our invented topics.
                                    if state_topic.starts_with("lab/readonly/")
                                        && state_topic.len() <= 128
                                        && state_topic.bytes().all(|b| {
                                            b.is_ascii_alphanumeric()
                                                || b == b'/'
                                                || b == b'_'
                                                || b == b'-'
                                        })
                                        && !subscribed.contains(state_topic)
                                    {
                                        if subscribed.len() >= 16 {
                                            bail!("too many synthetic MQTT state subscriptions")
                                        }
                                        subscribed.insert(state_topic.to_owned());
                                        client.subscribe(state_topic, QoS::AtLeastOnce).await?;
                                    }
                                }
                            }
                            tx.send(DiscoveryEvent::Config(intent)).await?;
                        }
                    } else if expected_states > 0 && subscribed.contains(topic) {
                        report.state_packets += 1;
                        if p.payload.len() <= 128 {
                            tx.send(DiscoveryEvent::SensorState {
                                topic: topic.to_owned(),
                                payload: p.payload.to_vec(),
                            })
                            .await?;
                        } else {
                            report.rejected += 1;
                        }
                    } else {
                        report.rejected += 1;
                    }
                } else {
                    report.received += 1;
                    report.rejected += 1;
                }
            }
            Ok(Ok(_)) => {}
            Ok(Err(e)) => bail!("isolated broker disconnected: {e}"),
            Err(_) => bail!("timed out while waiting for MQTT"),
        }
        if announce_states
            && !states_announced
            && report.received >= expected
            && !subscribed.is_empty()
            && subacks > subscribed.len()
        {
            use std::io::Write;
            println!("LAB_MQTT_STATES_READY");
            std::io::stdout().flush()?;
            states_announced = true;
        }
    }
    // MQTT event-loop must be polled for requests; do not await a disconnect
    // request that depends on the same blocked poll loop. Dropping stops lab IO.
    drop(client);
    let wanted: usize = reg.values().sum();
    for _ in 0..100 {
        let snapshot = hub.snapshot();
        let n = snapshot.devices.len();
        if snapshot
            .devices
            .iter()
            .any(|view| view.device.points.iter().any(|point| point.access.write))
        {
            bail!("synthetic MQTT discovery driver exposed a writeable point")
        }
        let mut values = 0;
        let mut binary_true = 0;
        for view in &snapshot.devices {
            if let Some(sample) = hub.state(&PointId::new(&view.device.id, "state")) {
                values += 1;
                if sample.value == Value::Bool(true) {
                    binary_true += 1;
                }
            }
        }
        if n == wanted && (expected_states == 0 || values == wanted) {
            report.moli_registered_devices = n;
            report.moli_states = values;
            report.moli_binary_true = binary_true;
            report.moli_native_adapter = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    cancel.cancel();
    drop(tx);
    let _ = tokio::time::timeout(Duration::from_secs(3), worker).await;
    if !report.moli_native_adapter {
        bail!("native Moli Hub failed to register expected {wanted} discovered synthetic sensors")
    }
    Ok(report)
}
pub async fn publish(topic: &str, payload: &[u8], retain: bool) -> Result<()> {
    require_network_none()?;
    if !(topic.starts_with("homeassistant/") || topic.starts_with("lab/readonly/"))
        || topic.len() > 512
        || topic.contains('+')
        || topic.contains('#')
        || payload.len() > 65_536
        || (topic.starts_with("lab/readonly/") && (payload.len() > 128 || retain))
    {
        bail!("unsafe MQTT test topic/payload")
    }
    let (client, mut loopback) = AsyncClient::builder(options("hars-p2b-publisher"))
        .capacity(32)
        .build();
    client
        .publish(
            topic,
            payload.to_vec(),
            PublishOptions::new(QoS::AtLeastOnce).retain(retain),
        )
        .await?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(6);
    let mut saw_ack = false;
    while tokio::time::Instant::now() < deadline {
        match tokio::time::timeout(Duration::from_secs(2), loopback.poll()).await {
            Ok(Ok(Event::Incoming(Incoming::PubAck(_)))) => {
                saw_ack = true;
                break;
            }
            Ok(Ok(_)) => {}
            Ok(Err(e)) => bail!("isolated publish error: {e}"),
            Err(_) => {}
        }
    }
    if !saw_ack {
        bail!("MQTT QoS1 publish never acknowledged")
    }
    drop(client);
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn in_memory_preview_remove_and_migration_never_commands_devices() {
        let mut report = Report::default();
        let mut reg = HashMap::new();
        let topic = "homeassistant/sensor/lab/config";
        let config = br#"{"unique_id":"lab_01","state_topic":"sandbox/lab/state"}"#;
        handle(&mut report, &mut reg, topic, config, true);
        assert_eq!(report.previews, 1);
        assert_eq!(reg.len(), 1);
        handle(
            &mut report,
            &mut reg,
            topic,
            br#"{"migrate_discovery":true}"#,
            true,
        );
        assert_eq!(report.migrations, 1);
        assert!(reg.is_empty());
        handle(&mut report, &mut reg, topic, b"", true);
        assert_eq!(report.removes, 1);
        assert_eq!(report.registry_entries, 0);
        handle(
            &mut report,
            &mut reg,
            "homeassistant/light/cmd/config",
            config,
            true,
        );
        assert_eq!(report.rejected, 1);
    }
}
