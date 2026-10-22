//! Lab-only Home Assistant WebSocket reader powered by the upstream hass-rs
//! library (MIT), version pinned in Cargo.lock. No WebSocket transport rewrite.
//!
//! Restricts its public API to a loopback WSS-less synthetic HA test server,
//! and exposes no call_service/fire_event or registry mutation commands.
//! The real HACS/Python process integration and secure Unix IPC remain future.
pub mod session;

use anyhow::{Context, Result, bail};
use chrono::DateTime;
use hass_rs::{HassClient, HassEntity, WSEvent};
use std::collections::BTreeMap;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;
use tokio::sync::mpsc::{self, Receiver, error::TryRecvError};

use tokio::time::timeout;
use url::Url;

const WAIT: Duration = Duration::from_secs(5);

/// Single-connection, synthetic sandbox only: no network reachability to
/// production HAOS; a properly authenticated production client will need a
/// separate security review (TLS, tokens, RBAC, reconnection, audit).
pub struct SandboxHaReader {
    client: HassClient,
}

impl SandboxHaReader {
    pub async fn connect_loopback(ws_url: &str, token: &str) -> Result<Self> {
        let url = Url::parse(ws_url).context("invalid WebSocket URL")?;
        let host = url.host_str().unwrap_or_default();
        if url.scheme() != "ws"
            || !matches!(host, "127.0.0.1" | "::1" | "[::1]")
            || url.port().is_none()
            || url.path() != "/api/websocket"
            || url.query().is_some()
            || url.fragment().is_some()
            || !url.username().is_empty()
            || url.password().is_some()
        {
            bail!("sandbox only: explicit loopback WebSocket required");
        }
        if token.is_empty() || token.len() > 4096 {
            bail!("missing or oversize sandbox token");
        }
        let mut client = timeout(WAIT, HassClient::new(ws_url))
            .await
            .context("WebSocket connect timed out")?
            .context("WebSocket connect failed")?;
        timeout(WAIT, client.auth_with_longlivedtoken(token))
            .await
            .context("Home Assistant authentication timed out")?
            .context("Home Assistant authentication rejected")?;
        Ok(Self { client })
    }

    /// Get the *current* typed states without issuing any device commands.
    /// This is a complete read at one point in time, NOT a state_changed
    /// subscription and NOT a durable, gap-free event stream.
    pub async fn states(&mut self) -> Result<Vec<HassEntity>> {
        timeout(WAIT, self.client.get_states())
            .await
            .context("get_states timed out")?
            .context("get_states failed")
    }

    /// Subscribe before snapshot, then apply buffered state-change events.
    /// Upstream hass-rs uses a bounded channel and has no automatic reconnect.
    /// Recreate this mirror on reconnect; it is not gap-free in heavy traffic.
    pub async fn bootstrap_shadow(&mut self) -> Result<(ShadowMirror, ShadowEvents)> {
        let events = timeout(WAIT, self.client.subscribe_event("state_changed"))
            .await
            .context("state subscription timed out")?
            .context("state subscription failed")?;
        // Crucial: drain the 20-item upstream receiver immediately, before
        // awaiting get_states. Otherwise >20 changes during get_states may
        // backpressure upstream's WS reader and block the snapshot response.
        let stream = ShadowEvents::drain_upstream(events);
        let initial = self.states().await?;
        stream.ensure_healthy()?;
        // Caller consumes queued events after bootstrap; timestamp validation
        // prevents an older queued value from overriding the snapshot.
        Ok((ShadowMirror::from_states(initial)?, stream))
    }
}

/// A bounded, explicitly fail-closed relay of upstream hass-rs events.
/// Upstream's subscription channel holds just 20 events; draining it
/// concurrently with get_states prevents a deadlock at bootstrap.
///
/// This is not a guarantee of delivery: if 256 pending events accumulate,
/// the relay marks its stream permanently invalid and requires resnapshot.
pub struct ShadowEvents {
    rx: Receiver<WSEvent>,
    overflow: Arc<AtomicBool>,
}
impl ShadowEvents {
    fn drain_upstream(mut upstream: Receiver<WSEvent>) -> Self {
        let (tx, rx) = mpsc::channel(256);
        let overflow = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&overflow);
        tokio::spawn(async move {
            while let Some(e) = upstream.recv().await {
                if tx.try_send(e).is_err() {
                    flag.store(true, Ordering::Release);
                    break;
                }
            }
        });
        Self { rx, overflow }
    }
    fn ensure_healthy(&self) -> Result<()> {
        if self.overflow.load(Ordering::Acquire) {
            bail!("state_changed stream overflow: full resnapshot required");
        }
        Ok(())
    }
    /// Never expose cached state if the upstream event relay closed while idle.
    pub fn check_live(&self) -> Result<()> {
        self.ensure_healthy()?;
        if self.rx.is_closed() {
            bail!("Home Assistant event source closed: invalidate cached state");
        }
        Ok(())
    }
    pub fn is_closed_or_overflowed(&self) -> bool {
        self.overflow.load(Ordering::Acquire) || self.rx.is_closed()
    }
    /// Returns Err on overflow or stream closure, never silently skips events.
    pub async fn recv(&mut self) -> Result<WSEvent> {
        self.ensure_healthy()?;
        let e = self
            .rx
            .recv()
            .await
            .context("state event relay closed; mirror stale")?;
        self.ensure_healthy()?;
        Ok(e)
    }
    pub fn try_recv(&mut self) -> Result<Option<WSEvent>> {
        self.ensure_healthy()?;
        match self.rx.try_recv() {
            Ok(e) => {
                self.ensure_healthy()?;
                Ok(Some(e))
            }
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => bail!("state event relay closed; mirror stale"),
        }
    }
}

/// Read-only shadow of Home Assistant's Python-owned entity states.
///
/// A caller MUST mark stale on WebSocket closure, timeout, stream overflow,
/// or reconnect before presenting this as current. No cross-session state
/// ownership transfers are implemented here.
#[derive(Debug)]
pub struct ShadowMirror {
    states: BTreeMap<String, HassEntity>,
    valid: bool,
}
impl ShadowMirror {
    pub fn from_states(states: Vec<HassEntity>) -> Result<Self> {
        let mut entries = BTreeMap::new();
        for entity in states {
            if entity.entity_id.is_empty() || entity.entity_id.len() > 512 {
                bail!("invalid HA entity id");
            }
            // Ensure invalid timestamps cannot accidentally overwrite
            // newer values during backlog reconciliation.
            DateTime::parse_from_rfc3339(&entity.last_updated)
                .context("invalid snapshot timestamp")?;
            if entries.insert(entity.entity_id.clone(), entity).is_some() {
                bail!("duplicate entity id in HA snapshot");
            }
        }
        Ok(Self {
            states: entries,
            valid: true,
        })
    }
    pub fn len(&self) -> Result<usize> {
        if !self.valid {
            bail!("state mirror is stale; resnapshot required");
        }
        Ok(self.states.len())
    }
    pub fn is_empty(&self) -> Result<bool> {
        Ok(self.len()? == 0)
    }
    pub fn get(&self, name: &str) -> Result<Option<&HassEntity>> {
        if !self.valid {
            bail!("state mirror is stale; resnapshot required");
        }
        Ok(self.states.get(name))
    }
    pub fn invalidate(&mut self) {
        self.valid = false;
    }

    /// Apply ONLY a state_changed event in a single known connection
    /// generation. Delayed events older than snapshot last_updated are skipped.
    /// Any conflicting/invalid event makes the entire mirror stale.
    pub fn apply(&mut self, msg: &WSEvent) -> Result<()> {
        if !self.valid {
            bail!("mirror must be rebuilt after disconnect");
        }
        match self.try_apply(msg) {
            Ok(()) => Ok(()),
            Err(e) => {
                self.invalidate();
                Err(e)
            }
        }
    }
    fn try_apply(&mut self, msg: &WSEvent) -> Result<()> {
        if msg.event.event_type != "state_changed" {
            bail!("unsupported event type");
        }
        let id = msg
            .event
            .data
            .entity_id
            .as_deref()
            .context("event entity_id missing")?;
        if id.is_empty() || id.len() > 512 {
            bail!("invalid entity_id in event");
        }
        let event_time =
            DateTime::parse_from_rfc3339(&msg.event.time_fired).context("invalid HA event time")?;
        let old_time = self
            .states
            .get(id)
            .map(|s| DateTime::parse_from_rfc3339(&s.last_updated))
            .transpose()
            .context("invalid current state timestamp")?;
        match &msg.event.data.new_state {
            Some(incoming) => {
                if incoming.entity_id != id {
                    bail!("mismatched event entity_id");
                }
                let new_time = DateTime::parse_from_rfc3339(&incoming.last_updated)
                    .context("invalid new-state timestamp")?;
                if old_time.is_some_and(|t| new_time < t) {
                    return Ok(());
                }
                if let Some(old) = self.states.get(id)
                    && old_time == Some(new_time)
                    && old != incoming
                {
                    bail!("ambiguous same-timestamp state change");
                }
                self.states.insert(id.to_owned(), incoming.clone());
            }
            None => {
                if old_time.is_some_and(|t| event_time < t) {
                    return Ok(());
                }
                self.states.remove(id);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::{SinkExt, StreamExt};
    use serde_json::{Value, json};
    use tokio::net::TcpListener;
    use tokio_tungstenite::{accept_async, tungstenite::Message};

    async fn fake_server() -> (String, tokio::task::JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let task = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut ws = accept_async(stream).await.unwrap();
            let mut recorded = vec![];
            ws.send(Message::Text(
                json!({"type":"auth_required","ha_version":"2026.10.1"})
                    .to_string()
                    .into(),
            ))
            .await
            .unwrap();
            while let Some(msg) = ws.next().await {
                let raw = msg.unwrap();
                let Ok(text) = raw.into_text() else { continue };
                let value: Value = serde_json::from_str(&text).unwrap();
                let cmd = value["type"].as_str().unwrap_or("");
                recorded.push(cmd.to_string());
                match cmd {
                    "auth" => {
                        if value["access_token"] == "synthetic-test-token" {
                            ws.send(Message::Text(
                                json!({"type":"auth_ok","ha_version":"2026.10.1"})
                                    .to_string()
                                    .into(),
                            ))
                            .await
                            .unwrap();
                        } else {
                            ws.send(Message::Text(
                                json!({"type":"auth_invalid","message":"invalid"})
                                    .to_string()
                                    .into(),
                            ))
                            .await
                            .unwrap();
                            break;
                        }
                    }
                    "get_states" => {
                        let values=(0..1232).map(|i| json!({
                            "entity_id":format!("sensor.synthetic_{i}"),
                            "state": format!("{i}"),
                            "last_changed":"2026-10-18T08:00:00.000000+00:00",
                            "last_updated":"2026-10-18T08:00:00.000000+00:00",
                            "attributes":{"unit_of_measurement":"C","synthetic":true},
                            "context":{"id":format!("mock-{i}"),"parent_id":null,"user_id":null}
                        })).collect::<Vec<_>>();
                        ws.send(Message::Text(
                            json!({
                                "id":value["id"],"type":"result","success":true,"result":values
                            })
                            .to_string()
                            .into(),
                        ))
                        .await
                        .unwrap();
                        break;
                    }
                    _ => panic!("Unexpected/non-read-only command: {cmd}"),
                }
            }
            recorded
        });
        (format!("ws://{addr}/api/websocket"), task)
    }

    #[tokio::test]
    async fn upstream_hass_rs_authenticates_and_reads_1232_synthetic_states() {
        let (url, server) = fake_server().await;
        let mut client = SandboxHaReader::connect_loopback(&url, "synthetic-test-token")
            .await
            .unwrap();
        let states = client.states().await.unwrap();
        assert_eq!(states.len(), 1232);
        assert_eq!(states[0].entity_id, "sensor.synthetic_0");
        assert_eq!(states[1231].attributes["synthetic"], true);
        drop(client);
        let cmds = server.await.unwrap();
        assert_eq!(cmds, vec!["auth", "get_states"]);
    }

    #[tokio::test]
    async fn upstream_hass_rs_rejects_wrong_token() {
        let (url, server) = fake_server().await;
        assert!(
            SandboxHaReader::connect_loopback(&url, "wrong")
                .await
                .is_err()
        );
        assert_eq!(server.await.unwrap(), vec!["auth"]);
    }

    #[tokio::test]
    async fn disconnect_during_read_fails_within_bounded_time() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut ws = accept_async(stream).await.unwrap();
            ws.send(Message::Text(
                json!({
                    "type":"auth_required","ha_version":"2026.10.1"
                })
                .to_string()
                .into(),
            ))
            .await
            .unwrap();
            let auth = ws.next().await.unwrap().unwrap().into_text().unwrap();
            assert_eq!(
                serde_json::from_str::<Value>(&auth).unwrap()["type"],
                "auth"
            );
            ws.send(Message::Text(
                json!({
                    "type":"auth_ok","ha_version":"2026.10.1"
                })
                .to_string()
                .into(),
            ))
            .await
            .unwrap();
            let request = ws.next().await.unwrap().unwrap().into_text().unwrap();
            assert_eq!(
                serde_json::from_str::<Value>(&request).unwrap()["type"],
                "get_states"
            );
            // No response and abrupt close. The upstream client may keep a
            // pending request, so our façade must never wait indefinitely.
            drop(ws);
        });
        let mut reader = SandboxHaReader::connect_loopback(
            &format!("ws://{addr}/api/websocket"),
            "synthetic-test-token",
        )
        .await
        .unwrap();
        let started = std::time::Instant::now();
        assert!(reader.states().await.is_err());
        assert!(started.elapsed() < Duration::from_secs(6));
        server.await.unwrap();
    }

    fn entity(name: &str, state: &str, updated: &str) -> HassEntity {
        serde_json::from_value(json!({
            "entity_id":name,
            "state":state,
            "last_updated":updated,
            "last_changed":updated,
            "attributes":{"fixture":true},
            "context":{"id":"synthetic","parent_id":null,"user_id":null}
        }))
        .unwrap()
    }
    fn event(name: &str, new_state: Option<&HassEntity>, fired_at: &str) -> WSEvent {
        serde_json::from_value(json!({
            "id":4,
            "event":{
                "event_type":"state_changed",
                "time_fired":fired_at,
                "origin":"LOCAL",
                "context":{"id":"synthetic","parent_id":null,"user_id":null},
                "data":{"entity_id":name,"new_state":new_state,"old_state":null}
            }
        }))
        .unwrap()
    }

    #[test]
    fn shadow_reconciles_newer_events_and_rejects_stale_snapshot_use() {
        let t0 = "2026-10-18T08:00:00.000000+00:00";
        let t1 = "2026-10-18T08:00:01.000000+00:00";
        let mut shadow =
            ShadowMirror::from_states(vec![entity("sensor.fixture", "off", t0)]).unwrap();
        let updated = entity("sensor.fixture", "on", t1);
        shadow
            .apply(&event("sensor.fixture", Some(&updated), t1))
            .unwrap();
        assert_eq!(shadow.get("sensor.fixture").unwrap().unwrap().state, "on");
        // Event from before initial snapshot may arrive in subscribe-first backlog.
        shadow
            .apply(&event(
                "sensor.fixture",
                Some(&entity("sensor.fixture", "off", t0)),
                t0,
            ))
            .unwrap();
        assert_eq!(shadow.get("sensor.fixture").unwrap().unwrap().state, "on");
        shadow.invalidate();
        assert!(shadow.len().is_err());
        assert!(shadow.get("sensor.fixture").is_err());
    }

    #[test]
    fn shadow_applies_valid_removal_and_ignores_out_of_order_deletion() {
        let t0 = "2026-10-18T08:00:00.000000+00:00";
        let t1 = "2026-10-18T08:00:01.000000+00:00";
        let mut shadow =
            ShadowMirror::from_states(vec![entity("sensor.fixture", "on", t1)]).unwrap();
        shadow.apply(&event("sensor.fixture", None, t0)).unwrap();
        assert_eq!(shadow.len().unwrap(), 1);
        shadow
            .apply(&event("sensor.fixture", None, "2026-10-18T08:00:02+00:00"))
            .unwrap();
        assert!(shadow.get("sensor.fixture").unwrap().is_none());
    }

    #[test]
    fn shadow_rejects_conflicting_or_duplicate_entities() {
        let t0 = "2026-10-18T08:00:00.000000+00:00";
        let one = entity("sensor.fixture", "on", t0);
        assert!(ShadowMirror::from_states(vec![one.clone(), one.clone()]).is_err());
        let mut shadow = ShadowMirror::from_states(vec![one.clone()]).unwrap();
        let inconsistent = entity("sensor.different", "off", t0);
        assert!(
            shadow
                .apply(&event("sensor.fixture", Some(&inconsistent), t0))
                .is_err()
        );
        assert!(shadow.len().is_err());
    }

    #[tokio::test]
    async fn upstream_subscribe_then_snapshot_replays_synthetic_state_change() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut ws = accept_async(stream).await.unwrap();
            let send = |v: Value| Message::Text(v.to_string().into());
            ws.send(send(
                json!({"type":"auth_required","ha_version":"2026.10.1"}),
            ))
            .await
            .unwrap();
            let mut commands = Vec::new();
            let mut sub_id = 0;
            while let Some(message) = ws.next().await {
                let raw = message.unwrap();
                let Ok(text) = raw.into_text() else { continue };
                let v: Value = serde_json::from_str(&text).unwrap();
                let cmd = v["type"].as_str().unwrap().to_owned();
                commands.push(cmd.clone());
                match cmd.as_str() {
                    "auth" => ws
                        .send(send(json!({
                            "type":"auth_ok","ha_version":"2026.10.1"
                        })))
                        .await
                        .unwrap(),
                    "subscribe_events" => {
                        assert_eq!(v["event_type"], "state_changed");
                        sub_id = v["id"].as_u64().unwrap();
                        ws.send(send(json!({
                            "type":"result","id":sub_id,"success":true,"result":null
                        })))
                        .await
                        .unwrap();
                    }
                    "get_states" => {
                        assert_ne!(sub_id, 0, "must subscribe before get_states");
                        let initial = vec![entity(
                            "sensor.synthetic",
                            "off",
                            "2026-10-18T08:00:00+00:00",
                        )];
                        ws.send(send(json!({
                            "type":"result","id":v["id"],"success":true,"result":initial
                        })))
                        .await
                        .unwrap();
                        let later = entity("sensor.synthetic", "on", "2026-10-18T08:00:01+00:00");
                        ws.send(send(json!({
                            "type":"event","id":sub_id,"event":{
                                "event_type":"state_changed",
                                "time_fired":"2026-10-18T08:00:01+00:00",
                                "origin":"LOCAL",
                                "context":{"id":"synthetic","parent_id":null,"user_id":null},
                                "data":{"entity_id":"sensor.synthetic",
                                    "old_state":null,"new_state":later}
                            }
                        })))
                        .await
                        .unwrap();
                        break;
                    }
                    _ => panic!("non-read-only WS command: {cmd}"),
                }
            }
            commands
        });
        let mut client = SandboxHaReader::connect_loopback(
            &format!("ws://{addr}/api/websocket"),
            "synthetic-test-token",
        )
        .await
        .unwrap();
        let (mut shadow, mut events) = client.bootstrap_shadow().await.unwrap();
        assert_eq!(
            shadow.get("sensor.synthetic").unwrap().unwrap().state,
            "off"
        );
        let next = timeout(Duration::from_secs(2), events.recv())
            .await
            .unwrap()
            .expect("state event missing");
        shadow.apply(&next).unwrap();
        assert_eq!(shadow.get("sensor.synthetic").unwrap().unwrap().state, "on");
        shadow.invalidate();
        assert!(shadow.get("sensor.synthetic").is_err());
        assert_eq!(
            server.await.unwrap(),
            vec!["auth", "subscribe_events", "get_states"]
        );
    }

    #[tokio::test]
    async fn event_relay_drains_bursts_beyond_upstream_20_slot_limit() {
        let (tx, rx) = mpsc::channel(20);
        let mut relay = ShadowEvents::drain_upstream(rx);
        let t = "2026-10-18T08:00:01+00:00";
        for i in 0..80 {
            let e = event(&format!("sensor.fixture_{i}"), None, t);
            timeout(Duration::from_secs(2), tx.send(e))
                .await
                .unwrap()
                .unwrap();
        }
        for i in 0..80 {
            let received = timeout(Duration::from_secs(2), relay.recv())
                .await
                .unwrap()
                .unwrap();
            assert_eq!(
                received.event.data.entity_id.as_deref(),
                Some(format!("sensor.fixture_{i}").as_str())
            );
        }
        assert!(relay.try_recv().unwrap().is_none());
    }

    #[tokio::test]
    async fn full_shadow_queue_is_detected_not_silently_skipped() {
        let (tx, rx) = mpsc::channel(20);
        let mut relay = ShadowEvents::drain_upstream(rx);
        let e = event("sensor.fixture", None, "2026-10-18T08:00:01+00:00");
        // Simulate a too-slow application while the upstream reader keeps
        // delivering; the relay must switch to fail-closed, not skip silently.
        for _ in 0..300 {
            if timeout(Duration::from_secs(2), tx.send(e.clone()))
                .await
                .unwrap()
                .is_err()
            {
                break; // upstream receiver has been dropped after overflow
            }
        }
        tokio::task::yield_now().await;
        assert!(
            relay.try_recv().is_err(),
            "overload must require fresh snapshot"
        );
        assert!(relay.recv().await.is_err());
    }

    #[tokio::test]
    async fn sandbox_refuses_non_loopback_or_mutable_url_metadata() {
        for s in [
            "ws://192.0.2.202:8123/api/websocket",
            "wss://127.0.0.1:8123/api/websocket",
            "ws://localhost:8123/api/websocket",
            "ws://127.0.0.1:8123/api/not-websocket",
            "ws://127.0.0.1:8123/api/websocket?token=x",
            "ws://user:secret@127.0.0.1:8123/api/websocket",
            "file:///etc/shadow",
        ] {
            assert!(
                SandboxHaReader::connect_loopback(s, "synthetic")
                    .await
                    .is_err(),
                "{s}"
            );
        }
    }
}
