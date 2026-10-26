//! Single bounded, non-production real WebSocket→Moli runner.
//! Genuine WebSocket frames, existing hass-rs reader, genuine upstream Hub.
//! No actuator, service, automation or TLS-production auth. Only loopback.
use crate::ha_event_convergence::EventConverger;
use anyhow::{Context, Result, bail};
use hars_ha_compat::session::ShadowSession;
use hars_interop_moli::registry_stage::RegistryStager;
use serde::Serialize;
use std::{
    path::Path,
    time::{Duration, Instant},
};

#[derive(Serialize, Debug)]
pub struct RunReport {
    pub source: &'static str,
    pub states_verified: usize,
    pub changes_observed: usize,
    pub requested_state_matched: bool,
    pub read_only: bool,
    pub production_ready: bool,
}
#[derive(Serialize, Debug)]
pub struct ReauthReport {
    pub first_epoch: u64,
    pub failed_epoch: u64,
    pub recovered_epoch: u64,
    pub old_state_revoked: bool,
    pub new_hub_verified: bool,
    pub write_commands: u64,
    pub production_ready: bool,
}

/// Re-authenticate the genuine Core WS after revoking the old Moli mirror.
/// Only synthetic entity IDs and the loopback network are permitted.
pub async fn prove_core_reauth(
    root: &Path,
    url: &str,
    token: &str,
    entity: &str,
    expected: &str,
) -> Result<ReauthReport> {
    let mut session = ShadowSession::new();
    let (mut c1, mut r1, first) = session.resync_loopback(url, token).await?;
    let mut old = EventConverger::start(root, session, first).await?;
    if old
        .read_verified(&mut c1, &mut r1, entity)
        .await?
        .as_deref()
        != Some(expected)
    {
        bail!("initial Core read mismatch");
    }
    old.invalidate().await?;
    if old.read_verified(&mut c1, &mut r1, entity).await.is_ok() {
        bail!("revoked source remained accessible");
    }
    let mut session = old.into_revoked_session()?;
    drop(c1);
    drop(r1);
    if session.get(entity).is_ok() {
        bail!("stale snapshot survived revoke");
    }
    if session
        .resync_loopback(url, "p8bh-fake-invalid-token")
        .await
        .is_ok()
    {
        bail!("Core accepted invalid authentication");
    }
    let failed_epoch = session.epoch();
    if session.get(entity).is_ok() {
        bail!("old state restored after denied login");
    }
    let (mut c2, mut r2, recovered) = session.resync_loopback(url, token).await?;
    if recovered != failed_epoch + 1 {
        bail!("reconnect did not advance epoch");
    }
    let mut fresh = EventConverger::start(root, session, recovered).await?;
    if fresh
        .read_verified(&mut c2, &mut r2, entity)
        .await?
        .as_deref()
        != Some(expected)
    {
        bail!("new snapshot or native Hub mismatch");
    }
    fresh.invalidate().await?;
    if fresh.read_verified(&mut c2, &mut r2, entity).await.is_ok() {
        bail!("recovered Hub remained readable after stop");
    }
    Ok(ReauthReport {
        first_epoch: first,
        failed_epoch,
        recovered_epoch: recovered,
        old_state_revoked: true,
        new_hub_verified: true,
        write_commands: 0,
        production_ready: false,
    })
}

/// Scoped laboratory operation, never runs indefinitely or dispatches commands.
/// Requires an existing staged identity and an expected observed literal state.
pub async fn run(
    root: &Path,
    url: &str,
    token: &str,
    entity: &str,
    expected: &str,
    required_changes: usize,
) -> Result<RunReport> {
    if required_changes == 0
        || required_changes > 64
        || entity.is_empty()
        || entity.len() > 255
        || expected.is_empty()
        || expected.len() > 4096
        || expected.chars().any(char::is_control)
    {
        bail!("invalid bounded lab runner arguments")
    }
    let db = RegistryStager::open(root)?;
    let active = db.active()?.context("no active Core registry shadow")?;
    if !active.entities.iter().any(|e| e.entity_id == entity) {
        bail!("lab entity is not in validated Core registry")
    }
    drop(db);
    let mut session = ShadowSession::new();
    let (mut client, mut relay, epoch) = session.resync_loopback(url, token).await?;
    let mut hub = EventConverger::start(root, session, epoch).await?;
    let started = Instant::now();
    let mut since_probe = Instant::now();
    let mut changed = 0usize;
    loop {
        let next = match hub.drain_events(&mut relay).await {
            Ok(n) => n,
            Err(e) => return Err(e),
        };
        changed = changed.saturating_add(next);
        if changed >= required_changes {
            let current = hub.read_verified(&mut client, &mut relay, entity).await?;
            if current.as_deref() != Some(expected) {
                hub.invalidate().await?;
                bail!("lab runner saw no matching authenticated Core state")
            }
            return Ok(RunReport {
                source: "loopback_hass_rs",
                states_verified: 1,
                changes_observed: changed,
                requested_state_matched: true,
                read_only: true,
                production_ready: false,
            });
        }
        if started.elapsed() >= Duration::from_secs(20) {
            hub.invalidate().await?;
            bail!("bounded lab run timed out without sufficient state changes")
        }
        if since_probe.elapsed() >= Duration::from_secs(2) {
            // Count events consumed by a reconciliation probe, otherwise the
            // bounded scenario could lose events before reaching its target.
            changed = changed.saturating_add(
                hub.probe_transport_reconciled(&mut client, &mut relay)
                    .await?,
            );
            since_probe = Instant::now();
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::{SinkExt, StreamExt};
    use serde_json::{Value, json};
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use tokio::net::TcpListener;
    use tokio_tungstenite::{accept_async, tungstenite::Message};

    #[tokio::test]
    async fn real_network_frames_end_in_real_readonly_moli_and_no_write_commands() {
        let tmp = tempfile::tempdir().unwrap();
        let private = tmp.path().join("shadow");
        fs::create_dir(&private).unwrap();
        fs::set_permissions(&private, fs::Permissions::from_mode(0o700)).unwrap();
        let db = RegistryStager::open(&private).unwrap();
        let raw = serde_json::to_vec(&json!({"key":"core.entity_registry","version":1,
            "data":{"entities":[{"entity_id":"sensor.any_provider",
                "platform":"arbitrary_thirdparty", "unique_id":"serial/1",
                "config_entry_id":"fictional_entry"}]}}))
        .unwrap();
        let stage = db.stage(&raw).unwrap();
        db.activate(&stage.batch_sha256, None).unwrap();
        drop(db);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (release_tx, release_rx) = tokio::sync::oneshot::channel::<()>();
        let server = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let mut ws = accept_async(socket).await.unwrap();
            let out = |j: Value| Message::Text(j.to_string().into());
            ws.send(out(
                json!({"type":"auth_required","ha_version":"2026.10.2"}),
            ))
            .await
            .unwrap();
            let mut kinds = Vec::new();
            let mut subscribed = None;
            let mut seen_snapshots = 0;
            while let Some(msg) = ws.next().await {
                let msg = msg.unwrap().into_text().unwrap();
                let j: Value = serde_json::from_str(&msg).unwrap();
                let kind = j["type"].as_str().unwrap().to_owned();
                kinds.push(kind.clone());
                match kind.as_str() {
                    "auth" => ws
                        .send(out(json!({"type":"auth_ok","ha_version":"2026.10.2"})))
                        .await
                        .unwrap(),
                    "subscribe_events" => {
                        subscribed = j["id"].as_u64();
                        ws.send(out(
                            json!({"type":"result","id":j["id"],"success":true,"result":null}),
                        ))
                        .await
                        .unwrap();
                    }
                    "get_states" => {
                        seen_snapshots += 1;
                        assert!(subscribed.is_some());
                        let t = "2026-10-25T00:00:00+00:00";
                        let snapshot = |state: &str, ts: &str| {
                            json!({
                                "entity_id":"sensor.any_provider","state":state,
                                "last_changed":ts,"last_updated":ts,"attributes":{},
                                "context":{"id":"mock","parent_id":null,"user_id":null}
                            })
                        };
                        ws.send(out(json!({"type":"result","id":j["id"],"success":true,
                            "result":[snapshot(if seen_snapshots==1{"off"}else{"on"},
                               if seen_snapshots==1{t}else{"2026-10-25T00:00:01+00:00"})]})))
                            .await
                            .unwrap();
                        if seen_snapshots == 1 {
                            let later = "2026-10-25T00:00:01+00:00";
                            ws.send(out(json!({"type":"event","id":subscribed.unwrap(),"event":{
                                "event_type":"state_changed","time_fired":later,
                                "origin":"LOCAL","context":{"id":"mock","parent_id":null,"user_id":null},
                                "data":{"entity_id":"sensor.any_provider","old_state":null,
                                    "new_state":snapshot("on",later)}}}))).await.unwrap();
                        } else {
                            break;
                        }
                    }
                    other => panic!("non-readonly Home Assistant websocket operation {other}"),
                }
            }
            // Keep the connection alive until the Rust runner has consumed
            // the verification response, avoiding scheduler-dependent CI races.
            let _ = release_rx.await;
            kinds
        });
        let res = run(
            &private,
            &format!("ws://{addr}/api/websocket"),
            "nonprod-fake-token",
            "sensor.any_provider",
            "on",
            1,
        )
        .await
        .unwrap();
        assert_eq!(res.states_verified, 1);
        assert_eq!(res.changes_observed, 1);
        assert!(res.requested_state_matched && res.read_only && !res.production_ready);
        release_tx.send(()).unwrap();
        assert_eq!(
            server.await.unwrap(),
            vec!["auth", "subscribe_events", "get_states", "get_states"]
        );
    }
    #[tokio::test]
    async fn reject_unregistered_entity_before_any_network_attempt() {
        let private = tempfile::tempdir().unwrap();
        let result = run(
            private.path(),
            "ws://127.0.0.1:7001/api/websocket",
            "fixture",
            "sensor.unregistered",
            "on",
            1,
        )
        .await;
        assert!(result.is_err());
    }
}
