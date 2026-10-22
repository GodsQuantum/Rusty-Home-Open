//! Moli Hub real-driver synthetic workload; isolated, no real integrations.
use moli_core::{Access, Device, DeviceId, InstanceId, Kind, PointId, PointSpec, Semantic, Value};
use moli_runtime::{BoxFuture, Driver, DriverCtx, Hub, HubOptions, spawn_driver};
use std::{hint::black_box, sync::Arc, time::Instant};
use tokio::sync::{Mutex, mpsc, oneshot};
use tokio_util::sync::CancellationToken;

const COUNT: usize = 1232;
type Update = (usize, i64, oneshot::Sender<()>);
struct SyntheticDriver {
    updates: Mutex<mpsc::Receiver<Update>>,
    ready: Mutex<Option<oneshot::Sender<()>>>,
    send_initial: bool,
}
impl Driver for SyntheticDriver {
    fn kind(&self) -> &'static str {
        "synthetic-readonly-benchmark"
    }
    fn run<'a>(&'a self, ctx: &'a mut DriverCtx) -> BoxFuture<'a, anyhow::Result<()>> {
        Box::pin(async move {
            let instance = ctx.instance().clone();
            for i in 0..COUNT {
                let id = ctx.device_id(&format!("sensor_{i}"));
                ctx.upsert_device(Device {
                    id: id.clone(),
                    instance: instance.clone(),
                    native_name: format!("Synthetic {i}").into(),
                    manufacturer: None,
                    model: None,
                    description: None,
                    native_room: None,
                    members: vec![],
                    points: vec![PointSpec {
                        key: "state".into(),
                        label: "State".into(),
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
                if self.send_initial {
                    ctx.set_state(&id, "state", Value::Int(i as i64));
                }
            }
            ctx.ready();
            if let Some(tx) = self.ready.lock().await.take() {
                let _ = tx.send(());
            }
            let mut rx = self.updates.lock().await;
            while let Some((i, v, ack)) = rx.recv().await {
                let id = ctx.device_id(&format!("sensor_{i}"));
                ctx.set_state(&id, "state", Value::Int(v));
                let _ = ack.send(());
            }
            Ok(())
        })
    }
}
fn start_driver(
    hub: &Hub,
    initial: bool,
) -> (
    mpsc::Sender<Update>,
    oneshot::Receiver<()>,
    CancellationToken,
    tokio::task::JoinHandle<()>,
) {
    let (tx, rx) = mpsc::channel(64);
    let (ready_tx, ready_rx) = oneshot::channel();
    let driver = Arc::new(SyntheticDriver {
        updates: Mutex::new(rx),
        ready: Mutex::new(Some(ready_tx)),
        send_initial: initial,
    });
    let cancel = CancellationToken::new();
    let handle = spawn_driver(hub, InstanceId::from("mock"), driver, cancel.clone());
    (tx, ready_rx, cancel, handle)
}
fn report(kind: &str, mut v: Vec<u128>) {
    v.sort_unstable();
    println!(
        "{}",
        serde_json::json!({
            "backend":kind, "entities":COUNT, "n":v.len(),
            "p50_ns":v[v.len()/2], "p95_ns":v[v.len()*95/100],
            "p99_ns":v[v.len()*99/100]
        })
    );
}
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let tmp = tempfile::tempdir()?;
    let file = tmp.path().join("state.json");
    let options = HubOptions {
        state_path: Some(file.clone()),
        ..Default::default()
    };
    let hub = Hub::new(options.clone())?;
    let (tx, ready, cancel, handle) = start_driver(&hub, true);
    ready.await?;
    assert_eq!(hub.snapshot().devices.len(), COUNT);
    let mut times = Vec::new();
    for i in 0..COUNT {
        let (ack, rx) = oneshot::channel();
        let start = Instant::now();
        tx.send((i, (i + 1) as i64, ack)).await?;
        rx.await?;
        times.push(start.elapsed().as_nanos());
    }
    report("moli_hub_synthetic_channel_to_driver_state_write", times);
    let mut times = Vec::new();
    for i in 0..COUNT {
        let point = PointId::new(
            &DeviceId::new(&InstanceId::from("mock"), &format!("sensor_{i}")),
            "state",
        );
        let start = Instant::now();
        let sample = hub
            .state(&point)
            .ok_or_else(|| anyhow::anyhow!("missing state {i}"))?;
        times.push(start.elapsed().as_nanos());
        assert_eq!(sample.value, Value::Int((i + 1) as i64));
        black_box(sample);
    }
    report("moli_hub_state_read", times);
    let persisted = hub.persist_state().await?;
    assert!(persisted);
    assert!(file.is_file());
    cancel.cancel();
    handle.await?;
    drop(tx);
    drop(hub);
    let hub2 = Hub::new(options)?;
    let (tx2, ready2, cancel2, handle2) = start_driver(&hub2, false);
    ready2.await?;
    assert_eq!(hub2.snapshot().devices.len(), COUNT);
    let mut recovered = 0;
    for i in 0..COUNT {
        let point = PointId::new(
            &DeviceId::new(&InstanceId::from("mock"), &format!("sensor_{i}")),
            "state",
        );
        let sample = hub2
            .state(&point)
            .ok_or_else(|| anyhow::anyhow!("restore missing {i}"))?;
        assert_eq!(sample.value, Value::Int((i + 1) as i64));
        recovered += 1;
    }
    println!(
        "{}",
        serde_json::json!({
            "backend":"moli_cache_after_explicit_persist_and_clean_restart",
            "recovered":recovered,"expected":COUNT,
            "note":"Explicit persist_state on graceful shutdown; NOT a crash/power-loss durability test"
        })
    );
    cancel2.cancel();
    handle2.await?;
    drop(tx2);
    Ok(())
}
