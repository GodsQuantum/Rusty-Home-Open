#!/usr/bin/env python3
"""Negative and positive static tests; no production App service is invoked."""
from __future__ import annotations
import copy
import json
from pathlib import Path
from assess_app_migration import GateFailure, assess

ROOT=Path(__file__).resolve().parent.parent
baseline=json.loads((ROOT/"audits/haos-apps.json").read_text())
plan=json.loads((ROOT/"config/app-migration-policy.json").read_text())
result=assess(baseline,plan)
assert result["result"]=="blocked",result
assert result["total_apps"]==9 and result["qualified_apps"]==0,result
assert result["critical_pending"]==3,result
print("PASS: all 9 HAOS Apps inventoried, critical owners blocked pending restore tests")

def rejects(a,b):
    try:
        assess(a,b)
    except GateFailure:
        return
    raise AssertionError("invalid App migration plan was accepted")

p=copy.deepcopy(plan)
p["apps"].pop()
rejects(baseline,p)
p=copy.deepcopy(plan)
p["apps"].append(copy.deepcopy(p["apps"][0]))
rejects(baseline,p)
p=copy.deepcopy(plan)
p["apps"][0]["qualification"]="production_qualified"
rejects(baseline,p)
p=copy.deepcopy(plan)
p["apps"][0]["qualification"]="random-undocumented-state"
rejects(baseline,p)
p=copy.deepcopy(plan)
p["apps"][0]["required_checks"]=[]
rejects(baseline,p)
p=copy.deepcopy(plan)
p["apps"][0]["slug"]="totally-unknown-app"
rejects(baseline,p)
print("PASS: missing/unknown/duplicate Apps and unsupported completion claims rejected")

all_qualified=copy.deepcopy(plan)
for entry in all_qualified["apps"]:
    entry["qualification"]="production_qualified"
    entry["evidence"]=[{"placeholder":"NOT REAL VALIDATION"}]
result=assess(baseline,all_qualified)
assert result["result"]=="requires_independent_e2e_certification",result
print("PASS: even all statically claimed-qualified Apps cannot authorize production")
