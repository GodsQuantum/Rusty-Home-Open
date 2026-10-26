#!/usr/bin/env python3
"""Immutable evidence for actual rootless OCI App laboratory milestones."""
import json,re
from pathlib import Path
r=Path(__file__).resolve().parent.parent
d=json.loads((r/"config/native-oci-app-evidence.json").read_text())
g=json.loads((r/"config/release-gates.json").read_text())
assert d["schema_version"]==1
assert d["production_release"]=="NOT_PRODUCTION_READY"
assert d["fully_certified_production_gates"]==0 and d["total_production_gates"]==15
ids=["p8ao","p8ap","p8aq","p8ar","p8as"]
prs=[117,118,120,121,122]
runs=[40264974609,40266019276,40290122155,40291069681,40292676656]
checks=[20,21,11,23,24]
heads=[
"fe87738ae63b7f7b00f08b0ebe24bab5be7f699c",
"dc5bd1425cdec7c655e96541a34ac04c687e07b5",
"315edc33ae71c172e52ab15bb4c571279f421dd1",
"44acfc75bd93242622e9d7ebf65deb1385f81283",
"cddcde82764cac38976e0ee7af7c9b9232103797"
]
merges=[
"b8d322bc8c37e3c4302f7eb08e45c76dbebccd7b",
"071b85bac4f887843be3a2f47d25e44ddcc03bd1",
"7cf1d3ad3fb74d9e7d8537f171e500b94d86eac0",
"5af8dc1a268c7316bfe99eb50290f94726a57541",
"95123e09ac73d63da28e6106fb63794cc2fa85af"
]
assert [c["case_id"] for c in d["cases"]]==ids
assert [c["pr"] for c in d["cases"]]==prs
assert [c["github_actions_run_id"] for c in d["cases"]]==runs
assert [c["checks_passed"] for c in d["cases"]]==checks
assert [c["feature_head_sha"] for c in d["cases"]]==heads
assert [c["merged_commit_sha"] for c in d["cases"]]==merges
for c in d["cases"]:
 assert c["checks_failed"]==0
 assert c["status"]=="VERIFIED_IN_DISPOSABLE_REAL_ROOTLESS_OCI_APP_LAB_ONLY"
 assert re.fullmatch(r"[a-f0-9]{40}",c["feature_head_sha"])
 assert c["github_actions_run_url"].endswith("/"+str(c["github_actions_run_id"]))
 wf=(r/c["github_workflow"]).read_text()
 evidence=(r/c.get("verified_marker_source",c["github_workflow"])).read_text()
 assert "jobs:" in wf and "run:" in wf,c["case_id"]
 assert c["verified_marker"] in evidence,c["case_id"]
 for doc in ("docs/HANDOFF_CURRENT.md","docs/WORK_LOG.md",
             "docs/CERTIFICATION_EVIDENCE_REGISTER.md",
             "docs/HAOS_RUST_GUEST_COMPAT_2026-10-25.md"):
  assert str(c["github_actions_run_id"]) in (r/doc).read_text(),(c["case_id"],doc)
assert len(g["required_gates"])==15
assert all(row.get("certification_artifact") is None for row in g["required_gates"])
assert all(row.get("status") not in ("certified","production_ready") for row in g["required_gates"])
assert "outbound egress" in " ".join(d["limitations"])
print("P8AO_THROUGH_P8AS_GENUINE_ROOTLESS_OCI_APP_PROOFS_PINNED_0_OF_15_PRODUCTION_PASS")
