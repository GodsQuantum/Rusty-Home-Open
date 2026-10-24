//! Read-only event-to-Moli convergence, built on existing hass-rs mirror.
//! No second transport, no action/automation, no production authentication.
//! Event source MUST be a fully authenticated ShadowSession epoch. An upstream
//! relay failure, invalid timestamp, or disconnected session revokes visibility.
use anyhow::{Context, Result, bail};
use hars_ha_compat::{SandboxHaReader, ShadowEvents, session::ShadowSession};
use hars_interop_moli::registry_stage::RegistryStager;
#[cfg(test)]
use hass_rs::WSEvent;
#[cfg(test)]
use moli_core::Origin;
use moli_core::{Access, Device, InstanceId, Kind, PointId, PointSpec, Semantic, Value};
use moli_runtime::{BoxFuture, Driver, DriverCtx, Hub, HubOptions, spawn_driver};
use std::{path::Path, sync::Arc, time::Duration};
use tokio::{
    sync::{mpsc, oneshot},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Debug)]
struct Entry {
    entity: String,
    point: String,
}
enum Update {
    Refresh(Vec<Option<String>>, oneshot::Sender<()>),
    Clear(oneshot::Sender<()>),
}
struct DriverShadow {
    entries: Vec<Entry>,
    updates: tokio::sync::Mutex<mpsc::Receiver<Update>>,
}
fn installed(ctx: &DriverCtx, entry: &Entry, state: Option<&str>) -> Result<()> {
    let point = PointId::from(entry.point.as_str());
    let (device, key) = point.split().context("invalid native point")?;
    if key != "state" || device.instance() != *ctx.instance() {
        bail!("wrong identity owner")
    }
    ctx.upsert_device(Device {
        id: device.clone(),
        instance: ctx.instance().clone(),
        native_name: entry.entity.clone().into(),
        manufacturer: None,
        model: None,
        native_room: None,
        members: Vec::new(),
        description: Some("Core-authenticated read-only shadow; no commands".into()),
        points: vec![PointSpec {
            key: "state".into(),
            label: "State".into(),
            kind: Kind::Text,
            access: Access {
                read: true,
                write: false,
            },
            unit: None,
            semantic: Semantic::Other,
        }],
    });
    if let Some(state) = state {
        ctx.set_state(&device, "state", Value::Text(state.into()));
    }
    Ok(())
}
impl Driver for DriverShadow {
    fn kind(&self) -> &'static str {
        "hars-ha-ws-shadow-readonly"
    }
    fn run<'a>(&'a self, ctx: &'a mut DriverCtx) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            let mut rx = self.updates.lock().await;
            for entry in &self.entries {
                // Upstream Moli may preserve its in-memory state across a
                // supervised driver restart. Clear it before publishing this
                // new driver instance, never inherit an old WS generation.
                let device = PointId::from(entry.point.as_str())
                    .split()
                    .context("invalid starting identity")?
                    .0;
                ctx.remove_device(&device);
                installed(ctx, entry, None)?;
            }
            ctx.ready();
            loop {
                tokio::select! {
                    msg=rx.recv()=>{
                        match msg {
                            Some(Update::Refresh(states,reply))=>{
                                if states.len()!=self.entries.len(){bail!("shadow length mismatch")}
                                for (entry,state) in self.entries.iter().zip(states.iter()){
                                    let device=PointId::from(entry.point.as_str()).split()
                                        .context("corrupt Moli point")?.0;
                                    // Removing devices clears their cached states in actual Moli
                                    // before any change is presented to other readers.
                                    ctx.remove_device(&device);
                                    installed(ctx,entry,state.as_deref())?;
                                }
                                let _=reply.send(());
                            }
                            Some(Update::Clear(reply))=>{
                                for entry in &self.entries{
                                    let device=PointId::from(entry.point.as_str()).split()
                                        .context("corrupt Moli point")?.0;
                                    ctx.remove_device(&device);
                                }
                                let _=reply.send(());
                            }
                            None=>break,
                        }
                    },
                    command=ctx.next_command()=>{
                        if let Some(command)=command {
                            command.reply(Err("Core WebSocket shadow forbids commands".into()));
                        }else{break}
                    }
                }
            }
            Ok(())
        })
    }
}

/// Intentionally no public Hub handle: callers cannot bypass the freshness
/// guard to read a cached point after losing a WebSocket session.
pub struct EventConverger {
    hub: Hub,
    session: ShadowSession,
    epoch: u64,
    entries: Vec<Entry>,
    tx: mpsc::Sender<Update>,
    cancel: CancellationToken,
    handle: JoinHandle<()>,
    valid: bool,
}
impl EventConverger {
    /// Caller must already have completed authentic WS subscribe-before-snapshot
    /// handshake for this exact session epoch. A fixture-created session is only
    /// permitted in isolated lab tests. No production token is read here.
    pub async fn start(dir: &Path, session: ShadowSession, epoch: u64) -> Result<Self> {
        if session.epoch() != epoch || session.len().is_err() {
            bail!("not a live session epoch")
        }
        let db = RegistryStager::open(dir)?;
        let stage = db.active()?.context("no staged HA Core entity registry")?;
        let rows = crate::ha_registry_shadow::validated(&stage)?;
        let entries = rows
            .into_iter()
            .map(|(entity, point)| Entry { entity, point })
            .collect::<Vec<_>>();
        let hub = Hub::new(HubOptions::default())?;
        let (tx, rx) = mpsc::channel(4);
        let cancel = CancellationToken::new();
        let handle = spawn_driver(
            &hub,
            InstanceId::from("ha-compat"),
            Arc::new(DriverShadow {
                entries: entries.clone(),
                updates: tokio::sync::Mutex::new(rx),
            }),
            cancel.clone(),
        );
        let mut ready = false;
        for _ in 0..300 {
            if hub.snapshot().devices.len() == entries.len() {
                ready = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        if !ready {
            cancel.cancel();
            bail!("genuine Moli Hub failed to register all HA devices")
        }
        let mut shadow = Self {
            hub,
            session,
            epoch,
            entries,
            tx,
            cancel,
            handle,
            valid: true,
        };
        shadow.refresh().await?;
        Ok(shadow)
    }
    fn observed(&self) -> Result<Vec<Option<String>>> {
        self.session.len()?;
        let mut results = Vec::with_capacity(self.entries.len());
        for e in &self.entries {
            let state = self.session.get(&e.entity)?;
            let text = state.and_then(|s| {
                (!matches!(s.state.as_str(), "unknown" | "unavailable")).then(|| s.state.clone())
            });
            if text
                .as_ref()
                .is_some_and(|s| s.is_empty() || s.len() > 4096 || s.chars().any(char::is_control))
            {
                bail!("unsafe state from authenticated Core epoch")
            }
            results.push(text)
        }
        Ok(results)
    }
    async fn refresh(&mut self) -> Result<()> {
        if !self.valid {
            bail!("shadow already invalid")
        }
        let values = self.observed()?;
        let (reply, recv) = oneshot::channel();
        self.tx
            .send(Update::Refresh(values, reply))
            .await
            .context("read-only driver has stopped")?;
        tokio::time::timeout(Duration::from_secs(3), recv)
            .await
            .context("state convergence timed out")?
            .context("driver dropped state acknowledgement")?;
        Ok(())
    }
    /// Immediately hide every Moli device after any untrusted state source.
    pub async fn invalidate(&mut self) -> Result<()> {
        self.valid = false;
        let _ = self.session.lost_connection(self.epoch);
        let (reply, recv) = oneshot::channel();
        self.tx
            .send(Update::Clear(reply))
            .await
            .context("driver lost during revoke")?;
        tokio::time::timeout(Duration::from_secs(3), recv)
            .await
            .context("revocation timed out")?
            .context("revocation unacknowledged")?;
        Ok(())
    }
    #[cfg(test)]
    async fn apply_event(&mut self, epoch: u64, event: &WSEvent) -> Result<()> {
        if !self.valid {
            bail!("shadow invalid; reconstruct from new Core snapshot")
        }
        // An old event must not invalidate a current generation; reject only.
        if epoch != self.epoch {
            bail!("stale websocket epoch")
        }
        if let Err(e) = self.session.event(epoch, event) {
            self.invalidate().await?;
            return Err(e);
        }
        if let Err(e) = self.refresh().await {
            self.invalidate().await?;
            return Err(e);
        }
        Ok(())
    }
    /// Active WebSocket transport probe. hass-rs 0.5.0 does not guarantee
    /// that a closed TCP connection immediately closes its subscription
    /// receiver; therefore an empty event queue is NOT proof of liveness.
    /// A bounded official get_states request checks the authenticated client.
    /// An error/timeout revokes all Moli devices before reporting failure.
    /// This is a liveness probe, NOT a gap-free change stream.
    pub async fn probe_transport(&mut self, client: &mut SandboxHaReader) -> Result<()> {
        if !self.valid {
            bail!("already revoked")
        }
        if let Err(e) = client.states().await {
            self.invalidate().await?;
            return Err(e);
        }
        Ok(())
    }
    /// Bounded incremental drain of the ALREADY EXISTING hass-rs event relay.
    /// Stream closure/overflow revokes all values; no transparent retry.
    pub async fn drain_events(&mut self, relay: &mut ShadowEvents) -> Result<usize> {
        let mut count = 0;
        for _ in 0..=256 {
            match self.session.try_recv_event(self.epoch, relay) {
                Ok(true) => {
                    if let Err(e) = self.refresh().await {
                        self.invalidate().await?;
                        return Err(e);
                    }
                    count += 1;
                }
                Ok(false) => return Ok(count),
                Err(e) => {
                    self.invalidate().await?;
                    return Err(e);
                }
            }
        }
        self.invalidate().await?;
        bail!("event relay saturated; reauthenticate and resnapshot")
    }
    /// Security-scoped read. Do not expose a raw Hub: it might contain state
    /// briefly while the next relay event is queued.
    fn observed_state(&self, id: &str) -> Result<Option<String>> {
        if !self.valid {
            bail!("shadow source lost: resnapshot required")
        }
        // This checks the 30s bounded lease even on idle streams.
        let core = self.session.get(id)?;
        if core.is_none() {
            return Ok(None);
        }
        let Some(entry) = self.entries.iter().find(|entry| entry.entity == id) else {
            return Ok(None);
        };
        Ok(self
            .hub
            .state(&PointId::from(entry.point.as_str()))
            .and_then(|sample| match sample.value {
                Value::Text(v) => Some(v.to_string()),
                _ => None,
            }))
    }
    /// Safe public read, including a bounded authenticated transport probe.
    /// An idle hass-rs subscription is not sufficient evidence of liveness.
    pub async fn read_verified(
        &mut self,
        client: &mut SandboxHaReader,
        relay: &mut ShadowEvents,
        id: &str,
    ) -> Result<Option<String>> {
        self.probe_transport(client).await?;
        self.drain_events(relay).await?;
        self.observed_state(id)
    }
    #[cfg(test)]
    async fn read_queued_lab(
        &mut self,
        relay: &mut ShadowEvents,
        id: &str,
    ) -> Result<Option<String>> {
        self.drain_events(relay).await?;
        self.observed_state(id)
    }
    #[cfg(test)]
    async fn prove_write_denied(&self) -> Result<()> {
        let Some(first) = self.entries.first() else {
            return Ok(());
        };
        if self
            .hub
            .command(
                &PointId::from(first.point.as_str()),
                Value::Text("forbidden".into()),
                Origin::Ui,
                None,
            )
            .await
            .is_ok()
        {
            bail!("Moli Hub accepted a write to a read-only HA Core shadow")
        }
        Ok(())
    }
    #[cfg(test)]
    fn device_count(&self) -> usize {
        self.hub.snapshot().devices.len()
    }
}
impl Drop for EventConverger {
    fn drop(&mut self) {
        self.cancel.cancel();
        self.handle.abort();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::{fs, os::unix::fs::PermissionsExt};
    fn entity(id: &str, state: &str, time: &str) -> hass_rs::HassEntity {
        serde_json::from_value(json!({
            "entity_id":id,"state":state,"last_updated":time,"last_changed":time,
            "attributes":{},"context":{"id":"test","parent_id":null,"user_id":null}
        }))
        .unwrap()
    }
    fn event(id: &str, state: &str, time: &str) -> WSEvent {
        serde_json::from_value(json!({
            "id":1,"event":{"event_type":"state_changed","time_fired":time,
            "origin":"LOCAL","context":{"id":"test","parent_id":null,"user_id":null},
            "data":{"entity_id":id,"old_state":null,"new_state":entity(id,state,time)}
        }}))
        .unwrap()
    }
    fn prepare() -> (tempfile::TempDir, std::path::PathBuf) {
        let home = tempfile::tempdir().unwrap();
        let dir = home.path().join("private");
        fs::create_dir(&dir).unwrap();
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o700)).unwrap();
        let reg = RegistryStager::open(&dir).unwrap();
        let entries = (0..2)
            .map(|i| {
                json!({
                    "entity_id":format!("sensor.any_vendor_{i}"),
                    "platform":"any_provider",
                    "unique_id":format!("serial/{i}"),"config_entry_id":"core-test"
                })
            })
            .collect::<Vec<_>>();
        let raw = serde_json::to_vec(&json!({"version":1,"key":"core.entity_registry",
            "data":{"entities":entries}}))
        .unwrap();
        let stage = reg.stage(&raw).unwrap();
        reg.activate(&stage.batch_sha256, None).unwrap();
        drop(reg);
        (home, dir)
    }
    const T0: &str = "2026-10-25T00:00:00+00:00";
    const T1: &str = "2026-10-25T00:00:01+00:00";
    const T2: &str = "2026-10-25T00:00:02+00:00";
    #[tokio::test]
    async fn mpsc_to_actual_moli_hub_is_readonly_stale_safe_and_revocable() {
        let (_home, dir) = prepare();
        let mut session = ShadowSession::new();
        let epoch = session.reconnecting().unwrap();
        session
            .install_snapshot(epoch, vec![entity("sensor.any_vendor_0", "off", T0)])
            .unwrap();
        let mut live = EventConverger::start(&dir, session, epoch).await.unwrap();
        assert_eq!(live.device_count(), 2);
        assert_eq!(
            live.observed_state("sensor.any_vendor_0")
                .unwrap()
                .as_deref(),
            Some("off")
        );
        assert!(
            live.observed_state("sensor.any_vendor_1")
                .unwrap()
                .is_none()
        );
        live.prove_write_denied().await.unwrap();
        live.apply_event(epoch, &event("sensor.any_vendor_0", "on", T1))
            .await
            .unwrap();
        assert_eq!(
            live.observed_state("sensor.any_vendor_0")
                .unwrap()
                .as_deref(),
            Some("on")
        );
        live.apply_event(epoch, &event("sensor.any_vendor_0", "off", T0))
            .await
            .unwrap();
        assert_eq!(
            live.observed_state("sensor.any_vendor_0")
                .unwrap()
                .as_deref(),
            Some("on")
        );
        assert!(
            live.apply_event(epoch + 1, &event("sensor.any_vendor_0", "off", T2))
                .await
                .is_err()
        );
        assert_eq!(
            live.observed_state("sensor.any_vendor_0")
                .unwrap()
                .as_deref(),
            Some("on")
        );
        live.apply_event(epoch, &event("sensor.any_vendor_1", "21", T1))
            .await
            .unwrap();
        assert_eq!(
            live.observed_state("sensor.any_vendor_1")
                .unwrap()
                .as_deref(),
            Some("21")
        );
        // Equal timestamp with contradictory state must revoke the entire Hub.
        assert!(
            live.apply_event(epoch, &event("sensor.any_vendor_1", "22", T1))
                .await
                .is_err()
        );
        assert!(live.observed_state("sensor.any_vendor_0").is_err());
        assert_eq!(live.device_count(), 0);
        assert!(
            live.apply_event(epoch, &event("sensor.any_vendor_0", "on", T2))
                .await
                .is_err()
        );
    }
    #[tokio::test]
    async fn unavailability_removal_and_unregistered_events_never_leave_fake_states() {
        let (_home, dir) = prepare();
        let mut session = ShadowSession::new();
        let epoch = session.reconnecting().unwrap();
        session
            .install_snapshot(epoch, vec![entity("sensor.any_vendor_0", "23", T0)])
            .unwrap();
        let mut hub = EventConverger::start(&dir, session, epoch).await.unwrap();
        assert_eq!(
            hub.observed_state("sensor.any_vendor_0")
                .unwrap()
                .as_deref(),
            Some("23")
        );
        hub.apply_event(epoch, &event("sensor.any_vendor_0", "unavailable", T1))
            .await
            .unwrap();
        assert!(hub.observed_state("sensor.any_vendor_0").unwrap().is_none());
        hub.apply_event(epoch, &event("sensor.any_vendor_0", "24", T2))
            .await
            .unwrap();
        assert_eq!(
            hub.observed_state("sensor.any_vendor_0")
                .unwrap()
                .as_deref(),
            Some("24")
        );
        let deletion: WSEvent = serde_json::from_value(json!({
            "id":1,"event":{"event_type":"state_changed",
            "time_fired":"2026-10-25T00:00:03+00:00","origin":"LOCAL",
            "context":{"id":"test","parent_id":null,"user_id":null},
            "data":{"entity_id":"sensor.any_vendor_0","old_state":null,"new_state":null}}
        }))
        .unwrap();
        hub.apply_event(epoch, &deletion).await.unwrap();
        assert!(hub.observed_state("sensor.any_vendor_0").unwrap().is_none());
        hub.apply_event(epoch, &event("sensor.unregistered", "evil", T2))
            .await
            .unwrap();
        assert_eq!(hub.device_count(), 2);
        assert!(hub.observed_state("sensor.unregistered").unwrap().is_none());
        hub.prove_write_denied().await.unwrap();
    }

    #[tokio::test]
    async fn disconnect_discards_devices_and_a_new_epoch_requires_fresh_snapshot() {
        let (_home, dir) = prepare();
        let mut session = ShadowSession::new();
        let old_epoch = session.reconnecting().unwrap();
        session
            .install_snapshot(old_epoch, vec![entity("sensor.any_vendor_0", "old", T0)])
            .unwrap();
        let mut a = EventConverger::start(&dir, session, old_epoch)
            .await
            .unwrap();
        assert!(a.prove_write_denied().await.is_ok());
        a.invalidate().await.unwrap();
        assert_eq!(a.device_count(), 0);
        assert!(a.observed_state("sensor.any_vendor_0").is_err());
        drop(a);
        let mut renewed = ShadowSession::new();
        let epoch = renewed.reconnecting().unwrap();
        renewed
            .install_snapshot(epoch, vec![entity("sensor.any_vendor_0", "fresh", T2)])
            .unwrap();
        let new = EventConverger::start(&dir, renewed, epoch).await.unwrap();
        assert_eq!(
            new.observed_state("sensor.any_vendor_0")
                .unwrap()
                .as_deref(),
            Some("fresh")
        );
    }

    #[tokio::test]
    async fn actual_loopback_websocket_drives_moli_and_disconnect_revokes_everything() {
        use futures_util::{SinkExt, StreamExt};
        use hars_ha_compat::session::ShadowSession;
        use serde_json::Value as Json;
        use tokio::net::TcpListener;
        use tokio_tungstenite::{accept_async, tungstenite::Message};
        let (_home, dir) = prepare();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let (stop_tx, stop_rx) = oneshot::channel::<()>();
        let server = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let mut ws = accept_async(socket).await.unwrap();
            let write = |data: Json| Message::Text(data.to_string().into());
            ws.send(write(
                json!({"type":"auth_required","ha_version":"2026.10.2"}),
            ))
            .await
            .unwrap();
            let mut subscribed = None;
            let mut cmds = Vec::new();
            while let Some(data) = ws.next().await {
                let text = data.unwrap().into_text().unwrap();
                let v: Json = serde_json::from_str(&text).unwrap();
                let cmd = v["type"].as_str().unwrap().to_owned();
                cmds.push(cmd.clone());
                match cmd.as_str() {
                    "auth" => ws
                        .send(write(json!({"type":"auth_ok","ha_version":"2026.10.2"})))
                        .await
                        .unwrap(),
                    "subscribe_events" => {
                        assert_eq!(v["event_type"], "state_changed");
                        subscribed = v["id"].as_u64();
                        ws.send(write(
                            json!({"type":"result","id":v["id"],"success":true,"result":null}),
                        ))
                        .await
                        .unwrap();
                    }
                    "get_states" => {
                        assert!(subscribed.is_some(), "subscription must precede snapshot");
                        ws.send(write(json!({"type":"result","id":v["id"],"success":true,
                            "result":[entity("sensor.any_vendor_0","off",T0)]})))
                            .await
                            .unwrap();
                        ws.send(write(
                            json!({"type":"event","id":subscribed.unwrap(),"event":{
                                "event_type":"state_changed","time_fired":T1,"origin":"LOCAL",
                                "context":{"id":"test","parent_id":null,"user_id":null},
                                "data":{"entity_id":"sensor.any_vendor_0","old_state":null,
                                    "new_state":entity("sensor.any_vendor_0","on",T1)}
                            }}),
                        ))
                        .await
                        .unwrap();
                        break;
                    }
                    other => panic!("unexpected write operation: {other}"),
                }
            }
            let _ = stop_rx.await;
            ws.close(None).await.unwrap();
            cmds
        });
        let mut session = ShadowSession::new();
        let (mut transport, mut events, epoch) = session
            .resync_loopback(
                &format!("ws://{address}/api/websocket"),
                "synthetic-test-token",
            )
            .await
            .unwrap();
        let mut converger = EventConverger::start(&dir, session, epoch).await.unwrap();
        let mut updated = false;
        for _ in 0..80 {
            if let Ok(Some(state)) = converger
                .read_queued_lab(&mut events, "sensor.any_vendor_0")
                .await
                && state == "on"
            {
                updated = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert!(
            updated,
            "real WS state event never reached true upstream Moli"
        );
        assert_eq!(
            converger.observed_state("sensor.any_vendor_1").unwrap(),
            None
        );
        converger.prove_write_denied().await.unwrap();
        stop_tx.send(()).unwrap();
        assert_eq!(
            server.await.unwrap(),
            vec!["auth", "subscribe_events", "get_states"]
        );
        // hass-rs can retain an idle event relay after TCP close. Our
        // explicit authenticated get_states liveness probe must time out
        // or reject; never trust an empty queue indefinitely.
        assert!(
            converger
                .read_verified(&mut transport, &mut events, "sensor.any_vendor_0")
                .await
                .is_err()
        );
        assert_eq!(converger.device_count(), 0);
        assert!(converger.observed_state("sensor.any_vendor_0").is_err());
    }
}
