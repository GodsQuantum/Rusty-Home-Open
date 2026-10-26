#!/usr/bin/env python3
"""Fail-closed release-gate negative controls that permit signed, scoped progress."""
from __future__ import annotations
import copy
import json
from pathlib import Path
from check_release_gate import audit, InvalidGate, individual_gate_matrix
ROOT=Path(__file__).resolve().parent.parent
cfg=json.loads((ROOT/"config/release-gates.json").read_text())
baseline=audit(cfg)
assert baseline["total_gates"]==15
assert baseline["blocking_gates"]+baseline["progress"]["certified"]==15
assert not baseline["deployment_authorized"] and not baseline["independent_release_signoff"]
assert all(g["status"]!="certified" or g["id"] not in baseline["blocked_ids"] for g in cfg["required_gates"])
print("PASS: release requires accepted independent evidence, not colored CI status")

forged=copy.deepcopy(cfg)
forged["required_gates"][0]["status"]="certified"
forged["required_gates"][0]["certification_artifact"]=None
report=audit(forged)
assert forged["required_gates"][0]["id"] in report["blocked_ids"]
assert report["progress"]["certified"]<=baseline["progress"]["certified"]
print("PASS: claimed certification without exact verified artifact is denied")

a=copy.deepcopy(cfg)
a["required_gates"].append(copy.deepcopy(a["required_gates"][0]))
try: audit(a)
except InvalidGate: pass
else: raise AssertionError("duplicate release gate accepted")
a=copy.deepcopy(cfg)
a["required_gates"].pop()
try: audit(a)
except InvalidGate: pass
else: raise AssertionError("missing release gate accepted")
a=copy.deepcopy(cfg)
for item in a["required_gates"]:
    item["status"]="certified"
    item["certification_artifact"]={"name":"not-a-file.json","sha256":"0"*64}
assert audit(a)["status"]=="NOT_PRODUCTION_READY"
assert audit(a)["progress"]["certified"]==0
print("PASS: all fabricated certs, malformed or missing proofs rejected")
a=copy.deepcopy(cfg)
a["required_gates"][0]["status"]="green-unknown"
try: audit(a)
except InvalidGate: pass
else: raise AssertionError("unknown gate state accepted")

rows=individual_gate_matrix(cfg,baseline)
assert len(rows)==15 and len({g["gate_id"] for g in rows})==15
assert all(len(g["mandatory_production_scenarios"])>=4 for g in rows)
assert {g["gate_id"] for g in rows if g["production_status"]=="blocked"}==set(baseline["blocked_ids"])
assert all(g["independent_proof_accepted"]==(g["production_status"]=="certified") for g in rows)
assert all(g["remaining_blockers"] for g in rows if g["production_status"]=="blocked")
assert individual_gate_matrix(forged,report)[0]["production_status"]=="blocked"
print("PASS: independent per-criterion matrix accepts real signed progress but refuses forged status")
