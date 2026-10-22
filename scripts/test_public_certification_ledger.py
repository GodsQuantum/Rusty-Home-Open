#!/usr/bin/env python3
"""CI check that release notes never claim unearned gate certification."""
import json
from pathlib import Path
root=Path(__file__).resolve().parent.parent
gates=json.loads((root/"config/release-gates.json").read_text())["required_gates"]
report=(root/"docs/CERTIFICATION_LEDGER.md").read_text()
assert len(gates)==15 and all(g["status"] in ("blocked","lab_partial") for g in gates)
assert "NOT_PRODUCTION_READY" in report and "0 / 15" in report
assert "not a certification score" in report.lower()
assert "not 29/29 certification" in report
tick=chr(96)
for gate in gates:
    marker="| "+tick+gate["id"]+tick+" | "+tick+gate["status"]+tick+" |"
    assert report.count(marker)==1,(gate["id"],report.count(marker))
assert sum(line.startswith("| "+tick) for line in report.splitlines())==15
print("PASS: exact 15 gates in ledger, 0 certified; limited lab evidence only")
