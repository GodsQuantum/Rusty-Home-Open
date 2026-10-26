//! P2h: one-topic, read-only MQTT 5 -> durable redb -> native Moli lab.
//! Deliberately not an authoritative retained snapshot or production adapter.
use crate::mqtt_discovery::{self, Intent};
use crate::mqtt_hub::{DiscoveryDriver, DiscoveryEvent};
use crate::mqtt_live::require_network_none;
use crate::mqtt_registry::DiscoveryRegistry;
use anyhow::{Result, bail};
use moli_core::{InstanceId, PointId, Value};
use moli_runtime::{Hub, HubOptions, spawn_driver};
use rumqttc5::{
    AsyncClient, Event, Incoming, MqttOptions, QoS, RetainForwardRule, SubscribeFilterInput,
    SubscribeReasonCode,
};
use serde::Serialize;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

const CONFIG: &str = "homeassistant/sensor/p2h_probe/config";
const STATE: &str = "lab/readonly/p2h";

#[derive(Debug, Default, Serialize)]
pub struct ReconcileReport {
    pub received: usize,
    pub retained_configs: usize,
    pub retained_tombstones: usize,
    pub same_id_readd_verified: bool,
    pub old_state_cleared_before_readd: bool,
    pub ignored_unretained_empty: usize,
    pub sensor_packets: usize,
    pub recovered_stale_on_open: usize,
    pub durable_entries: usize,
    pub moli_devices: usize,
    pub moli_states: usize,
    pub broker_snapshot_authoritative: bool,
    pub device_ownership_granted: bool,
    pub read_only: bool,
    pub production_ready: bool,
}

/// Only on an isolated loopback broker. Never rebuild the live Moli state
/// from recovered redb: it is permanently stale until each live packet proves
/// the configuration. A SUBACK does NOT delimit the retained replay.
pub async fn observe(root: &Path, expected: usize) -> Result<ReconcileReport> {
    observe_inner(root, expected, false).await
}

/// P2k: different threat scenario than P2i broker restart. One still-running
/// upstream Moli Hub must accept a second, retained discovery with the SAME
/// stable unique_id only after genuine broker tombstone and state deletion.
pub async fn observe_same_id_readd(root: &Path, expected: usize) -> Result<ReconcileReport> {
    if expected != 5 {
        bail!("P2k expects exact five live MQTT5 publications");
    }
    observe_inner(root, expected, true).await
}

async fn observe_inner(
    root: &Path,
    expected: usize,
    same_id_readd: bool,
) -> Result<ReconcileReport> {
    require_network_none()?;
    if root != Path::new("/state") || expected > 8 {
        bail!("P2h lab accepts /state private bind and at most 8 synthetic packets");
    }
    let registry = DiscoveryRegistry::open(root)?;
    let mut report = ReconcileReport {
        recovered_stale_on_open: registry.recover_unverified()?.len(),
        read_only: true,
        ..Default::default()
    };
    let hub = Hub::new(HubOptions::default())?;
    let cancel = CancellationToken::new();
    let (tx, rx) = mpsc::channel(16);
    let worker = spawn_driver(
        &hub,
        InstanceId::from("hars-mqtt-p2h-lab"),
        Arc::new(DiscoveryDriver::new(rx)),
        cancel.clone(),
    );
    let mut opts = MqttOptions::new("hars-mqtt5-registry-lab", ("127.0.0.1", 1883));
    opts.set_keep_alive(5);
    let (client, mut loopback) = AsyncClient::builder(opts).capacity(16).build();
    client
        .subscribe_many([
            SubscribeFilterInput::new(CONFIG, QoS::AtLeastOnce)
                .preserve_retain(true)
                .retain_forward_rule(RetainForwardRule::OnEverySubscribe),
            SubscribeFilterInput::new(STATE, QoS::AtLeastOnce)
                .preserve_retain(true)
                .retain_forward_rule(RetainForwardRule::Never),
        ])
        .await?;
    let mut suback = false;
    let mut active = false;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(12);
    let mut quiet_deadline: Option<tokio::time::Instant> = None;
    loop {
        let now = tokio::time::Instant::now();
        let until = if let Some(q) = quiet_deadline {
            q.min(deadline)
        } else {
            deadline
        };
        if now >= until {
            if expected == 0 && suback {
                break;
            }
            bail!("MQTT5 registry lab timed out waiting for expected packets");
        }
        match tokio::time::timeout(until - now, loopback.poll()).await {
            Ok(Ok(Event::Incoming(Incoming::SubAck(ack)))) => {
                if suback
                    || ack.return_codes.len() != 2
                    || ack
                        .return_codes
                        .iter()
                        .any(|c| !matches!(c, SubscribeReasonCode::Success(_)))
                {
                    bail!("invalid MQTT5 subscription acknowledgement");
                }
                suback = true;
                println!("LAB_MQTT5_REGISTRY_READY");
                std::io::stdout().flush()?;
                if expected == 0 {
                    quiet_deadline = Some(tokio::time::Instant::now() + Duration::from_millis(850));
                }
            }
            Ok(Ok(Event::Incoming(Incoming::Publish(p)))) => {
                if !suback {
                    bail!("PUBLISH before MQTT5 SUBACK");
                }
                let topic = std::str::from_utf8(p.topic.as_ref())?;
                if topic == CONFIG {
                    report.received += 1;
                    match (p.retain, p.payload.is_empty()) {
                        (true, false) => {
                            let intent = mqtt_discovery::preview(topic, &p.payload, true)
                                .map_err(|e| anyhow::anyhow!("invalid MQTT5 config: {e:?}"))?;
                            let Intent::Preview { ref sensors, .. } = intent else {
                                bail!("only sensor preview may become live");
                            };
                            if sensors.len() != 1
                                || sensors[0].unique_id != "lab_p2h_probe"
                                || sensors[0].state_topic != STATE
                                || active
                            {
                                bail!("unsafe or conflicting MQTT5 discovery");
                            }
                            // Atomic commit first, then in-memory projection.
                            registry.retain_validated(topic, &p.payload)?;
                            tx.send(DiscoveryEvent::Config(intent)).await?;
                            active = true;
                            report.retained_configs += 1;
                            if same_id_readd && report.retained_configs == 2 {
                                // Wait for the REAL upstream Moli worker to
                                // re-register the formerly deleted ID, while
                                // preserving absolutely NO previous state.
                                let mut clean = false;
                                for _ in 0..150 {
                                    let snapshot = hub.snapshot();
                                    if snapshot.devices.len() == 1 {
                                        let id = &snapshot.devices[0].device.id;
                                        if hub.state(&PointId::new(id, "state")).is_none() {
                                            clean = true;
                                            break;
                                        }
                                    }
                                    tokio::time::sleep(Duration::from_millis(10)).await;
                                }
                                if !clean {
                                    bail!("same-ID rediscovery either failed or inherited state");
                                }
                                report.old_state_cleared_before_readd = true;
                            }
                        }
                        (true, true) => {
                            let removed = registry.remove_retained(topic, true)?;
                            if active && !removed {
                                bail!("active config missing during tombstone");
                            }
                            tx.send(DiscoveryEvent::Config(Intent::Remove {
                                id: "p2h_probe".into(),
                            }))
                            .await?;
                            active = false;
                            report.retained_tombstones += 1;
                            if same_id_readd {
                                let mut cleared = false;
                                for _ in 0..150 {
                                    if hub.snapshot().devices.is_empty()
                                        && registry.recover_unverified()?.is_empty()
                                    {
                                        cleared = true;
                                        break;
                                    }
                                    tokio::time::sleep(Duration::from_millis(10)).await;
                                }
                                if !cleared {
                                    bail!("genuine tombstone did not clear native Hub and redb");
                                }
                            }
                        }
                        (false, true) => {
                            report.ignored_unretained_empty += 1;
                        }
                        (false, false) => bail!("nonretained discovery cannot be trusted"),
                    }
                } else if topic == STATE {
                    report.received += 1;
                    if !active || p.retain || p.payload.is_empty() || p.payload.len() > 128 {
                        bail!("unverified, retained, or unsafe MQTT5 sensor state");
                    }
                    tx.send(DiscoveryEvent::SensorState {
                        topic: topic.to_owned(),
                        payload: p.payload.to_vec(),
                    })
                    .await?;
                    report.sensor_packets += 1;
                    if same_id_readd && report.sensor_packets == 1 {
                        let mut initial = false;
                        for _ in 0..150 {
                            let snapshot = hub.snapshot();
                            if snapshot.devices.len() == 1 {
                                let id = &snapshot.devices[0].device.id;
                                if hub
                                    .state(&PointId::new(id, "state"))
                                    .is_some_and(|v| v.value == Value::Text("21.5".into()))
                                {
                                    initial = true;
                                    break;
                                }
                            }
                            tokio::time::sleep(Duration::from_millis(10)).await;
                        }
                        if !initial {
                            bail!("original MQTT5 sensor never reached native Moli");
                        }
                    }
                } else {
                    bail!("unexpected MQTT5 topic");
                }
                if expected > 0 && report.received == expected {
                    break;
                }
                if report.received > expected {
                    bail!("excess MQTT5 packets during lab verification");
                }
            }
            Ok(Ok(_)) => {}
            Ok(Err(err)) => bail!("MQTT5 broker connection failed: {err}"),
            Err(_) if expected == 0 && suback => break,
            Err(_) => bail!("MQTT5 poll deadline expired"),
        }
    }
    drop(client);
    report.durable_entries = registry.recover_unverified()?.len();
    let wanted = usize::from(active);
    let mut converged = false;
    for _ in 0..150 {
        let snapshot = hub.snapshot();
        let actual = snapshot.devices.len();
        let mut states = 0;
        for view in &snapshot.devices {
            if view.device.points.iter().any(|point| point.access.write) {
                bail!("Moli MQTT lab must never expose write access");
            }
            if let Some(sample) = hub.state(&PointId::new(&view.device.id, "state"))
                && sample.value == Value::Text((if same_id_readd { "31.2" } else { "21.5" }).into())
            {
                states += 1;
            }
        }
        if actual == wanted && states == usize::from(report.sensor_packets > 0 && active) {
            report.moli_devices = actual;
            report.moli_states = states;
            converged = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(12)).await;
    }
    cancel.cancel();
    drop(tx);
    let _ = tokio::time::timeout(Duration::from_secs(3), worker).await;
    if !converged
        || report.moli_devices != wanted
        || (active && report.durable_entries != 1)
        || (report.retained_tombstones > 0 && !same_id_readd && report.durable_entries != 0)
        || (same_id_readd
            && (report.retained_configs != 2
                || report.retained_tombstones != 1
                || report.sensor_packets != 2
                || report.durable_entries != 1
                || report.moli_devices != 1
                || report.moli_states != 1
                || !report.old_state_cleared_before_readd))
    {
        bail!("Moli / durable registry mismatch; no authorization");
    }
    if same_id_readd {
        report.same_id_readd_verified = true;
    }
    Ok(report)
}
