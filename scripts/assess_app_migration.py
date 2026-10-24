#!/usr/bin/env python3
"""Refuse to certify HAOS App migration without per-App evidence.

Only sanctioned sanitized inventory and published mapping are read. This tool
never accesses a Docker socket, a Proxmox VM, an App, or a backup password.
"""
from __future__ import annotations
import argparse
import json
import re
from pathlib import Path

ROOT=Path(__file__).resolve().parent.parent
MANDATORY_CHECKS={"app_data_restore","ui_ingress","rollback","resource_budget","permissions","version_pinning"}
MAX_APPS=50_000
MAX_REQUIRED_CHECKS=64
VALID_SLUG=re.compile(r"[a-z0-9][a-z0-9_-]{0,119}\Z",re.ASCII)

def checked_slug(value:object)->str:
    if not isinstance(value,str) or not VALID_SLUG.fullmatch(value):
        raise GateFailure("unsafe or unsupported App slug")
    return value
class GateFailure(ValueError): pass

def assess(baseline:dict,policy:dict)->dict:
    if not isinstance(baseline,dict) or not isinstance(policy,dict):
        raise GateFailure("invalid schema")
    if not isinstance(baseline.get("apps"),list) or not isinstance(policy.get("apps"),list):
        raise GateFailure("invalid schema")
    if len(baseline["apps"])>MAX_APPS or len(policy["apps"])>MAX_APPS:
        raise GateFailure("too many Apps to assess safely")
    installed={}
    for v in baseline["apps"]:
        if not isinstance(v,dict):
            raise GateFailure("invalid HAOS baseline App")
        slug=checked_slug(v.get("slug"))
        if slug in installed: raise GateFailure("duplicate baseline slug")
        installed[slug]=v
    plans={}
    for v in policy["apps"]:
        if not isinstance(v,dict):
            raise GateFailure("invalid App migration policy")
        slug=checked_slug(v.get("slug"))
        if slug in plans: raise GateFailure("duplicate policy slug")
        if v.get("qualification") not in ("blocked","lab_qualified","production_qualified"):
            raise GateFailure("unknown app qualification status")
        checks=v.get("required_checks")
        if (not isinstance(checks,list) or not checks or len(checks)>MAX_REQUIRED_CHECKS
            or not all(isinstance(s,str) and 0<len(s)<=120 and s.isascii() for s in checks)
            or len(set(checks))!=len(checks)):
            raise GateFailure("App missing/invalid acceptance checks")
        evidence=v.get("evidence")
        if not isinstance(evidence,list) or len(evidence)>256 or not all(isinstance(e,dict) for e in evidence):
            raise GateFailure("App evidence missing or invalid")
        if v["qualification"]=="production_qualified" and not evidence:
            raise GateFailure("unsupported production claim without evidence")
        plans[slug]=v
    if installed.keys()!=plans.keys():
        raise GateFailure("App policy does not exactly match HAOS inventory")
    # Accept any vendor and any inventory size, including an empty host.
    # An empty host must NEVER be interpreted as automatically qualified.
    ready={s for s,p in plans.items() if p["qualification"]=="production_qualified"}
    return {
        "result":"requires_independent_e2e_certification" if installed and len(ready)==len(installed) else "blocked",
        "source":"caller_provided_sanitized_inventory_and_policy_not_verified_live",
        "total_apps":len(installed),
        "qualified_apps":len(ready),
        "pending_apps":len(installed)-len(ready),
        "critical_pending":sum(p.get("risk")=="critical" and s not in ready for s,p in plans.items()),
        "missing_policy_entries":0,
        "has_device_data_or_secrets":False,
        "migration_performed":False,
        "operations_executed":0,
        "required_platform_checks":sorted(MANDATORY_CHECKS),
        "note":"Any App count is supported. Static per-App claims alone never certify backup, restore, migration or entire HAOS/HACS production",
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
