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
ids=["p8ao","p8ap","p8aq","p8ar","p8as","p8at","p8au","p8av","p8aw","p8ay","p8az"]
prs=[117,118,120,121,122,124,125,126,127,130,131]
runs=[40264974609,40266019276,40290122155,40291069681,40292676656,40295014593,40297108559,40299977840,40301611933,40314671475,40315761886]
checks=[20,21,11,23,24,25,26,10,28,18,20]
heads=[
"fe87738ae63b7f7b00f08b0ebe24bab5be7f699c",
"dc5bd1425cdec7c655e96541a34ac04c687e07b5",
"315edc33ae71c172e52ab15bb4c571279f421dd1",
"44acfc75bd93242622e9d7ebf65deb1385f81283",
"cddcde82764cac38976e0ee7af7c9b9232103797",
"f11a80226a55a7b2e7c7e346beae16e9997a2a94",
"fa64f2940624d949f70d8ad501404d819f59f2f4",
"a4f66d11c1ff127547ac4bebe906cf8fd43c47ae",
"be76a29af314ab6607f9dec1f5c0302967e5b4f0",
"3004210c539fcb4c6d43055cf67f0e634e59ae73",
"ddcd3550a134abeeb899d38d107c6a1044eeebe2"
]
merges=[
"b8d322bc8c37e3c4302f7eb08e45c76dbebccd7b",
"071b85bac4f887843be3a2f47d25e44ddcc03bd1",
"7cf1d3ad3fb74d9e7d8537f171e500b94d86eac0",
"5af8dc1a268c7316bfe99eb50290f94726a57541",
"95123e09ac73d63da28e6106fb63794cc2fa85af",
"42db2dc1f1bf53873b37a95e83b88c6dfd5bd658",
"05008d4cb9d2d577bec1567ea3bcda67f1faaa2e",
"6ed03afa6d9a55a28cd4a51d28d48c21480fe57d",
"b1881a64ccfcc47f75e39a61fe3b1142c78b861f",
"075e18e525c960cee19390180b09eba2bf2b65c2",
"a351529c6a4ce6ac826430e3b5ad8b6f400028c1"
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
print("P8AO_THROUGH_P8AZ_GENUINE_ROOTLESS_OCI_APP_PROOFS_PINNED_0_OF_15_PRODUCTION_PASS")
