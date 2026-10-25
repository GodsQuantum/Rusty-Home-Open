//! Genuine upstream Moli Hub read-only projection of the durable HA registry.
//! No live HA state, entity commands, user authentication or production cutover.
use anyhow::{Context, Result, bail};
use hars_interop_moli::{
    HaEntity, project,
    registry_stage::{RegistryStager, Stage, parse_core_state_snapshot},
};
use moli_core::{
    Access, Device, DriverStatus, InstanceId, Kind, Origin, PointId, PointSpec, Semantic, Value,
};
use moli_runtime::{BoxFuture, Driver, DriverCtx, Hub, HubOptions, spawn_driver};
use serde::Serialize;
use serde_json::{Map, Value as JsonValue};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    sync::Arc,
    time::Duration,
};
use tokio_util::sync::CancellationToken;

struct RegistryShadowDriver {
    entries: Vec<(String, String, Option<String>)>,
}
impl Driver for RegistryShadowDriver {
    fn kind(&self) -> &'static str {
        "hars-ha-readonly-registry-shadow"
    }
    fn run<'a>(&'a self, ctx: &'a mut DriverCtx) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            for (id, native, state) in &self.entries {
                let point = PointId::from(native.as_str());
                let (device, key) = point.split().context("invalid native point")?;
                if key != "state" || device.instance() != *ctx.instance() {
                    bail!("registry identity does not belong to HA shadow driver")
                }
                ctx.upsert_device(Device {
                    id: device.clone(),
                    instance: ctx.instance().clone(),
                    native_name: id.clone().into(),
                    manufacturer: None,
                    model: None,
                    description: Some("Core entity registry identity only; STATE UNKNOWN".into()),
                    native_room: None,
                    members: vec![],
                    points: vec![PointSpec {
                        key: "state".into(),
                        label: "State unknown".into(),
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
                    ctx.set_state(&device, "state", Value::Text(state.clone().into()));
                }
            }
            ctx.ready();
            // In case a future upstream version misroutes any command to
            // this read-only driver, refuse it; NEVER publish a state.
            while let Some(command) = ctx.next_command().await {
                command.reply(Err("identity shadow refuses all commands".to_owned()))
            }
            Ok(())
        })
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct ShadowReport {
    pub staged_entities: usize,
    pub native_devices: usize,
    pub unknown_states: usize,
    pub known_states: usize,
    pub read_only: bool,
    pub commands_denied: bool,
    pub real_core_runtime: bool,
    pub production_ready: bool,
}

pub(crate) fn validated(stage: &Stage) -> Result<Vec<(String, String)>> {
    if stage.version != 1
        || !stage.read_only
        || !stage.no_device_commands
        || !stage.no_live_states
        || stage.entity_count != stage.entities.len()
    {
        bail!("untrusted stage metadata")
    }
    let mut rows = Vec::with_capacity(stage.entities.len());
    let mut entities = BTreeSet::new();
    let mut natives = BTreeSet::new();
    for e in &stage.entities {
        if e.state_known
            || !entities.insert(e.entity_id.as_str())
            || !natives.insert(e.native_point.as_str())
        {
            bail!("untrusted, ambiguous, or live registry identity")
        }
        let projected = project(&HaEntity {
            entity_id: e.entity_id.clone(),
            platform: e.platform.clone(),
            config_entry_id: e.config_entry_id.clone(),
            unique_id: Some(e.unique_id.clone()),
            state: JsonValue::Null,
            attributes: Map::new(),
        })
        .map_err(|_| anyhow::anyhow!("unprojectable identity"))?;
        if projected.point_id.as_str() != e.native_point || projected.stable_identity != e.stable {
            bail!("identity stage does not match upstream Moli adapter")
        }
        rows.push((e.entity_id.clone(), e.native_point.clone()));
    }
    Ok(rows)
}
/// Unprivileged offline proof using a REAL upstream Moli runtime Hub.
/// Loads already staged Home Assistant Core identities but no live states.
/// The proof is never exposed as a device/automation control interface.
pub async fn observe(dir: &Path) -> Result<ShadowReport> {
    observe_snapshot(dir, None).await
}
/// One bounded, read-only snapshot from a real Core state API.
/// This is NOT a gap-free WebSocket mirror and cannot command a device.
pub async fn observe_states(dir: &Path, raw: &[u8]) -> Result<ShadowReport> {
    observe_snapshot(dir, Some(raw)).await
}
async fn observe_snapshot(dir: &Path, raw: Option<&[u8]>) -> Result<ShadowReport> {
    let db = RegistryStager::open(dir)?;
    let stage = db
        .active()?
        .context("no approved read-only registry shadow")?;
    let rows = validated(&stage)?;
    drop(db);
    let mut observed = BTreeMap::new();
    if let Some(raw) = raw {
        for state in parse_core_state_snapshot(raw)? {
            if !rows.iter().any(|(id, _)| id == &state.entity_id) {
                bail!("snapshot contains entity not present in verified Core registry")
            }
            if !matches!(state.state.as_str(), "unknown" | "unavailable") {
                observed.insert(state.entity_id, state.state);
            }
        }
    }
    let rows = rows
        .into_iter()
        .map(|(id, native)| {
            let value = observed.get(&id).cloned();
            (id, native, value)
        })
        .collect::<Vec<_>>();
    let known = rows.iter().filter(|(_, _, v)| v.is_some()).count();
    let hub = Hub::new(HubOptions::default())?;
    let cancellation = CancellationToken::new();
    let handle = spawn_driver(
        &hub,
        InstanceId::from("ha-compat"),
        Arc::new(RegistryShadowDriver {
            entries: rows.clone(),
        }),
        cancellation.clone(),
    );
    let mut ready = false;
    for _ in 0..350 {
        let snapshot = hub.snapshot();
        if snapshot.devices.len() == rows.len()
            && snapshot
                .drivers
                .iter()
                .any(|s| matches!(s.status, DriverStatus::Running))
        {
            ready = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let result = async {
        if !ready {
            bail!("upstream Moli Hub never registered all HA identities")
        }
        for (_, id, expected) in &rows {
            let point = PointId::from(id.as_str());
            let (device, key) = point.split().context("missing point")?;
            let entry = hub
                .device(&device)
                .context("device lost after registration")?;
            let spec = entry
                .device
                .point(key)
                .context("read-only state spec not installed")?;
            let current = hub.state(&point).map(|v| v.value);
            let expected_value = expected.as_ref().map(|v| Value::Text(v.clone().into()));
            if !spec.access.read || spec.access.write || current != expected_value {
                bail!("identity shadow erroneously writable or assigned fabricated state")
            }
        }
        // One deliberately attempted UI command: must be refused BY THE
        // GENUINE MOLI HUB without ever routing it to a physical actuator.
        let mut denied = true;
        if let Some((_, id, _)) = rows.first() {
            denied = hub
                .command(
                    &PointId::from(id.as_str()),
                    Value::Text("would-command".into()),
                    Origin::Ui,
                    None,
                )
                .await
                .is_err();
        }
        if !denied {
            bail!("read-only Moli Hub unexpectedly accepted a command")
        }
        Ok(ShadowReport {
            staged_entities: rows.len(),
            native_devices: hub.snapshot().devices.len(),
            unknown_states: rows.len() - known,
            known_states: known,
            read_only: true,
            commands_denied: denied,
            real_core_runtime: false,
            production_ready: false,
        })
    }
    .await;
    cancellation.cancel();
    let _ = tokio::time::timeout(Duration::from_secs(3), handle).await;
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    fn sample(number: usize) -> Vec<u8> {
        let ents = (0..number)
            .map(|i| {
                json!({
                    "entity_id":format!("sensor.any_vendor_{i}"),
                    "platform":format!("provider_{}",i%29),
                    "unique_id":format!("serial/{i}"),
                    "config_entry_id":format!("integration_{}",i%7)
                })
            })
            .collect::<Vec<_>>();
        serde_json::to_vec(&json!({"key":"core.entity_registry","version":1,
            "minor_version":25,"data":{"entities":ents}}))
        .unwrap()
    }
    #[tokio::test]
    async fn genuine_upstream_moli_hub_registers_readonly_without_state() {
        let dir = tempfile::tempdir().unwrap();
        let private = dir.path().join("private");
        fs::create_dir(&private).unwrap();
        fs::set_permissions(&private, fs::Permissions::from_mode(0o700)).unwrap();
        let stage = RegistryStager::open(&private).unwrap();
        let b = stage.stage(&sample(2)).unwrap();
        stage.activate(&b.batch_sha256, None).unwrap();
        drop(stage);
        let proof = observe(&private).await.unwrap();
        assert_eq!(proof.staged_entities, 2);
        assert_eq!(proof.native_devices, 2);
        assert_eq!(proof.unknown_states, 2);
        assert!(proof.commands_denied && proof.read_only && !proof.production_ready);
    }
    #[tokio::test]
    async fn exact_official_state_snapshot_updates_only_registered_readonly_points() {
        let dir = tempfile::tempdir().unwrap();
        let private = dir.path().join("private");
        fs::create_dir(&private).unwrap();
        fs::set_permissions(&private, fs::Permissions::from_mode(0o700)).unwrap();
        let db = RegistryStager::open(&private).unwrap();
        let stage = db.stage(&sample(2)).unwrap();
        db.activate(&stage.batch_sha256, None).unwrap();
        drop(db);
        let state = serde_json::to_vec(&json!([
            {"entity_id":"sensor.any_vendor_0","state":"21.5",
             "last_updated":"2026-10-24T23:59:00+00:00"}
        ]))
        .unwrap();
        let result = observe_states(&private, &state).await.unwrap();
        assert_eq!(result.known_states, 1);
        assert_eq!(result.unknown_states, 1);
        assert_eq!(result.native_devices, 2);
        assert!(result.commands_denied && result.read_only && !result.real_core_runtime);
        assert_eq!(observe(&private).await.unwrap().known_states, 0);
        let unknown = serde_json::to_vec(&json!([
            {"entity_id":"sensor.spoofed","state":"on",
             "last_updated":"2026-10-24T23:59:00+00:00"}
        ]))
        .unwrap();
        assert!(observe_states(&private, &unknown).await.is_err());
        let unavailable = serde_json::to_vec(&json!([
            {"entity_id":"sensor.any_vendor_0","state":"unavailable",
             "last_updated":"2026-10-24T23:59:00+00:00"}
        ]))
        .unwrap();
        assert_eq!(
            observe_states(&private, &unavailable)
                .await
                .unwrap()
                .known_states,
            0
        );
    }

    #[tokio::test]
    async fn more_than_thousand_vendors_are_not_hardcoded() {
        let dir = tempfile::tempdir().unwrap();
        let private = dir.path().join("private");
        fs::create_dir(&private).unwrap();
        fs::set_permissions(&private, fs::Permissions::from_mode(0o700)).unwrap();
        let stage = RegistryStager::open(&private).unwrap();
        let b = stage.stage(&sample(1232)).unwrap();
        stage.activate(&b.batch_sha256, None).unwrap();
        drop(stage);
        let proof = observe(&private).await.unwrap();
        assert_eq!(proof.native_devices, 1232);
        assert_eq!(proof.unknown_states, 1232);
        assert!(proof.commands_denied);
    }
}
