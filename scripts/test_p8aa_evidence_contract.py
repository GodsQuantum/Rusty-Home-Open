#!/usr/bin/env python3
"""Reject forged production status or missing P8aa genuine CI provenance."""
import json
import re
from pathlib import Path

root=Path(__file__).resolve().parent.parent
e=json.loads((root/"config/haos-native-backup-evidence.json").read_text())
g=json.loads((root/"config/release-gates.json").read_text())
assert e["schema_version"]==1
assert e["case_id"]=="p8aa-genuine-haos-supervisor-backup-native-registry-stage"
assert e["result"]=="verified_disposable_qemu_lab_only"
assert e["github_result"]=="success"
assert e["pull_request"]==99 and e["github_run"]==40238122942
assert e["run_url"]=="https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40238122942"
assert re.fullmatch(r"[a-f0-9]{40}",e["head_sha"])
assert e["head_sha"]=="9c6e3220f433153c0dc28ef0d2549bd52a345a8f"
assert e["merge_sha"]=="d297bbbd29ac313723b54794ad27aceaf0636166"
assert e["junit_tests"]=={"selected":3,"passed":3,"failed":0,"skipped":0}
assert e["successful_pr_checks"]==17 and e["skipped_pr_checks"]==1
assert e["certified_release_gates"]==0 and e["required_release_gates"]==15
assert e["production_release"]=="NOT_PRODUCTION_READY"
# Extra independently verified automation tests are append-only; the
# ORIGINAL 2026-10-25 P8aa receipt above is immutable.
new=e["additional_automation_cases"]
assert [x["id"] for x in new]==[
 "p8be-real-supervisor-native-source-custody",
 "p8bf-direct-second-haos-automation-restore",
 "p8bf-native-escrow-second-haos-automation-restore",
]
assert [(x["pr"],x["github_run"],x["successful_pr_checks"]) for x in new]==[
 (136,40328174220,12),(137,40330463271,12),(137,40330463267,12)]
heads=[
 "3527be53ac91e3a329e3289772786d4a73a1d9f9",
 "ed07cc772951079347eaaa08234623e2b0674f55",
 "ed07cc772951079347eaaa08234623e2b0674f55"]
merges=[
 "004f2cb52c9993cff34653f133e8c31919585803",
 "18b519e9b91451fed38a22665a9109ca7682225b",
 "18b519e9b91451fed38a22665a9109ca7682225b"]
for index,c in enumerate(new):
    assert c["head_sha"]==heads[index] and c["merge_sha"]==merges[index]
    assert c["head_sha"]!=c["merge_sha"]
    assert c["status"].startswith("verified_disposable_")
    assert len(c["claim"])>100 and len(c["limit"])>75
    source=root/c["source_file"]
    assert source.is_file() and c["marker"] in source.read_text()
    assert str(c["github_run"]) in (root/"docs/CERTIFICATION_EVIDENCE_REGISTER.md").read_text()
assert e["gate"]=="haos_backup_restore" and e["gate_status"]=="lab_partial"
assert len(e["tested"])==6 and len(e["not_tested"])>=7
assert "official HAOS 18.3" in e["environment"]
assert "synthetic entity identities" in e["environment"]
wf=(root/".github/workflows/p8aa-haos-native-restore.yml").read_text()
case=(root/"scripts/test_p8aa_genuine_archive_qemu.py").read_text()
for marker in e["markers"]:
    assert marker in wf+case+(root/"scripts/test_p8aa_qemu_contract.py").read_text(),marker
for path in ("docs/HANDOFF_CURRENT.md","docs/WORK_LOG.md",
             "docs/CERTIFICATION_EVIDENCE_REGISTER.md",
             "docs/HAOS_TO_NATIVE_BACKUP_STAGE_2026-10-25.md"):
    content=(root/path).read_text()
    assert e["github_run"].__str__() in content,path
    assert e["head_sha"] in content,path
assert g["publication"]=="public_alpha_lab_only"
assert len(g["required_gates"])==15
assert not any(row.get("status") in ("certified","production_ready")
               or row.get("certification_artifact") is not None
               for row in g["required_gates"])
gate={row["id"]:row for row in g["required_gates"]}
assert gate["haos_backup_restore"]["status"]=="lab_partial"
assert gate["rollback_verified"]["status"]=="blocked"
assert str(e["github_run"]) in json.dumps(gate["haos_backup_restore"])
assert "entire Home Assistant Core runtime migration" in e["not_tested"]
assert "physical radios and whole-machine disaster rollback" in e["not_tested"]
print("P8AA_3_OF_3_GENUINE_QEMU_LAB_EVIDENCE_PINNED_0_OF_15_PRODUCTION_PASS")
