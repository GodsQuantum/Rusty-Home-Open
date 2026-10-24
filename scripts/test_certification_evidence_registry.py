#!/usr/bin/env python3
"""Fail-closed consistency check for historical HAOS evidence (NO network or writes).

The registry is a scope-bounded evidence index, NOT a release certificate.
Existing proofs cannot vanish from the project without failing CI.
"""
from __future__ import annotations
import ast
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
REGISTRY = ROOT / "config/certification-evidence-registry.json"
SOURCE = ROOT / "scripts/p8c2_real_haos_supervisor_pytest.py"
WORKFLOW = ROOT / ".github/workflows/p8c2-haos-qemu.yml"
JUNIT = ROOT / "scripts/check_p8d_junit.py"

def main() -> None:
    entry = json.loads(REGISTRY.read_text(encoding="utf-8"))
    assert entry["schema_version"] == 1
    release = entry["production_release"]
    assert release == {
        "state": "NOT_PRODUCTION_READY",
        "certified_gates": 0,
        "required_gates": 15,
    }
    gates = json.loads((ROOT / "config/release-gates.json").read_text())["required_gates"]
    assert len(gates) == 15
    assert all(row["status"] in ("blocked", "lab_partial") for row in gates)

    proof = entry["verification"]
    assert proof["junit_case_count"] == 17 and proof["all_junit_passed"] is True
    assert proof["conclusion"] == "success"
    assert re.fullmatch("[0-9a-f]{40}", proof["run_sha"])
    assert re.fullmatch("[0-9a-f]{40}", proof["git_main_when_recorded"])
    assert re.fullmatch(
        r"https://github\.com/GodsQuantum/Rusty-Home-Open/actions/runs/[0-9]+",
        proof["run_url"],
    )
    assert "same VM" in proof["environment"]

    tests_source = SOURCE.read_text(encoding="utf-8")
    definitions = {
        node.name for node in ast.parse(tests_source).body
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef))
    }
    ci = WORKFLOW.read_text(encoding="utf-8")
    junit = JUNIT.read_text(encoding="utf-8")
    cases = entry["proven_cases"]
    assert len(cases) >= 8
    ids: set[str] = set()
    for row in cases:
        case_id = row["id"]
        assert re.fullmatch(r"[a-z0-9]+(?:-[a-z0-9]+)*", case_id)
        assert case_id not in ids, ("duplicate evidence id", case_id)
        ids.add(case_id)
        test = row["test"]
        assert test in definitions, (case_id, "test function was removed")
        assert f'supervisor_test/test_supervisor.py::{test}' in ci, (
            case_id, "test not selected by real HAOS CI"
        )
        assert f'"{test}"' in junit, (case_id, "JUnit gate omits test")
        assert row["marker"] in tests_source, (case_id, "proof marker no longer in source")
        assert row["evidence_url"] == proof["run_url"]
        assert row["status"] == "verified_in_disposable_qemu"
        assert len(row["claim"]) > 30 and len(row["scope_limit"]) > 30
    required = {
        "haos-full-backup",
        "haos-full-restore-same-guest",
        "haos-encrypted-full-backup-restore",
        "haos-wrong-password",
        "haos-real-file-editor-partial-data",
        "haos-versioned-app-rollback",
        "haos-sync-install-release",
        "haos-async-expired-job-fence",
    }
    assert required <= ids, ("previously certified lab evidence disappeared", required - ids)

    absent = {item["id"]: item for item in entry["not_yet_proven"]}
    assert absent["generic-cross-vendor-data-migration"]["status"] == "not_verified"
    assert absent["production-full-device-restore"]["status"] == "not_verified"
    assert "full-restore-fresh-second-machine" not in absent
    second = entry["cross_vm_verification"]
    assert second["id"] == "haos-full-restore-fresh-second-vm"
    assert second["status"] == "verified_in_two_disposable_qemu_guests"
    assert second["conclusion"] == "success"
    assert re.fullmatch("[0-9a-f]{40}", second["run_sha"])
    assert second["run_sha"] != proof["run_sha"]
    assert second["source_junit_passed"] == second["target_junit_passed"] == 3
    assert second["run_url"] == "https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40135221183"
    assert second["run_sha"] == "8d3b1263f0cb65f11fdbb9f3979a58af052b3731"
    guest = (ROOT / "scripts/p8p_cross_vm_backup_pytest.py").read_text(encoding="utf-8")
    qemu = (ROOT / ".github/workflows/p8p-cross-vm-recovery.yml").read_text(encoding="utf-8")
    strict = (ROOT / "scripts/check_p8p_cross_vm_junit.py").read_text(encoding="utf-8")
    assert "boot!=target_boot" in guest and "sha256sum " in guest
    assert qemu.count("./run_tests.sh ") == 2
    for phase in ("source", "target"):
        assert second[phase + "_test"] in guest
        assert second[phase + "_test"] in qemu
        assert 'junit_reports/tests.xml ' + phase in qemu
        assert phase in strict
    for marker in second["markers"]:
        assert marker in qemu or marker in strict, marker
    assert "NOT migration to Rusty Home" in second["scope_limit"]
    summary = (ROOT / "docs/CERTIFICATION_EVIDENCE_REGISTER.md").read_text(encoding="utf-8")
    journal = (ROOT / "docs/WORK_LOG.md").read_text(encoding="utf-8")
    for item in ids:
        assert f"`{item}`" in summary, ("missing from user-facing index", item)
    assert "même VM" in summary and "NOT_PRODUCTION_READY" in summary
    assert "avant" in journal and "40127707580" in journal
    for md in ("README.md", "README.fr.md", "docs/RELEASE_STATUS.md", "docs/CERTIFICATION_LEDGER.md"):
        assert "CERTIFICATION_EVIDENCE_REGISTER.md" in (ROOT / md).read_text(), md
    print(f"PASS: {len(cases)} historic SAME-VM proofs plus separate fresh second-VM 3/3+3/3 "
          "real QEMU recovery; generic Rust migration NOT verified, 0/15 production gates")

if __name__ == "__main__":
    main()
