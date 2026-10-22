//! Synthetic Moli process-kill boundary probe.
//! Only run via scripts/test_native_hub_kill9.py: it sends SIGKILL to this
//! disposable child, not to any production service.
use hars_native_hub::NativeHubReadOnly;
use moli_core::{Access, Device, DeviceId, InstanceId, Kind, PointId, PointSpec, Semantic, Value};
use moli_runtime::{BoxFuture, Driver, DriverCtx, Hub, HubOptions, spawn_driver};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

const COUNT: usize = 32;
const INSTANCE: &str = "hars-synthetic-kill9";
struct Fake {
    mode: &'static str,
    ready: std::sync::Mutex<Option<oneshot::Sender<()>>>,
}
impl Driver for Fake {
    fn kind(&self) -> &'static str {
        "sandbox-kill9"
    }
    fn run<'a>(&'a self, ctx: &'a mut DriverCtx) -> BoxFuture<'a, anyhow::Result<()>> {
        Box::pin(async move {
            for i in 0..COUNT {
                let id = ctx.device_id(&format!("entity_{i}"));
                ctx.upsert_device(Device {
                    id: id.clone(),
                    instance: ctx.instance().clone(),
                    native_name: format!("Synthetic {i}").into(),
                    manufacturer: None,
                    model: None,
                    description: None,
                    native_room: None,
                    members: vec![],
                    points: vec![PointSpec {
                        key: "state".into(),
                        label: "Value".into(),
                        kind: Kind::Numeric {
                            min: None,
                            max: None,
                            step: None,
                        },
                        access: Access {
                            read: true,
                            write: false,
                        },
                        unit: None,
                        semantic: Semantic::Other,
                    }],
                });
                if self.mode != "restore" {
                    ctx.set_state(&id, "state", Value::Int(1000 + i as i64));
                }
            }
            ctx.ready();
            if let Some(tx) = self.ready.lock().unwrap().take() {
                let _ = tx.send(());
            }
            std::future::pending::<()>().await;
            #[allow(unreachable_code)]
            Ok(())
        })
    }
}
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let mode = args.next().ok_or_else(|| anyhow::anyhow!("missing mode"))?;
    let path = PathBuf::from(args.next().ok_or_else(|| anyhow::anyhow!("missing path"))?);
    if args.next().is_some()
        || !matches!(
            mode.as_str(),
            "flushed" | "unflushed" | "restore" | "validate_strict"
        )
    {
        anyhow::bail!("usage: crash_probe MODE FLAT_SCRATCH_FILE");
    }
    // Tests enforce ephemeral parent under /runtime/; deny accidental
    // placement outside project by requiring filename dedicated to the probe.
    if path.file_name().and_then(|s| s.to_str()) != Some("crash-probe-state.json") {
        anyhow::bail!("unsafe probe output filename");
    }
    if mode == "validate_strict" {
        let _validated = NativeHubReadOnly::new(Some(path))?;
        println!("STRICT_OK");
        return Ok(());
    }
    let hub = Hub::new(HubOptions {
        state_path: Some(path),
        ..HubOptions::default()
    })?;
    let (tx, rx) = oneshot::channel();
    let fake = Arc::new(Fake {
        mode: if mode == "restore" {
            "restore"
        } else {
            "write"
        },
        ready: std::sync::Mutex::new(Some(tx)),
    });
    let stop = CancellationToken::new();
    let job = spawn_driver(&hub, InstanceId::from(INSTANCE), fake, stop.clone());
    tokio::time::timeout(std::time::Duration::from_secs(5), rx).await??;
    if mode == "restore" {
        let mut recovered = 0;
        for i in 0..COUNT {
            let id = DeviceId::new(&InstanceId::from(INSTANCE), &format!("entity_{i}"));
            if let Some(sample) = hub.state(&PointId::new(&id, "state")) {
                if sample.value != Value::Int(1000 + i as i64) {
                    anyhow::bail!("corrupt value for item {}", i);
                }
                recovered += 1;
            }
        }
        println!("RECOVERED {recovered}");
        stop.cancel();
        job.await?;
        return Ok(());
    }
    if mode == "flushed" && !hub.persist_state().await? {
        anyhow::bail!("no state flushed");
    }
    println!("READY {mode} {COUNT}");
    std::io::stdout().flush()?;
    // Deliberately NEVER shut down cleanly. Parent will kill -9 once READY
    // confirms the appropriate saved/unsaved state boundary.
    std::future::pending::<()>().await;
    #[allow(unreachable_code)]
    Ok(())
}
