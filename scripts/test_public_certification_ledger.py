#!/usr/bin/env python3
"""CI: exactly 15 documented release gates, never an unearned certificate.

The dated laboratory snapshots are historical; release gates may progress
one independently-signed criterion at a time without weakening verification.
"""
import json
from pathlib import Path
from check_release_gate import audit, individual_gate_matrix
root=Path(__file__).resolve().parent.parent
manifest=json.loads((root/"config/release-gates.json").read_text())
gates=manifest["required_gates"]
result=audit(manifest)
report=(root/"docs/CERTIFICATION_LEDGER.md").read_text()
assert len(gates)==result["total_gates"]==15
assert result["independent_release_signoff"] is False
assert result["deployment_authorized"] is False
assert result["progress"]["certified"] + result["blocking_gates"]==15
assert all(g["status"]!="certified" or g["id"] not in result["blocked_ids"] for g in gates), "unverified signed claims must not appear as certified"
assert f"**{result['progress']['certified']} / 15 mandatory production release gates certified.**" in report
assert "not a certification score" in report.lower()
assert "not 29/29 certification" in report
assert ("NOT_PRODUCTION_READY" if result["blocking_gates"] else "technical_evidence_complete_pending_release_signoff") in report
rows=individual_gate_matrix(manifest,result)
assert len(rows)==15 and len({g["gate_id"] for g in rows})==15
tick=chr(96)
for gate in gates:
    marker="| "+tick+gate["id"]+tick+" | "+tick+gate["status"]+tick+" |"
    assert report.count(marker)==1,(gate["id"],report.count(marker))
gate_section=report.split("## Release gate matrix",1)[1].split("## Effort estimates",1)[0]
assert sum(line.startswith("| "+tick) for line in gate_section.splitlines())==15
print("PASS: 15 source-backed individual gate verdicts, independently signed evidence required for each certified criterion")
