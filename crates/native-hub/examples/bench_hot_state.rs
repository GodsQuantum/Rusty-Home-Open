//! Hot in-memory state read microbenchmark against a genuine, upstream Moli Hub.
//! No persistence, networks, permissions, apps, automations or real devices.
//! NOT a Home Assistant replacement/full-stack benchmark.
use moli_core::{Access, Device, DeviceId, InstanceId, Kind, PointId, PointSpec, Semantic, Value};
use moli_runtime::{BoxFuture, Driver, DriverCtx, Hub, HubOptions, spawn_driver};
use std::{hint::black_box, sync::Arc, time::Instant};
use tokio::sync::{Mutex, oneshot};
use tokio_util::sync::CancellationToken;

const COUNT: usize = 1232;
const WARMUP: usize = 5;
const ROUNDS: usize = 30;

struct Fixture {
    ready: Mutex<Option<oneshot::Sender<()>>>,
}
impl Driver for Fixture {
    fn kind(&self) -> &'static str {
        "p14-synthetic-cache-read"
    }
    fn run<'a>(&'a self, ctx: &'a mut DriverCtx) -> BoxFuture<'a, anyhow::Result<()>> {
        Box::pin(async move {
            let instance = ctx.instance().clone();
            for i in 0..COUNT {
                let id = ctx.device_id(&format!("sensor_{i}"));
                ctx.upsert_device(Device {
                    id: id.clone(),
                    instance: instance.clone(),
                    native_name: format!("Test sensor {i}").into(),
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
                ctx.set_state(&id, "state", Value::Int(i as i64));
            }
            ctx.ready();
            if let Some(tx) = self.ready.lock().await.take() {
                let _ = tx.send(());
            }
            std::future::pending::<()>().await;
            Ok(())
        })
    }
}

fn proc_memory_kib(key: &str) -> u64 {
    std::fs::read_to_string("/proc/self/smaps_rollup")
        .unwrap_or_default()
        .lines()
        .find_map(|line| {
            let (name, rest) = line.split_once(':')?;
            if name == key {
                rest.split_whitespace().next()?.parse().ok()
            } else {
                None
            }
        })
        .unwrap_or(0)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let hub = Hub::new(HubOptions::default())?;
    let (tx, rx) = oneshot::channel();
    let cancel = CancellationToken::new();
    let fixture = Arc::new(Fixture {
        ready: Mutex::new(Some(tx)),
    });
    let handle = spawn_driver(&hub, InstanceId::from("benchmark"), fixture, cancel.clone());
    rx.await?;
    anyhow::ensure!(
        hub.snapshot().devices.len() == COUNT,
        "missing synthetic devices"
    );
    let instance = InstanceId::from("benchmark");
    let points: Vec<PointId> = (0..COUNT)
        .map(|i| PointId::new(&DeviceId::new(&instance, &format!("sensor_{i}")), "state"))
        .collect();
    let mut round_ns = Vec::with_capacity(ROUNDS);
    for round in 0..(WARMUP + ROUNDS) {
        let start = Instant::now();
        let mut checksum = 0i64;
        for (i, point) in points.iter().enumerate() {
            let sample = hub
                .state(point)
                .ok_or_else(|| anyhow::anyhow!("missing state {i}"))?;
            match sample.value {
                Value::Int(number) => {
                    anyhow::ensure!(number == i as i64, "wrong sample");
                    checksum += black_box(number);
                }
                _ => anyhow::bail!("unexpected sample type"),
            }
        }
        let elapsed = start.elapsed().as_nanos();
        anyhow::ensure!(
            checksum == ((COUNT - 1) * COUNT / 2) as i64,
            "wrong checksum"
        );
        if round >= WARMUP {
            round_ns.push(u64::try_from(elapsed)?);
        }
    }
    let output = serde_json::json!({
        "schema_version":1,
        "backend":"moli_native_rust",
        "scope":"hot_in_memory_state_read",
        "entities": COUNT,
        "warmup_rounds":WARMUP,
        "round_ns":round_ns,
        "rss_kb":proc_memory_kib("Rss"),
        "pss_kb":proc_memory_kib("Pss"),
        "durable":false,
        "api":"Moli Hub.state(PointId) synthetic Value::Int",
        "note":"Single process hot-state retrieval only, not a full Home Assistant workload"
    });
    println!("{output}");
    cancel.cancel();
    handle.await?;
    Ok(())
}
