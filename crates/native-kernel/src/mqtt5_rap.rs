//! MQTT 5 RAP=1 laboratory proof, never a production broker driver.
//! No HAOS, secrets, actuator commands, or host/LAN networking.
use crate::mqtt_discovery::{self, Intent, Rejection};
use crate::mqtt_live::require_network_none;
use anyhow::{Result, bail};
use rumqttc5::{
    AsyncClient, Event, Incoming, MqttOptions, QoS, RetainForwardRule, SubscribeFilterInput,
    SubscribeReasonCode,
};
use serde::Serialize;
use std::io::Write;
use std::time::Duration;

const TOPIC: &str = "homeassistant/sensor/p2f_probe/config";

#[derive(Debug, Default, Serialize)]
pub struct RapReport {
    pub received: usize,
    pub ignored_empty_nonretained: usize,
    pub validated_retained_configs: usize,
    pub validated_retained_deletes: usize,
    pub mqtt5_rap: bool,
    pub read_only: bool,
    pub production_ready: bool,
}

/// Broker-local MQTT 5 subscriber. RETAIN bits are counted only on actual
/// packets received on an established MQTT 5 RAP=1 subscription. A SUBACK
/// signals enrollment, not an authoritative retained snapshot boundary.
pub async fn observe(expected: usize) -> Result<RapReport> {
    require_network_none()?;
    if expected != 3 {
        bail!("MQTT5 RAP lab expects exactly three synthetic live messages")
    }
    let mut opts = MqttOptions::new("hars-mqtt5-rap-lab", ("127.0.0.1", 1883));
    opts.set_keep_alive(5);
    let (client, mut loopback) = AsyncClient::builder(opts).capacity(8).build();
    client
        .subscribe_many([SubscribeFilterInput::new(TOPIC, QoS::AtLeastOnce)
            .preserve_retain(true)
            .retain_forward_rule(RetainForwardRule::Never)])
        .await?;
    let mut report = RapReport {
        mqtt5_rap: true,
        read_only: true,
        ..RapReport::default()
    };
    let deadline = tokio::time::Instant::now() + Duration::from_secs(12);
    let mut suback = false;
    while report.received < expected {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            bail!("MQTT5 RAP lab timed out waiting for three live messages");
        }
        match tokio::time::timeout(remaining, loopback.poll()).await {
            Ok(Ok(Event::Incoming(Incoming::SubAck(ack)))) => {
                if suback
                    || ack.return_codes.len() != 1
                    || !matches!(ack.return_codes[0], SubscribeReasonCode::Success(_))
                {
                    bail!("MQTT5 RAP broker refused or duplicated the subscription");
                }
                suback = true;
                println!("LAB_MQTT5_RAP_READY");
                std::io::stdout().flush()?;
            }
            Ok(Ok(Event::Incoming(Incoming::Publish(p)))) => {
                if !suback || p.topic.as_ref() != TOPIC.as_bytes() {
                    bail!("unexpected MQTT5 packet or unvalidated topic");
                }
                report.received += 1;
                match (p.retain, p.payload.is_empty()) {
                    (false, true) => {
                        if mqtt_discovery::preview(TOPIC, p.payload.as_ref(), false)
                            != Err(Rejection::NotRetained)
                        {
                            bail!("nonretained empty MQTT5 payload incorrectly accepted");
                        }
                        report.ignored_empty_nonretained += 1;
                    }
                    (true, false) => match mqtt_discovery::preview(TOPIC, p.payload.as_ref(), true)
                    {
                        Ok(Intent::Preview { sensors, .. })
                            if sensors.len() == 1 && sensors[0].unique_id == "lab_p2f_probe" =>
                        {
                            report.validated_retained_configs += 1;
                        }
                        _ => bail!("unsafe MQTT5 retained config"),
                    },
                    (true, true) => match mqtt_discovery::preview(TOPIC, &[], true) {
                        Ok(Intent::Remove { .. }) => report.validated_retained_deletes += 1,
                        _ => bail!("invalid MQTT5 retained tombstone"),
                    },
                    (false, false) => bail!("unexpected MQTT5 unretained configuration"),
                }
            }
            Ok(Ok(_)) => {}
            Ok(Err(err)) => bail!("MQTT5 RAP isolated broker error: {err}"),
            Err(_) => bail!("MQTT5 RAP packet wait timed out"),
        }
    }
    if !suback
        || report.ignored_empty_nonretained != 1
        || report.validated_retained_configs != 1
        || report.validated_retained_deletes != 1
    {
        bail!("MQTT5 RAP retained classification mismatch");
    }
    drop(client);
    Ok(report)
}
