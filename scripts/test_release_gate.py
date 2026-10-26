#!/usr/bin/env python3
"""Negative tests for fail-closed production gate. No service is deployed."""
from __future__ import annotations
import copy
import json
from pathlib import Path
from check_release_gate import audit, InvalidGate, individual_gate_matrix

ROOT=Path(__file__).resolve().parent.parent
cfg=json.loads((ROOT/"config/release-gates.json").read_text())
baseline=audit(cfg)
assert baseline["status"]=="NOT_PRODUCTION_READY",baseline
assert baseline["blocking_gates"]==15,baseline
assert baseline["progress"]=={"blocked":6,"lab_partial":9,"certified":0},baseline
print("PASS: all 15 critical technical gates block production")

a=copy.deepcopy(cfg)
a["required_gates"][0]["status"]="certified"
assert audit(a)["blocking_gates"]==15
print("PASS: changing status to certified without evidence cannot release")

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
print("PASS: all manually certified statuses with non-existent evidence are blocked")

a=copy.deepcopy(cfg)
a["required_gates"][0]["status"]="green-unknown"
try: audit(a)
except InvalidGate: pass
else: raise AssertionError("unknown gate state accepted")
print("PASS: missing/duplicate/invalid proofs rejected")

# Matrix is a transparent per-criterion verdict; it may never equate CI/lab to release.
gates=individual_gate_matrix(cfg,baseline)
assert len(gates)==15 and len({g["gate_id"] for g in gates})==15
assert sum(g["laboratory_status"]=="lab_partial" for g in gates)==9
assert all(g["production_status"]=="blocked" and not g["independent_proof_accepted"] for g in gates)
assert all(len(g["mandatory_production_scenarios"])>=4 and g["remaining_blockers"] for g in gates)
forged=copy.deepcopy(cfg)
forged["required_gates"][0]["status"]="certified"
assert individual_gate_matrix(forged,audit(forged))[0]["production_status"]=="blocked"
print("PASS: all 15 individual certification verdicts preserve fail-closed release")
