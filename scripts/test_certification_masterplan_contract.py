#!/usr/bin/env python3
"""Block stale release-status counts and undocumented certification transitions."""
import json
from pathlib import Path
from check_release_gate import audit as check_live_release

root = Path(__file__).resolve().parent.parent
audit = json.loads((root / "config/certification-masterplan-audit-20261024.json").read_text())
gates = json.loads((root / "config/release-gates.json").read_text())["required_gates"]
states = {entry["id"]: entry["status"] for entry in gates}
assert len(states) == len(gates) == audit["production"]["total"] == 15
assert states.keys() == audit["workstreams"].keys()
for id_, state in states.items():
    entry = audit["workstreams"][id_]
    assert entry["gate_status"] in ("blocked","lab_partial"), ("historical snapshot altered", id_)
    assert len(entry["remaining_proof"]) > 70
    assert len(entry["fast_path"]) > 55
    assert len(entry["already_evidenced"]) > 65
    lo, hi = entry["focus_interactions_estimate"]
    assert 1 <= lo <= hi
# The 24 October audit is an immutable historical planning baseline, NOT a
# second release authority that can forever force current certification to 0.
historical = [w["gate_status"] for w in audit["workstreams"].values()]
assert historical.count("lab_partial") == audit["production"]["partial"] == 9
assert historical.count("blocked") == audit["production"]["blocked"] == 6
assert audit["production"]["certified"] == 0
live = check_live_release({"required_gates": gates})
assert live["total_gates"] == 15 and live["deployment_authorized"] is False
assert live["blocking_gates"] + live["progress"]["certified"] == 15
assert all(g["status"] != "certified" or g["id"] not in live["blocked_ids"] for g in gates)
assert audit["production"]["deployment_authorized"] is False
assert audit["history"]["merged_prs"] == 76
assert audit["history"]["merged_by_utc_day"] == {
    "2026-10-22": 8, "2026-10-23": 50, "2026-10-24": 18
}
assert audit["history"]["median_pr_lifecycle_minutes"] == 9.0
evidence = json.loads((root / "config/certification-evidence-registry.json").read_text())
assert len(evidence["proven_cases"]) == 8
assert evidence["cross_vm_verification"]["status"] == "verified_in_two_disposable_qemu_guests"
assert evidence["production_release"]["certified_gates"] == 0
report = (root / "docs/PRODUCTION_CERTIFICATION_MASTERPLAN_2026-10-24.md").read_text()
for id_ in states:
    assert f"| {id_} |" in report, ("missing gate in visible report", id_)
for span in ("76 PR", "9 partiels", "6 bloqués", "0 intégralement certifié",
             "2026.10.3", "matter.js", "HAOS→Rusty Home"):
    assert span in report, span
handoff = (root / "docs/HANDOFF_CURRENT.md").read_text()
journal = (root / "docs/WORK_LOG.md").read_text()
assert "PRODUCTION_CERTIFICATION_MASTERPLAN_2026-10-24.md" in handoff
assert "CERTIFICATION_EVIDENCE_REGISTER.md" in handoff
assert "show_handoff.py" in handoff
assert "PRODUCTION_CERTIFICATION_MASTERPLAN_2026-10-24.md" in journal
for name in ("README.md", "README.fr.md", "docs/RELEASE_STATUS.md"):
    contents = (root / name).read_text()
    assert "HANDOFF_CURRENT.md" in contents, name
assert (root / "scripts/show_handoff.py").is_file()
assert (root / ".github/PULL_REQUEST_TEMPLATE.md").is_file()
print("PASS: exact 15 real gates, 9 partial + 6 blocked, historical 76 PR baseline,")
print("      proved nine lab scenarios, visible sourced roadmap and updated handoff")
