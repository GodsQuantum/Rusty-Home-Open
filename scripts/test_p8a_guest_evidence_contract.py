#!/usr/bin/env python3
"""Exact real KVM evidence: immutable SHA+run and fail closed on production forgery."""
from pathlib import Path
import json,re
root=Path(__file__).resolve().parent.parent
d=json.loads((root/"config/haos-rust-guest-evidence.json").read_text())
g=json.loads((root/"config/release-gates.json").read_text())
assert d["schema_version"]==1
assert d["production_release"]=="NOT_PRODUCTION_READY"
assert d["fully_certified_production_gates"]==0 and d["total_production_gates"]==15
assert [c["case_id"] for c in d["cases"]]==["p8ac","p8ad","p8ae","p8af","p8ag","p8ah","p8ai","p8aj","p8ak","p8al"]
assert [c["checks_passed"] for c in d["cases"]]==[18,11,12,13,14,15,16,10,11,9]
assert [c["github_actions_run_id"] for c in d["cases"]]==[40243098816,40243728521,40247000475,40247834775,40250349459,40251693506,40252930987,40254861417,40255870443,40257162323]
assert [c["pr"] for c in d["cases"]]==[103,104,105,106,108,109,110,111,112,113]
assert [c["feature_head_sha"] for c in d["cases"]]==[
 "29f9a45db72e4ef69aa0abed00dda17483d7133e",
 "73f91ea3330adfa642e40c0854a001a4eabf206d",
 "4534b23a4b2f0913430251dfe4b32d75fa788c94",
 "267e0fb0e287bb0072a0d933dc6365526f64e4e3",
 "add44fa09a4bd34d9d19348a36dfcfe4a0ec464a",
 "1855cd9700b45a49d77caa841c787e4a83e8b1d8",
 "6849ce2913400005e896f9dbf6e930cb4dfc0c7d",
 "e1a2c97d4a4343be6c3ddb7b24fcbbfef9a1310c",
 "e4b14d9206ec273eb5cbde6a5050aff215a5040c",
 "4cb0c9d946d029c8aee5f95ca583212535a0287a"]
assert d["cases"][3]["merged_commit_sha"]=="d8c5bee817c7b7e8ef5ea6af24d62d366fbe610a"
assert d["cases"][-1]["merged_commit_sha"]=="a756696b60081d641ad7ec9cf43144f3361a201b"
for item in d["cases"]:
 assert item["status"]=="VERIFIED_IN_DISPOSABLE_REAL_HAOS_KVM_LAB_ONLY"
 assert item["checks_failed"]==0 and len(item["merged_commit_sha"])==40
 assert re.fullmatch("[a-f0-9]{40}",item["feature_head_sha"])
 assert item["github_actions_run_url"].endswith("/"+str(item["github_actions_run_id"]))
 wf=(root/item["github_workflow"]).read_text()
 assert item["verified_marker"] in (root/item.get("verified_marker_source",item["github_workflow"])).read_text(),item["case_id"]
 for doc in ("docs/HANDOFF_CURRENT.md","docs/WORK_LOG.md","docs/CERTIFICATION_EVIDENCE_REGISTER.md",
             "docs/HAOS_RUST_GUEST_COMPAT_2026-10-25.md"):
  assert str(item["github_actions_run_id"]) in (root/doc).read_text(),doc
assert len(g["required_gates"])==15
assert all(row.get("certification_artifact") is None for row in g["required_gates"])
assert not any(row.get("status") in ("certified","production_ready")
               for row in g["required_gates"])
statuses={v["id"]:v["status"] for v in g["required_gates"]}
assert statuses["rollback_verified"]=="blocked"
assert statuses["packaging_updates"]=="lab_partial"
assert "real household" in " ".join(d["critical_limits"]).lower()
print("P8AC_THROUGH_P8AL_RUST_HOST_PROOFS_PINNED_0_OF_15_PRODUCTION_PASS")
