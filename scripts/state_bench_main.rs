//! Synthetic 1,232-entry workload applied to genuine upstream state crates.
//! Moli Hub requires driver lifecycle; not included until matched fake driver.
use hc_state::StateStore;
use hc_types::device::DeviceState;
use homecore::{Context, EntityId, StateMachine};
use std::hint::black_box;
use std::time::Instant;

fn summary(
    label: &str,
    mut samples: Vec<u128>,
    count: usize,
    persisted: Option<bool>,
    extra: &str,
) {
    samples.sort_unstable();
    let p50 = samples[samples.len() / 2];
    let p95 = samples[samples.len() * 95 / 100];
    let p99 = samples[samples.len() * 99 / 100];
    println!(
        "{}",
        serde_json::json!({"backend":label,"sample_count":samples.len(),"entity_count":count,"p50_ns":p50,"p95_ns":p95,"p99_ns":p99,"durable_after_reopen":persisted,"note":extra})
    );
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    const COUNT: usize = 1232;
    let root = tempfile::tempdir()?;
    let sm = StateMachine::new();
    let ruview_entities: Vec<_> = (0..COUNT)
        .map(|i| EntityId::parse(format!("sensor.synthetic_{i}")))
        .collect::<Result<Vec<_>, _>>()?;
    for (i, id) in ruview_entities.iter().enumerate() {
        sm.set(
            id.clone(),
            i.to_string(),
            serde_json::json!({"seq":i,"unit":"mock"}),
            Context::new(),
        );
    }
    assert_eq!(sm.all().len(), COUNT);
    let mut times = Vec::with_capacity(COUNT);
    for (i, id) in ruview_entities.iter().enumerate() {
        let start = Instant::now();
        let next = sm.set(
            id.clone(),
            (i + 1).to_string(),
            serde_json::json!({"seq":i+1,"unit":"mock"}),
            Context::new(),
        );
        black_box(next);
        times.push(start.elapsed().as_nanos());
    }
    for (i, id) in ruview_entities.iter().enumerate() {
        let s = sm.get(id).ok_or("RuView missing state")?;
        assert_eq!(s.state, (i + 1).to_string());
    }
    summary(
        "ruview_homecore_dashmap_hot_write",
        times,
        COUNT,
        Some(false),
        "in-memory state set; all values present; no persistent state backend",
    );
    let mut times = Vec::with_capacity(COUNT);
    for id in &ruview_entities {
        let start = Instant::now();
        black_box(sm.get(id));
        times.push(start.elapsed().as_nanos());
    }
    summary(
        "ruview_homecore_dashmap_get",
        times,
        COUNT,
        None,
        "single warm per-entity get; Arc clone",
    );
    drop(sm);
    let reopened = StateMachine::new();
    assert_eq!(reopened.all().len(), 0);
    println!(
        "{}",
        serde_json::json!({"backend":"ruview_homecore_reboot","recovered":0,"expected":COUNT,"note":"no persistent state supplied by tested P1 crate"})
    );

    // homeCore-io: each of these 1,232 entries is committed to redb (durable).
    // History/Audit use separate SQLite files, zero network or real devices.
    let state_path = root.path().join("state.redb");
    let history_path = root.path().join("history.db");
    let audit_path = root.path().join("audit.db");
    let st = StateStore::open_with_audit(
        state_path.to_str().ok_or("bad temp path")?,
        history_path.to_str().ok_or("bad temp path")?,
        audit_path.to_str().ok_or("bad temp path")?,
    )
    .await?;
    let devices: Vec<_> = (0..COUNT)
        .map(|i| {
            let mut d = DeviceState::new(
                format!("synthetic.device_{i}"),
                format!("Mock {i}"),
                "ha-rust-bench",
            );
            d.available = true;
            d.attributes
                .insert("state".into(), serde_json::json!(i.to_string()));
            d
        })
        .collect();
    for d in &devices {
        st.upsert_device(d).await?;
    }
    let mut times = Vec::with_capacity(COUNT);
    for (i, device) in devices.iter().enumerate() {
        let mut updated = device.clone();
        updated
            .attributes
            .insert("state".into(), serde_json::json!((i + 1).to_string()));
        let start = Instant::now();
        st.upsert_device(&updated).await?;
        times.push(start.elapsed().as_nanos());
    }
    summary(
        "homecore_io_redb_durable_hot_write",
        times,
        COUNT,
        Some(true),
        "transactional redb committed per device; not semantically equivalent to in-memory RuView",
    );
    let mut times = Vec::with_capacity(COUNT);
    for (i, d) in devices.iter().enumerate() {
        let start = Instant::now();
        let result = st
            .get_device(&d.device_id)
            .await?
            .ok_or("homeCore missing state")?;
        times.push(start.elapsed().as_nanos());
        assert_eq!(
            result.attributes["state"],
            serde_json::json!((i + 1).to_string())
        );
    }
    summary(
        "homecore_io_redb_get",
        times,
        COUNT,
        Some(true),
        "redb get with async spawn_blocking and JSON deserialize",
    );
    drop(st);
    let reopened = StateStore::open_with_audit(
        state_path.to_str().ok_or("bad temp path")?,
        history_path.to_str().ok_or("bad temp path")?,
        audit_path.to_str().ok_or("bad temp path")?,
    )
    .await?;
    let mut recovered = 0;
    for (i, d) in devices.iter().enumerate() {
        let item = reopened
            .get_device(&d.device_id)
            .await?
            .ok_or("homeCore lost state after reopen")?;
        assert_eq!(
            item.attributes["state"],
            serde_json::json!((i + 1).to_string())
        );
        recovered += 1;
    }
    assert_eq!(recovered, COUNT);
    println!(
        "{}",
        serde_json::json!({"backend":"homecore_io_redb_reopen","recovered":recovered,"expected":COUNT,"note":"clean reopen; crash/fsync power-loss was NOT tested"})
    );
    Ok(())
}
