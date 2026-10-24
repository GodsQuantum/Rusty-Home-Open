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

# Vendor-neutral compatibility is NOT constrained to the nine fictional
# examples in this repository. No accepted static audit permits migration.
def generated(count):
    slugs=[f"future_vendor_{i:05d}" for i in range(count)]
    baseline={"source":"sanitized_third_party_fixture",
              "apps":[{"slug":s, "version":"1.0"} for s in slugs]}
    policy={"apps":[{"slug":s, "qualification":"blocked", "risk":"critical" if i==0 else "low",
                     "required_checks":["app_data_restore", "rollback", "permissions"],
                     "evidence":[]} for i,s in enumerate(slugs)]}
    return baseline,policy

for size in (0,1,2,9,10,50,1000):
    a,b=generated(size)
    result=assess(a,b)
    assert result["total_apps"]==size and result["pending_apps"]==size,result
    assert result["result"]=="blocked"
    assert result["source"]=="caller_provided_sanitized_inventory_and_policy_not_verified_live"
    if size:
        b["apps"][0]["qualification"]="production_qualified"
        b["apps"][0]["evidence"]=[{"lab_fixture":"never production proof"}]
        partly=assess(a,b)
        assert partly["qualified_apps"]==1
        expected_status = "requires_independent_e2e_certification" if size==1 else "blocked"
        assert partly["result"]==expected_status, partly
        for entry in b["apps"]:
            entry["qualification"]="production_qualified"
            entry["evidence"]=[{"lab_fixture":"never production proof"}]
        claimed=assess(a,b)
        assert claimed["result"]=="requires_independent_e2e_certification"
        assert claimed["migration_performed"] is False and claimed["operations_executed"]==0
        assert claimed["has_device_data_or_secrets"] is False
print("PASS: empty, single, nine, ten, fifty, thousand arbitrary-vendor inventories without allowlists")

for malicious in ("../escape","Core_ssh","a/b","a\\b","x y","x"*121,".hidden","-hyphen", "", "équipement"):
    a,b=generated(1)
    a["apps"][0]["slug"]=malicious
    b["apps"][0]["slug"]=malicious
    rejects(a,b)
a,b=generated(1)
b["apps"][0]["required_checks"]=["rollback","rollback"]
rejects(a,b)
a,b=generated(1)
b["apps"][0]["evidence"]=["not an auditable object"]
rejects(a,b)
a,b=generated(1)
b["apps"][0]["required_checks"]=["x"*121]
rejects(a,b)
a,b=generated(1)
a["apps"].extend([{"slug":"u"}]*50000)
rejects(a,b)
for item in (None,[],42):
    try: assess(item,{"apps":[]})
    except GateFailure: pass
    else: raise AssertionError("non-object inventory accepted")
print("PASS: bounded dynamic inventory validation refuses duplicates, unsafe paths and malformed metadata")
