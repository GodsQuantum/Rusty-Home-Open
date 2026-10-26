#!/usr/bin/env python3
"""Machine-readable release gate, fail closed until independent end-to-end proof.

Not a software attestation service: publishing still requires human approval,
CI provenance, threat modeling, reproducible build and deployment tests.
"""
from __future__ import annotations
import argparse
import json
from pathlib import Path
from release_attestation import verified_release_artifact, REQUIRED_SCENARIOS

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
    observed={"blocked":0,"lab_partial":0,"certified":0}
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
            observed[status]+=1
            blocked.append(key)
            continue
        # A local SHA-256 never proves an external run or independent review.
        # Require strict scoped evidence and two distinct registered Ed25519
        # reviewers over exact bytes before recognizing production evidence.
        if not verified_release_artifact(root,key,proof):
            observed["blocked"]+=1
            blocked.append(key)
            continue
        observed["certified"]+=1
    if seen!=EXPECTED_GATES:
        raise InvalidGate("release gate manifest schema mismatch")
    # This reports technical preconditions, NOT authorization to publish:
    # live deployment signoff and rollback approval must be separate.
    return {
        "status":"technical_evidence_complete_pending_release_signoff" if not blocked else "NOT_PRODUCTION_READY",
        "total_gates":len(seen),
        "progress":observed,
        "blocking_gates":len(blocked),
        "blocked_ids":sorted(blocked),
        "independent_release_signoff":False,
        "deployment_authorized":False,
    }

def individual_gate_matrix(config: dict, report: dict)->list[dict]:
    """Per-gate evidence, not a proxy for release approval or a CI pass count."""
    blocked=set(report["blocked_ids"])
    return [{
        "gate_id":gate["id"],
        "laboratory_status":gate["status"],
        "production_status":("certified" if gate["id"] not in blocked else "blocked"),
        "independent_proof_accepted":gate["id"] not in blocked,
        "mandatory_production_scenarios":sorted(REQUIRED_SCENARIOS[gate["id"]]),
        "remaining_blockers":gate.get("blocked_reason",[]),
    } for gate in config["required_gates"]]

def main()->None:
    ap=argparse.ArgumentParser()
    ap.add_argument("--config",type=Path,default=ROOT/"config/release-gates.json")
    ap.add_argument("--expect-blocked",action="store_true")
    ap.add_argument("--assert-ready",action="store_true")
    ap.add_argument("--matrix",action="store_true",help="show all 15 individual production gates and blockers")
    args=ap.parse_args()
    manifest=json.loads(args.config.read_text())
    report=audit(manifest)
    result={"summary":report,"gates":individual_gate_matrix(manifest,report)} if args.matrix else report
    print(json.dumps(result,indent=2,sort_keys=True))
    if args.expect_blocked and report["status"]!="NOT_PRODUCTION_READY":
        raise SystemExit("FAIL: release-gate guard unexpectedly cleared")
    if args.assert_ready:
        raise SystemExit("BLOCKED: independent release signoff and deployment approval are not implemented")

if __name__=="__main__":
    main()
