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
assert [c["case_id"] for c in d["cases"]]==["p8ac","p8ad","p8ae","p8af"]
assert [c["checks_passed"] for c in d["cases"]]==[18,11,12,13]
assert [c["github_actions_run_id"] for c in d["cases"]]==[40243098816,40243728521,40247000475,40247834775]
assert [c["pr"] for c in d["cases"]]==[103,104,105,106]
assert [c["feature_head_sha"] for c in d["cases"]]==[
 "29f9a45db72e4ef69aa0abed00dda17483d7133e",
 "73f91ea3330adfa642e40c0854a001a4eabf206d",
 "4534b23a4b2f0913430251dfe4b32d75fa788c94",
 "267e0fb0e287bb0072a0d933dc6365526f64e4e3"]
assert d["cases"][-1]["merged_commit_sha"]=="d8c5bee817c7b7e8ef5ea6af24d62d366fbe610a"
for item in d["cases"]:
 assert item["status"]=="VERIFIED_IN_DISPOSABLE_REAL_HAOS_KVM_LAB_ONLY"
 assert item["checks_failed"]==0 and len(item["merged_commit_sha"])==40
 assert re.fullmatch("[a-f0-9]{40}",item["feature_head_sha"])
 assert item["github_actions_run_url"].endswith("/"+str(item["github_actions_run_id"]))
 wf=(root/item["github_workflow"]).read_text()
 assert item["verified_marker"] in wf,item["case_id"]
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
print("P8AC_P8AD_P8AE_P8AF_OFFICIAL_HAOS_KVM_PROOFS_PINNED_0_OF_15_PRODUCTION_PASS")
