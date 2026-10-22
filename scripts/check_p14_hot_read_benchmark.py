#!/usr/bin/env python3
"""P14: strict schema + scope classification for genuine hot-state read benchmark."""
import json
import math
from pathlib import Path
import statistics
import sys

def validate(report, backend):
    if not isinstance(report,dict):
        raise ValueError("expected object")
    expected={
        "schema_version":1,"backend":backend,"scope":"hot_in_memory_state_read",
        "entities":1232, "warmup_rounds":5
    }
    for key,value in expected.items():
        if report.get(key)!=value: raise ValueError("incorrect "+key)
    values=report.get("round_ns")
    if not isinstance(values,list) or len(values)!=30:
        raise ValueError("require exactly 30 measured, non-warmup rounds")
    if any(type(v) is not int or v<=0 or v>120_000_000_000 for v in values):
        raise ValueError("malformed or implausible nanoseconds")
    for key in ("rss_kb","pss_kb"):
        val=report.get(key)
        if type(val) is not int or val<=0 or val>100_000_000:
            raise ValueError("invalid "+key)
    if report.get("durable") is not False: raise ValueError("not in-memory only")
    return values

def compare(core,rust):
    a=validate(core,"official_core_python")
    b=validate(rust,"moli_native_rust")
    med_core=statistics.median(a)
    med_rust=statistics.median(b)
    ratio=med_core/med_rust
    if not math.isfinite(ratio) or ratio<=0: raise ValueError("non-finite ratio")
    def summary(values):
        ordered=sorted(values)
        return {
            "median_batch_ns":statistics.median(values),
            "p95_batch_ns":ordered[28],
            "p99_batch_ns":ordered[29],
            "median_ns_per_read":round(statistics.median(values)/1232,3)
        }
    return {
        "schema_version":1,
        "classification":"LIMITED_MICROBENCHMARK_NOT_FULL_STACK",
        "claim_full_haos_speedup":False,
        "scope":"single-process hot-state read; no persistence, events, apps, auth, IO or transport",
        "entities":1232,"rounds":30,"warmup_rounds":5,
        "core":summary(a),
        "rust":summary(b),
        "core_median_divided_by_rust_median_hot_read_only":round(ratio,3),
        "python_rss_kb":core["rss_kb"],
        "python_pss_kb":core["pss_kb"],
        "rust_rss_kb":rust["rss_kb"],
        "rust_pss_kb":rust["pss_kb"],
        "interpretation":"Moli numeric Value::Int and Core State.state string have different APIs and semantics; runner performance noisy. Not a Home Assistant replacement speed comparison."
    }

def main(argv):
    if len(argv)!=4:
        raise SystemExit("usage: check_p14_hot_read_benchmark.py core.json rust.json summary.json")
    source_a=Path(argv[1])
    source_b=Path(argv[2])
    target=Path(argv[3])
    summary=compare(json.loads(source_a.read_text()),json.loads(source_b.read_text()))
    target.write_text(json.dumps(summary,indent=2,sort_keys=True)+"\n")
    print(json.dumps(summary,sort_keys=True))
if __name__=="__main__":
    main(sys.argv)
