#!/usr/bin/env python3
"""Fail-closed anti-duplication ledger for distinct Core -> Rust/Moli lab claims.

Offline contract: does not pretend to re-run historical jobs or validate
production readiness. Exact CI SHA + URLs are reviewed when evidence is added.
"""
from pathlib import Path
import json
import re

ROOT = Path(__file__).resolve().parent.parent
ledger = json.loads((ROOT / "config/native-migration-evidence.json").read_text())
assert ledger["schema_version"] == 1
assert ledger["release"] == {
    "status": "NOT_PRODUCTION_READY",
    "certified_gates": 0,
    "required_gates": 15,
}
assert "HAOS-to-HAOS" in ledger["distinction"]
assert "no complete HAOS replacement" in ledger["distinction"]

required = {
    "core-registry-stage": 89,
    "core-registry-real-moli-hub": 90,
    "core-real-state-snapshot": 91,
    "loopback-event-convergence": 92,
    "bounded-ws-moli-runner": 93,
    "official-core-ws-to-moli": 94,
    "official-core-ws-moli-consistency": 95,
}
rows = ledger["proofs"]
assert len(rows) >= len(required)
seen = set()
ci_real = (ROOT / ".github/workflows/core-interop-public.yml").read_text()
ci_rust = (ROOT / ".github/workflows/ci.yml").read_text()
md = (ROOT / "docs/CERTIFICATION_EVIDENCE_REGISTER.md").read_text()

for proof in rows:
    id_ = proof["id"]
    assert id_ not in seen, id_
    seen.add(id_)
    assert re.fullmatch(r"[a-z0-9]+(?:-[a-z0-9]+)*", id_)
    assert isinstance(proof["pr"], int) and proof["pr"] >= 89
    assert all(re.fullmatch(r"[0-9a-f]{40}", proof[key])
               for key in ("head_sha", "merge_sha"))
    assert proof["head_sha"] != proof["merge_sha"]
    assert re.fullmatch(
        r"https://github\.com/GodsQuantum/Rusty-Home-Open/actions/runs/[0-9]+",
        proof["run_url"],
    )
    path = Path(proof["source_file"])
    assert not path.is_absolute() and ".." not in path.parts
    assert path.parts[0] in ("scripts", "crates")
    source = ROOT / path
    assert source.is_file() and not source.is_symlink()
    assert proof["marker"] in source.read_text(), (id_, "test marker disappeared")
    assert proof["status"] in (
        "verified_in_pinned_offline_core",
        "verified_in_synthetic_loopback_ws",
    )
    assert len(proof["claim"]) > 45 and len(proof["scope_limit"]) > 65
    assert id_ in md, (id_, "not in user-facing evidence index")
    if proof["pr"] in (89, 90, 91):
        assert "run_p8q_real_core_to_native_registry.sh" in ci_real
        assert proof["status"] == "verified_in_pinned_offline_core"
    elif proof["pr"] in (92, 93):
        assert "cargo test --locked -p hars-native-kernel --lib" in ci_rust
        assert proof["status"] == "verified_in_synthetic_loopback_ws"
    elif proof["pr"] in (94, 95):
        assert "run_p8v_genuine_core_ws_to_moli.sh" in ci_real
        assert proof["status"] == "verified_in_pinned_offline_core"

assert required.keys() <= seen
for proof in rows:
    if proof["id"] in required:
        assert proof["pr"] == required[proof["id"]]
assert "0/15" in md
assert "ne valide PAS" in md
assert "NOT_PRODUCTION_READY" in md
print(f"PASS: {len(rows)} distinct, SHA-pinned Core-to-native lab proofs indexed;")
print("      HAOS-to-HAOS evidence retained separately, deployment 0/15 denied")
