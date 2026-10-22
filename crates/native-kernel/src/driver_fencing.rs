//! P1f.1: Moli driver validates fencing epoch at the SYNTHETIC effect sink.
//! No hardware, network, authenticated users or general HA compatibility.
use crate::fencing::FencedSink;
use anyhow::{Context, Result, bail};
use moli_core::{Access, Device, InstanceId, Kind, Origin, PointId, PointSpec, Semantic, Value};
use moli_runtime::{BoxFuture, Driver, DriverCtx, Hub, HubOptions, spawn_driver};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

const INSTANCE: &str = "hars-fenced";
const CONTROL: &str = "hars-fenced:actuator/command";
const SENSOR: &str = "hars-fenced:actuator/on";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    protocol: String,
    command_id: String,
    owner: String,
    epoch: u64,
    requested_on: bool,
}
fn point(key: &str, kind: Kind, write: bool) -> PointSpec {
    PointSpec {
        key: key.into(),
        label: key.into(),
        kind,
        access: Access {
            read: !write,
            write,
        },
        unit: None,
        semantic: Semantic::Other,
    }
}
struct FencedDriver {
    root: PathBuf,
}
impl Driver for FencedDriver {
    fn kind(&self) -> &'static str {
        "hars-epoch-synthetic-driver"
    }
    fn run<'a>(&'a self, ctx: &'a mut DriverCtx) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            let device = ctx.device_id("actuator");
            ctx.upsert_device(Device {
                id: device.clone(),
                instance: ctx.instance().clone(),
                native_name: "fenced-fake-actuator".into(),
                manufacturer: None,
                model: None,
                description: None,
                native_room: None,
                members: vec![],
                points: vec![
                    point("command", Kind::Text, true),
                    point("on", Kind::Binary, false),
                ],
            });
            // Drop DB handle before another controller claims a new epoch.
            let old = FencedSink::open(&self.root)?.actuator()?;
            ctx.set_state(
                &device,
                "on",
                Value::Bool(old.as_ref().is_some_and(|v| v.requested_on)),
            );
            ctx.ready();
            while let Some(command) = ctx.next_command().await {
                if command.key.as_ref() != "command" {
                    command.reply(Err("unsupported fake point".into()));
                    continue;
                }
                let Value::Text(raw) = &command.value else {
                    command.reply(Err("typed epoch command expected".into()));
                    continue;
                };
                let parsed = serde_json::from_str::<Envelope>(raw);
                let outcome = (|| -> Result<Option<bool>> {
                    let e = parsed.context("malformed or missing epoch envelope")?;
                    if e.protocol != "hars-fence-v1" {
                        bail!("wrong epoch protocol")
                    }
                    let sink = FencedSink::open(&self.root)?;
                    sink.apply(&e.owner, e.epoch, &e.command_id, e.requested_on)?;
                    Ok(sink.actuator()?.map(|v| v.requested_on))
                })();
                match outcome {
                    Ok(Some(value)) => {
                        ctx.set_state(&device, "on", Value::Bool(value));
                        command.reply(Ok(()));
                    }
                    Ok(None) => command.reply(Err("no committed fake effect".into())),
                    Err(_) => command.reply(Err("fence rejected synthetic command".into())),
                }
            }
            Ok(())
        })
    }
}
async fn wait_ready(hub: &Hub) -> Result<()> {
    for _ in 0..200 {
        if hub.state(&PointId::from(SENSOR)).is_some() {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    bail!("fenced fake Moli driver not ready")
}
async fn send(hub: &Hub, owner: &str, epoch: u64, id: &str, value: bool) -> Result<()> {
    let envelope = Envelope {
        protocol: "hars-fence-v1".into(),
        command_id: id.into(),
        owner: owner.into(),
        epoch,
        requested_on: value,
    };
    hub.command(
        &PointId::from(CONTROL),
        Value::Text(serde_json::to_string(&envelope)?.into()),
        Origin::Ui,
        None,
    )
    .await
    .map_err(|e| anyhow::anyhow!("Moli fake driver command refused: {e}"))
}
async fn await_on(hub: &Hub, want: bool) -> Result<()> {
    for _ in 0..100 {
        if hub.state(&PointId::from(SENSOR)).map(|v| v.value) == Some(Value::Bool(want)) {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    bail!("fenced fake actuator not updated to requested state")
}
/// Execute real Moli Hub -> driver -> redb fenced synthetic effect,
/// then transfer ownership and prove the old epoch cannot actuate again.
pub async fn prove(root: &Path) -> Result<serde_json::Value> {
    let sink = FencedSink::open(root)?;
    let initial = sink.grant_first("driver_a")?;
    drop(sink);
    let hub = Hub::new(HubOptions::default())?;
    let cancel = CancellationToken::new();
    let driver = spawn_driver(
        &hub,
        InstanceId::from(INSTANCE),
        Arc::new(FencedDriver {
            root: root.to_path_buf(),
        }),
        cancel.clone(),
    );
    wait_ready(&hub).await?;
    send(&hub, "driver_a", initial.value, "a_first", true).await?;
    await_on(&hub, true).await?;
    let sink = FencedSink::open(root)?;
    let new = sink.transfer("driver_a", initial.value, "driver_b")?;
    drop(sink);
    if send(&hub, "driver_a", initial.value, "a_late", false)
        .await
        .is_ok()
    {
        bail!("obsolete controller changed the fake Moli driver")
    }
    await_on(&hub, true).await?;
    send(&hub, "driver_b", new.value, "b_first", false).await?;
    await_on(&hub, false).await?;
    // Invalid/unauthorized packet is never allowed to mutate synthetic state.
    if hub
        .command(
            &PointId::from(CONTROL),
            Value::Text("{}".into()),
            Origin::Ui,
            None,
        )
        .await
        .is_ok()
    {
        bail!("malformed driver epoch packet accepted")
    }
    let result = FencedSink::open(root)?;
    let effect = result.actuator()?.context("missing synthetic effect")?;
    if effect.owner != "driver_b" || effect.epoch != 2 || effect.requested_on {
        bail!("wrong fenced effect committed")
    }
    drop(result);
    cancel.cancel();
    let _ = tokio::time::timeout(Duration::from_secs(4), driver).await;
    drop(hub);
    let reopened = FencedSink::open(root)?;
    let restored = reopened
        .actuator()?
        .context("fenced effect absent after restart")?;
    if restored.epoch != 2 || restored.owner != "driver_b" || restored.requested_on {
        bail!("driver-side synthetic fencing not durable after reopen")
    }
    Ok(serde_json::json!({
        "native_moli_driver":true,
        "validated_at_synthetic_effect":true,
        "stale_owner_denied":true,
        "malformed_envelope_denied":true,
        "durable_epoch":new.value,
        "no_python":true,
        "hardware_fenced":false,
        "production_ready":false,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn real_moli_driver_checks_epoch_before_fake_effect() {
        let folder = tempfile::tempdir().unwrap();
        std::fs::set_permissions(folder.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let proof = prove(folder.path()).await.unwrap();
        assert_eq!(proof["durable_epoch"], 2);
        assert_eq!(proof["hardware_fenced"], false);
    }
    use std::os::unix::fs::PermissionsExt;
}
