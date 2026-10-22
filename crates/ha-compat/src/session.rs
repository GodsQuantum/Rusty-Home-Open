//! Generation-aware read-only HA state mirror lifecycle.
//! A reconnect never restores trust in previous-generation snapshots/events.
//! Offline by default, no auth secrets stored here, no device commands.
use crate::{SandboxHaReader, ShadowEvents, ShadowMirror};
use anyhow::{Result, bail};
use hass_rs::{HassEntity, WSEvent};
use std::time::{Duration, Instant};

const SNAPSHOT_MAX_AGE: Duration = Duration::from_secs(30);

#[derive(Debug, Default)]
pub struct ShadowSession {
    epoch: u64,
    mirror: Option<ShadowMirror>,
    // An epoch may accept exactly one successfully validated snapshot.
    // Closing or invalidating a mirror never re-opens the same epoch.
    snapshot_installed: bool,
    // No idle event implies that a transport is healthy. Require a bounded
    // full re-subscription and snapshot even if its relay remains open.
    snapshot_at: Option<Instant>,
}

impl ShadowSession {
    pub fn new() -> Self {
        Self::default()
    }

    /// Begin the next connection attempt. Previous mirror is discarded
    /// BEFORE network/auth, so failures never expose a stale "current" value.
    pub fn reconnecting(&mut self) -> Result<u64> {
        self.epoch = self
            .epoch
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("shadow epoch exhausted"))?;
        self.mirror = None;
        self.snapshot_at = None;
        self.snapshot_installed = false;
        Ok(self.epoch)
    }

    /// A complete snapshot may become authoritative only after a successful
    /// subscription + get_states handshake on this exact connection epoch.
    pub fn install_snapshot(&mut self, epoch: u64, states: Vec<HassEntity>) -> Result<()> {
        self.ensure_current_epoch(epoch)?;
        if self.snapshot_installed {
            bail!("connection generation already used: reauthentication required");
        }
        let snapshot = ShadowMirror::from_states(states)?;
        self.mirror = Some(snapshot);
        self.snapshot_installed = true;
        self.snapshot_at = Some(Instant::now());
        Ok(())
    }

    pub fn event(&mut self, epoch: u64, event: &WSEvent) -> Result<()> {
        self.ensure_current_epoch(epoch)?;
        if let Err(error) = self.ensure_snapshot_fresh() {
            self.mirror = None;
            return Err(error);
        }
        let mirror = self
            .mirror
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("shadow snapshot missing"))?;
        if let Err(error) = mirror.apply(event) {
            self.mirror = None; // permanently invalidate this epoch after failure
            return Err(error);
        }
        Ok(())
    }

    /// Call on stream close, timeout, overflow or heartbeat failure.
    /// This does NOT automatically rejoin or guarantee no transport gaps.
    pub fn lost_connection(&mut self, epoch: u64) -> Result<()> {
        self.ensure_current_epoch(epoch)?;
        self.mirror = None;
        self.snapshot_at = None;
        // snapshot_installed is intentionally NOT cleared. Reauthentication
        // must increment the epoch before accepting another snapshot.
        Ok(())
    }

    /// Health-checked read: no cached value when source closed or overflowed.
    pub fn read_live(
        &mut self,
        epoch: u64,
        relay: &mut ShadowEvents,
        id: &str,
    ) -> Result<Option<&HassEntity>> {
        self.ensure_current_epoch(epoch)?;
        if let Err(error) = self.ensure_snapshot_fresh() {
            self.mirror = None;
            return Err(error);
        }
        // Drain the bounded relay before presenting a cache value; a mere
        // connection-health check would still expose unprocessed updates.
        // Continuous producers cannot starve readers indefinitely.
        for _ in 0..=256 {
            if !self.try_recv_event(epoch, relay)? {
                return self.get(id);
            }
        }
        self.mirror = None;
        bail!("too many queued HA changes while reading state: resnapshot required");
    }

    /// Explicit heartbeat, reauthentication or transport failure invalidation.
    pub fn transport_failed(&mut self, epoch: u64) -> Result<()> {
        self.lost_connection(epoch)
    }

    /// Nonblocking event poll. An empty queue is valid only while connected.
    pub fn try_recv_event(&mut self, epoch: u64, relay: &mut ShadowEvents) -> Result<bool> {
        self.ensure_current_epoch(epoch)?;
        if let Err(error) = self.ensure_snapshot_fresh() {
            self.mirror = None;
            return Err(error);
        }
        if let Err(error) = relay.check_live() {
            self.mirror = None;
            return Err(error);
        }
        match relay.try_recv() {
            Ok(Some(event)) => {
                self.event(epoch, &event)?;
                Ok(true)
            }
            Ok(None) => Ok(false),
            Err(error) => {
                self.mirror = None;
                Err(error)
            }
        }
    }

    pub fn len(&self) -> Result<usize> {
        self.ensure_snapshot_fresh()?;
        self.mirror
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("shadow is stale / not initialized"))?
            .len()
    }

    pub fn is_empty(&self) -> Result<bool> {
        Ok(self.len()? == 0)
    }

    pub fn get(&self, name: &str) -> Result<Option<&HassEntity>> {
        self.ensure_snapshot_fresh()?;
        self.mirror
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("shadow is stale / not initialized"))?
            .get(name)
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    /// Only for ephemeral localhost fixtures. Discards old states *before*
    /// authentication and a new subscription+snapshot. No live HAOS tokens.
    pub async fn resync_loopback(
        &mut self,
        url: &str,
        token: &str,
    ) -> Result<(SandboxHaReader, ShadowEvents, u64)> {
        let epoch = self.reconnecting()?;
        let mut client = SandboxHaReader::connect_loopback(url, token).await?;
        let (shadow, events) = client.bootstrap_shadow().await?;
        // Real subscription and full snapshot succeeded for this epoch.
        // No second install is allowed after a disconnect or event failure.
        self.mirror = Some(shadow);
        self.snapshot_installed = true;
        self.snapshot_at = Some(Instant::now());
        Ok((client, events, epoch))
    }

    /// Relay overflow or stream closure invalidates all cached states.
    pub async fn recv_event(&mut self, epoch: u64, relay: &mut ShadowEvents) -> Result<()> {
        self.ensure_current_epoch(epoch)?;
        if let Err(error) = self.ensure_snapshot_fresh() {
            self.mirror = None;
            return Err(error);
        }
        // Even when an upstream TCP connection remains open but totally idle,
        // no caller may await a state event past the finite snapshot lease.
        let remaining = self
            .snapshot_at
            .ok_or_else(|| anyhow::anyhow!("HA state snapshot lease missing"))?
            .elapsed();
        let wait = SNAPSHOT_MAX_AGE.saturating_sub(remaining);
        match tokio::time::timeout(wait, relay.recv()).await {
            Ok(Ok(event)) => self.event(epoch, &event),
            Ok(Err(error)) => {
                self.mirror = None;
                Err(error)
            }
            Err(_) => {
                self.mirror = None;
                self.snapshot_at = None;
                bail!("HA state event stream idle past authenticated snapshot lease");
            }
        }
    }

    fn ensure_snapshot_fresh(&self) -> Result<()> {
        let at = self.snapshot_at.ok_or_else(|| {
            anyhow::anyhow!("no active authenticated HA snapshot: resync required")
        })?;
        if at.elapsed() >= SNAPSHOT_MAX_AGE {
            bail!("HA state snapshot lease expired: reauthenticate and resnapshot");
        }
        if self.mirror.is_none() {
            bail!("HA state snapshot revoked: reconnect required");
        }
        Ok(())
    }

    fn ensure_current_epoch(&self, epoch: u64) -> Result<()> {
        if epoch == 0 || epoch != self.epoch {
            bail!("wrong/stale WebSocket connection epoch");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn entity(name: &str, state: &str, time: &str) -> HassEntity {
        serde_json::from_value(json!({
            "entity_id":name,"state":state,"last_changed":time,
            "last_updated":time,"attributes":{},"context":{
                "id":"synthetic","parent_id":null,"user_id":null}
        }))
        .unwrap()
    }

    fn event(name: &str, state: &str, time: &str) -> WSEvent {
        let value = entity(name, state, time);
        serde_json::from_value(json!({
            "id":1,"event":{
                "event_type":"state_changed",
                "time_fired":time,"origin":"LOCAL","context":{
                    "id":"synthetic","parent_id":null,"user_id":null
                },
                "data":{"entity_id":name,"old_state":null,"new_state":value}
            }
        }))
        .unwrap()
    }

    #[tokio::test]
    async fn authentic_ws_resync_never_reuses_old_generation() {
        use futures_util::{SinkExt, StreamExt};
        use serde_json::Value;
        use tokio::net::TcpListener;
        use tokio_tungstenite::{accept_async, tungstenite::Message};
        async fn launch(state: &'static str) -> (String, tokio::task::JoinHandle<()>) {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            let handle = tokio::spawn(async move {
                let (connection, _) = listener.accept().await.unwrap();
                let mut ws = accept_async(connection).await.unwrap();
                let reply = |obj: Value| Message::Text(obj.to_string().into());
                ws.send(reply(
                    json!({"type":"auth_required","ha_version":"2026.10.1"}),
                ))
                .await
                .unwrap();
                let auth = ws.next().await.unwrap().unwrap().into_text().unwrap();
                let v: Value = serde_json::from_str(&auth).unwrap();
                assert_eq!(v["type"], "auth");
                assert_eq!(v["access_token"], "synthetic-test-token");
                ws.send(reply(json!({"type":"auth_ok","ha_version":"2026.10.1"})))
                    .await
                    .unwrap();
                let subscribe = ws.next().await.unwrap().unwrap().into_text().unwrap();
                let sub: Value = serde_json::from_str(&subscribe).unwrap();
                assert_eq!(sub["type"], "subscribe_events");
                ws.send(reply(
                    json!({"type":"result","id":sub["id"],"success":true,"result":null}),
                ))
                .await
                .unwrap();
                let get = ws.next().await.unwrap().unwrap().into_text().unwrap();
                let get: Value = serde_json::from_str(&get).unwrap();
                assert_eq!(get["type"], "get_states");
                ws.send(reply(json!({
                    "type":"result","id":get["id"],"success":true,
                    "result":[entity("sensor.mock",state,"2026-10-18T08:00:00+00:00")]
                })))
                .await
                .unwrap();
            });
            (format!("ws://{addr}/api/websocket"), handle)
        }
        let mut mirror = ShadowSession::new();
        let (url_a, server_a) = launch("old").await;
        let (_client_a, _events_a, epoch_a) = mirror
            .resync_loopback(&url_a, "synthetic-test-token")
            .await
            .unwrap();
        assert_eq!(epoch_a, 1);
        assert_eq!(mirror.get("sensor.mock").unwrap().unwrap().state, "old");
        server_a.await.unwrap();
        mirror.lost_connection(epoch_a).unwrap();
        assert!(mirror.len().is_err());

        let (url_b, server_b) = launch("new").await;
        let (_client_b, _events_b, epoch_b) = mirror
            .resync_loopback(&url_b, "synthetic-test-token")
            .await
            .unwrap();
        assert_eq!(epoch_b, 2);
        assert_eq!(mirror.get("sensor.mock").unwrap().unwrap().state, "new");
        assert!(
            mirror
                .event(
                    epoch_a,
                    &event("sensor.mock", "incorrect", "2026-10-18T09:00:00+00:00")
                )
                .is_err()
        );
        assert_eq!(mirror.get("sensor.mock").unwrap().unwrap().state, "new");
        server_b.await.unwrap();
    }

    #[tokio::test]
    async fn failed_reauthentication_drops_last_snapshot() {
        let mut shadow = ShadowSession::new();
        let epoch = shadow.reconnecting().unwrap();
        shadow
            .install_snapshot(
                epoch,
                vec![entity("sensor.mock", "old", "2026-10-18T08:00:00+00:00")],
            )
            .unwrap();
        assert_eq!(shadow.len().unwrap(), 1);
        let result = shadow
            .resync_loopback("ws://192.0.2.202:8123/api/websocket", "fake")
            .await;
        assert!(result.is_err());
        assert!(
            shadow.len().is_err(),
            "old snapshot was exposed after failed reconnect"
        );
        assert_eq!(shadow.epoch(), 2);
    }

    #[test]
    fn reconnect_discards_stale_state_and_old_events() {
        let mut s = ShadowSession::new();
        assert!(s.len().is_err());
        let a = s.reconnecting().unwrap();
        assert_eq!(a, 1);
        s.install_snapshot(
            a,
            vec![entity("sensor.mock", "off", "2026-10-18T08:00:00+00:00")],
        )
        .unwrap();
        assert_eq!(s.len().unwrap(), 1);
        s.event(a, &event("sensor.mock", "on", "2026-10-18T08:00:01+00:00"))
            .unwrap();
        assert_eq!(s.get("sensor.mock").unwrap().unwrap().state, "on");

        let b = s.reconnecting().unwrap();
        assert_eq!(b, 2);
        assert!(s.get("sensor.mock").is_err());
        assert!(
            s.event(a, &event("sensor.mock", "bad", "2026-10-18T09:00:00+00:00"))
                .is_err()
        );
        assert!(s.install_snapshot(a, vec![]).is_err());
        assert!(s.len().is_err());
        s.install_snapshot(
            b,
            vec![entity("sensor.mock", "fresh", "2026-10-18T10:00:00+00:00")],
        )
        .unwrap();
        assert_eq!(s.get("sensor.mock").unwrap().unwrap().state, "fresh");
        assert!(s.install_snapshot(b, vec![]).is_err());
    }

    #[test]
    fn overflow_disconnect_or_bad_event_fails_closed() {
        let mut s = ShadowSession::new();
        let epoch = s.reconnecting().unwrap();
        s.install_snapshot(
            epoch,
            vec![entity("sensor.mock", "on", "2026-10-18T08:00:00+00:00")],
        )
        .unwrap();
        let bad = event("sensor.different", "on", "2026-10-18T08:00:01+00:00");
        let mut bad = bad;
        bad.event.data.entity_id = Some("sensor.mock".into());
        assert!(s.event(epoch, &bad).is_err());
        assert!(s.len().is_err());
        assert!(
            s.event(
                epoch,
                &event("sensor.mock", "stale", "2026-10-18T09:00:00+00:00")
            )
            .is_err()
        );
        let new_epoch = s.reconnecting().unwrap();
        s.install_snapshot(new_epoch, vec![]).unwrap();
        assert_eq!(s.len().unwrap(), 0);
        s.lost_connection(new_epoch).unwrap();
        assert!(s.get("sensor.mock").is_err());
        assert!(s.lost_connection(epoch).is_err());
    }

    #[test]
    fn invalid_snapshot_does_not_advance_ready_state() {
        let mut s = ShadowSession::new();
        let epoch = s.reconnecting().unwrap();
        let e = entity("sensor.mock", "unknown", "NOT_A_TIMESTAMP");
        assert!(s.install_snapshot(epoch, vec![e]).is_err());
        assert!(s.len().is_err());
        let good = entity("sensor.mock", "unknown", "2026-10-18T08:00:00+00:00");
        s.install_snapshot(epoch, vec![good]).unwrap();
        assert_eq!(s.len().unwrap(), 1);
    }

    #[tokio::test]
    async fn dropped_event_source_invalidates_cache_without_waiting_for_next_event() {
        let (upstream_tx, upstream_rx) = tokio::sync::mpsc::channel(8);
        let mut relay = crate::ShadowEvents::drain_upstream(upstream_rx);
        let mut session = ShadowSession::new();
        let epoch = session.reconnecting().unwrap();
        session
            .install_snapshot(
                epoch,
                vec![entity("sensor.mock", "old", "2026-10-18T08:00:00+00:00")],
            )
            .unwrap();
        assert!(
            session
                .read_live(epoch, &mut relay, "sensor.mock")
                .unwrap()
                .is_some()
        );
        drop(upstream_tx);
        for _ in 0..100 {
            tokio::task::yield_now().await;
            if relay.is_closed_or_overflowed() {
                break;
            }
        }
        assert!(session.read_live(epoch, &mut relay, "sensor.mock").is_err());
        assert!(session.get("sensor.mock").is_err());
        assert!(session.try_recv_event(epoch, &mut relay).is_err());
    }

    #[tokio::test]
    async fn overflow_cannot_be_ignored_by_cached_read() {
        let (upstream_tx, upstream_rx) = tokio::sync::mpsc::channel(512);
        let mut relay = crate::ShadowEvents::drain_upstream(upstream_rx);
        let mut session = ShadowSession::new();
        let epoch = session.reconnecting().unwrap();
        session
            .install_snapshot(
                epoch,
                vec![entity("sensor.mock", "old", "2026-10-18T08:00:00+00:00")],
            )
            .unwrap();
        for _ in 0..400 {
            upstream_tx
                .try_send(event("sensor.mock", "new", "2026-10-18T08:00:01+00:00"))
                .unwrap();
        }
        for _ in 0..1000 {
            if relay.is_closed_or_overflowed() {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert!(
            relay.is_closed_or_overflowed(),
            "bounded relay did not signal overflow"
        );
        assert!(session.read_live(epoch, &mut relay, "sensor.mock").is_err());
        assert!(session.len().is_err());
        drop(upstream_tx);
    }

    #[tokio::test]
    async fn nonblocking_receive_requires_same_generation_and_never_revives_stale_cache() {
        let (tx_old, rx_old) = tokio::sync::mpsc::channel(8);
        let mut old_relay = crate::ShadowEvents::drain_upstream(rx_old);
        let mut session = ShadowSession::new();
        let old_epoch = session.reconnecting().unwrap();
        session
            .install_snapshot(
                old_epoch,
                vec![entity("sensor.mock", "old", "2026-10-18T08:00:00+00:00")],
            )
            .unwrap();
        tx_old
            .send(event("sensor.mock", "changed", "2026-10-18T08:00:01+00:00"))
            .await
            .unwrap();
        for _ in 0..100 {
            if session.try_recv_event(old_epoch, &mut old_relay).unwrap() {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert_eq!(
            session.get("sensor.mock").unwrap().unwrap().state,
            "changed"
        );
        let new_epoch = session.reconnecting().unwrap();
        session
            .install_snapshot(
                new_epoch,
                vec![entity("sensor.mock", "fresh", "2026-10-18T09:00:00+00:00")],
            )
            .unwrap();
        assert!(session.try_recv_event(old_epoch, &mut old_relay).is_err());
        assert_eq!(session.get("sensor.mock").unwrap().unwrap().state, "fresh");
        session.transport_failed(new_epoch).unwrap();
        assert!(session.get("sensor.mock").is_err());
        drop(tx_old);
    }

    #[tokio::test]
    async fn live_read_drains_queued_state_change_before_returning_cached_value() {
        let (upstream_tx, upstream_rx) = tokio::sync::mpsc::channel(8);
        let mut relay = crate::ShadowEvents::drain_upstream(upstream_rx);
        let mut session = ShadowSession::new();
        let epoch = session.reconnecting().unwrap();
        session
            .install_snapshot(
                epoch,
                vec![entity("sensor.mock", "off", "2026-10-18T08:00:00+00:00")],
            )
            .unwrap();
        upstream_tx
            .send(event("sensor.mock", "on", "2026-10-18T08:00:01+00:00"))
            .await
            .unwrap();
        for _ in 0..100 {
            if upstream_tx.capacity() == 8 {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert_eq!(
            upstream_tx.capacity(),
            8,
            "mock event was not forwarded to relay"
        );
        assert_eq!(
            session
                .read_live(epoch, &mut relay, "sensor.mock")
                .unwrap()
                .unwrap()
                .state,
            "on"
        );
        assert!(!session.try_recv_event(epoch, &mut relay).unwrap());
        drop(upstream_tx);
    }

    #[test]
    fn revoked_epoch_must_never_install_a_second_snapshot_without_reconnecting() {
        let mut s = ShadowSession::new();
        let epoch = s.reconnecting().unwrap();
        s.install_snapshot(
            epoch,
            vec![entity(
                "sensor.mock",
                "authenticated",
                "2026-10-18T08:00:00+00:00",
            )],
        )
        .unwrap();
        s.lost_connection(epoch).unwrap();
        // CRITICAL: a forged/get_states-like response without reauthentication
        // must not re-authorize the same connection generation.
        assert!(
            s.install_snapshot(
                epoch,
                vec![entity(
                    "sensor.mock",
                    "untrusted",
                    "2026-10-18T09:00:00+00:00",
                )]
            )
            .is_err()
        );
        assert!(s.get("sensor.mock").is_err());
        let fresh_epoch = s.reconnecting().unwrap();
        s.install_snapshot(
            fresh_epoch,
            vec![entity("sensor.mock", "fresh", "2026-10-18T10:00:00+00:00")],
        )
        .unwrap();
        assert_eq!(s.get("sensor.mock").unwrap().unwrap().state, "fresh");
    }

    #[tokio::test]
    async fn stale_snapshot_lease_blocks_cached_reads_even_if_socket_is_open() {
        use std::time::{Duration, Instant};
        let (upstream_tx, upstream_rx) = tokio::sync::mpsc::channel(8);
        let mut relay = crate::ShadowEvents::drain_upstream(upstream_rx);
        let mut session = ShadowSession::new();
        let epoch = session.reconnecting().unwrap();
        session
            .install_snapshot(
                epoch,
                vec![entity(
                    "sensor.mock",
                    "unverified",
                    "2026-10-18T08:00:00+00:00",
                )],
            )
            .unwrap();
        // Do not sleep for 30s: simulate a monotonic stale lease internally.
        session.snapshot_at = Some(Instant::now() - Duration::from_secs(31));
        assert!(session.get("sensor.mock").is_err());
        assert!(session.len().is_err());
        assert!(session.read_live(epoch, &mut relay, "sensor.mock").is_err());
        assert!(
            session
                .event(
                    epoch,
                    &event("sensor.mock", "fake_refresh", "2026-10-18T09:00:00+00:00",)
                )
                .is_err()
        );
        assert!(session.install_snapshot(epoch, vec![]).is_err());
        let next = session.reconnecting().unwrap();
        session.install_snapshot(next, vec![]).unwrap();
        assert_eq!(session.len().unwrap(), 0);
        drop(upstream_tx);
    }

    #[test]
    fn invalid_first_snapshot_may_retry_before_any_successful_install() {
        let mut s = ShadowSession::new();
        let epoch = s.reconnecting().unwrap();
        let invalid = entity("sensor.mock", "bad", "INVALID_TIMESTAMP");
        assert!(s.install_snapshot(epoch, vec![invalid]).is_err());
        assert!(s.len().is_err());
        s.install_snapshot(
            epoch,
            vec![entity("sensor.mock", "safe", "2026-10-18T08:00:00+00:00")],
        )
        .unwrap();
        assert!(s.install_snapshot(epoch, vec![]).is_err());
        assert_eq!(s.get("sensor.mock").unwrap().unwrap().state, "safe");
    }

    #[tokio::test]
    async fn idle_open_stream_cannot_block_past_snapshot_lease_without_resync() {
        use std::time::{Duration, Instant};
        let (upstream_tx, upstream_rx) = tokio::sync::mpsc::channel(8);
        let mut relay = crate::ShadowEvents::drain_upstream(upstream_rx);
        let mut session = ShadowSession::new();
        let epoch = session.reconnecting().unwrap();
        session
            .install_snapshot(
                epoch,
                vec![entity("sensor.mock", "old", "2026-10-18T08:00:00+00:00")],
            )
            .unwrap();
        session.snapshot_at = Some(Instant::now() - (SNAPSHOT_MAX_AGE - Duration::from_millis(25)));
        let result = tokio::time::timeout(
            Duration::from_secs(2),
            session.recv_event(epoch, &mut relay),
        )
        .await;
        assert!(result.is_ok(), "idle upstream caused an unbounded await");
        assert!(result.unwrap().is_err(), "expired session was accepted");
        assert!(session.get("sensor.mock").is_err());
        assert!(session.install_snapshot(epoch, vec![]).is_err());
        drop(upstream_tx);
    }
}
