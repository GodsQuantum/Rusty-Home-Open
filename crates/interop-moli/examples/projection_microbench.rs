//! Self-contained synthetic Moli type-projection microbenchmark.
//! NOT a native engine speed comparison; it excludes real HA integration work.
use hars_interop_moli::{HaEntity, project};
use serde_json::{Map, json};
use std::{hint::black_box, time::Instant};

fn main() {
    let items: Vec<HaEntity> = (0..1232)
        .map(|i| HaEntity {
            entity_id: format!("sensor.mock_{i}"),
            platform: "synthetic".to_string(),
            config_entry_id: Some("mock-config".to_string()),
            unique_id: Some(format!("id/group/{i}")),
            state: json!(i % 100),
            attributes: Map::from_iter([("unit".to_string(), json!("C"))]),
        })
        .collect();
    let mut per_item = Vec::new();
    // Warm up exactly the same path as each measurement.
    for round in 0..25 {
        let start = Instant::now();
        for x in &items {
            let converted = project(black_box(x)).expect("fixture valid");
            assert!(converted.stable_identity);
            black_box(converted);
        }
        if round >= 5 {
            per_item.push(start.elapsed().as_nanos() as f64 / items.len() as f64);
        }
    }
    per_item.sort_by(f64::total_cmp);
    let rss_kib: Option<u64> = std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|x| x.starts_with("VmHWM:"))?
                .split_whitespace()
                .nth(1)?
                .parse()
                .ok()
        });
    println!(
        "{}",
        serde_json::json!({
            "kind": "adapter_microbenchmark",
            "note": "single-process, synthetic HA identity projection only; no HA Core/Apps workload",
            "source": "moli-core d54c3de + HARS experimental adapter",
            "entities": items.len(),
            "warmup_batches": 5,
            "measured_batches": per_item.len(),
            "p50_ns_per_entity": per_item[per_item.len()/2].round() as u64,
            "p95_ns_per_entity": per_item[(per_item.len()*95/100).min(per_item.len()-1)].round() as u64,
            "peak_process_rss_kib": rss_kib
        })
    );
}
