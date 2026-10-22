#!/usr/bin/env python3
"""Refuse to certify HAOS App migration without per-App evidence.

Only sanctioned sanitized inventory and published mapping are read. This tool
never accesses a Docker socket, a Proxmox VM, an App, or a backup password.
"""
from __future__ import annotations
import argparse
import json
from pathlib import Path

ROOT=Path(__file__).resolve().parent.parent
MANDATORY_CHECKS={"app_data_restore","ui_ingress","rollback","resource_budget","permissions","version_pinning"}
class GateFailure(ValueError): pass

def assess(baseline:dict,policy:dict)->dict:
    if not isinstance(baseline.get("apps"),list) or not isinstance(policy.get("apps"),list):
        raise GateFailure("invalid schema")
    installed={}
    for v in baseline["apps"]:
        if not isinstance(v,dict) or not isinstance(v.get("slug"),str):
            raise GateFailure("invalid HAOS baseline App")
        if v["slug"] in installed: raise GateFailure("duplicate baseline slug")
        installed[v["slug"]]=v
    plans={}
    for v in policy["apps"]:
        if not isinstance(v,dict) or not isinstance(v.get("slug"),str):
            raise GateFailure("invalid App migration policy")
        if v["slug"] in plans: raise GateFailure("duplicate policy slug")
        if v.get("qualification") not in ("blocked","lab_qualified","production_qualified"):
            raise GateFailure("unknown app qualification status")
        checks=v.get("required_checks")
        if not isinstance(checks,list) or not checks or not all(isinstance(s,str) and s for s in checks):
            raise GateFailure("App missing acceptance checks")
        evidence=v.get("evidence")
        if not isinstance(evidence,list):
            raise GateFailure("App evidence missing")
        if v["qualification"]=="production_qualified" and not evidence:
            raise GateFailure("unsupported production claim without evidence")
        plans[v["slug"]]=v
    if installed.keys()!=plans.keys():
        raise GateFailure("App policy does not exactly match HAOS inventory")
    if len(installed)!=9:
        raise GateFailure("sample lab host baseline changed; explicit revised audit required")
    ready={s for s,p in plans.items() if p["qualification"]=="production_qualified"}
    return {
        "result":"requires_independent_e2e_certification" if len(ready)==len(installed) else "blocked",
        "source":"sanitized HAOS synthetic VM audit",
        "total_apps":len(installed),
        "qualified_apps":len(ready),
        "pending_apps":len(installed)-len(ready),
        "critical_pending":sum(p.get("risk")=="critical" and s not in ready for s,p in plans.items()),
        "missing_policy_entries":0,
        "has_device_data_or_secrets":False,
        "migration_performed":False,
        "operations_executed":0,
        "required_platform_checks":sorted(MANDATORY_CHECKS),
        "note":"Per-App qualification alone does not certify entire HAOS/HACS migration",
    }

def main()->None:
    ap=argparse.ArgumentParser()
    ap.add_argument("--baseline",type=Path,default=ROOT/"audits/haos-apps.json")
    ap.add_argument("--policy",type=Path,default=ROOT/"config/app-migration-policy.json")
    ap.add_argument("--assert-production",action="store_true")
    args=ap.parse_args()
    result=assess(json.loads(args.baseline.read_text()),json.loads(args.policy.read_text()))
    print(json.dumps(result,sort_keys=True,indent=2))
    if args.assert_production:
        raise SystemExit("BLOCKED: this static policy checker is not an end-to-end production certification")

if __name__=="__main__":
    main()
