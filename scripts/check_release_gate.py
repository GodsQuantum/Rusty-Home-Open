#!/usr/bin/env python3
"""Machine-readable release gate, fail closed until independent end-to-end proof.

Not a software attestation service: publishing still requires human approval,
CI provenance, threat modeling, reproducible build and deployment tests.
"""
from __future__ import annotations
import argparse
import hashlib
import json
from pathlib import Path
import re

ROOT=Path(__file__).resolve().parent.parent
EXPECTED_GATES=frozenset((
    "authenticated_host_transport","ha_core_hacs_29_runtime","hacs_github_oauth",
    "apps_9_lifecycle","supervisor_apps_restore","haos_backup_restore",
    "rollback_verified","single_radio_owner","ha_ws_reconnection",
    "production_ui_auth","native_drivers_services","performance_full_stack",
    "packaging_updates","third_party_licenses","security_stress_fuzz"))
class InvalidGate(ValueError): pass

def audit(config: dict, root: Path=ROOT)->dict:
    rows=config.get("required_gates")
    if not isinstance(rows,list):
        raise InvalidGate("missing release gates")
    seen=set()
    blocked=[]
    for gate in rows:
        if not isinstance(gate,dict) or not isinstance(gate.get("id"),str):
            raise InvalidGate("invalid gate record")
        key=gate["id"]
        if key in seen: raise InvalidGate("duplicate release gate")
        seen.add(key)
        status=gate.get("status")
        if status not in ("blocked","lab_partial","certified"):
            raise InvalidGate("invalid status")
        proof=gate.get("certification_artifact")
        if status!="certified":
            blocked.append(key)
            continue
        # A status toggle is NOT enough: each certified gate needs a pinned
        # immutable evidence file under project docs/certifications, and hash.
        if not isinstance(proof,dict):
            blocked.append(key);continue
        name=proof.get("name")
        digest=proof.get("sha256")
        if (not isinstance(name,str)
            or not re.fullmatch(r"[a-zA-Z0-9][a-zA-Z0-9_.-]*\.json",name)
            or not isinstance(digest,str)
            or not re.fullmatch(r"[0-9a-f]{64}",digest)):
            blocked.append(key);continue
        path=root/"docs/certifications"/name
        if path.is_symlink() or not path.is_file():
            blocked.append(key);continue
        if hashlib.sha256(path.read_bytes()).hexdigest()!=digest:
            blocked.append(key);continue
    if seen!=EXPECTED_GATES:
        raise InvalidGate("release gate manifest schema mismatch")
    # This reports technical preconditions, NOT authorization to publish:
    # live deployment signoff and rollback approval must be separate.
    return {
        "status":"technical_evidence_complete_pending_release_signoff" if not blocked else "NOT_PRODUCTION_READY",
        "total_gates":len(seen),
        "blocking_gates":len(blocked),
        "blocked_ids":sorted(blocked),
        "independent_release_signoff":False,
        "deployment_authorized":False,
    }

def main()->None:
    ap=argparse.ArgumentParser()
    ap.add_argument("--config",type=Path,default=ROOT/"config/release-gates.json")
    ap.add_argument("--expect-blocked",action="store_true")
    ap.add_argument("--assert-ready",action="store_true")
    args=ap.parse_args()
    report=audit(json.loads(args.config.read_text()))
    print(json.dumps(report,indent=2,sort_keys=True))
    if args.expect_blocked and report["status"]!="NOT_PRODUCTION_READY":
        raise SystemExit("FAIL: release-gate guard unexpectedly cleared")
    if args.assert_ready:
        raise SystemExit("BLOCKED: independent release signoff and deployment approval are not implemented")

if __name__=="__main__":
    main()
