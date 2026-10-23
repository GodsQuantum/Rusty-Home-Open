#!/usr/bin/env python3
"""Production gate requires two independently signed, exact-scope proof files."""
from __future__ import annotations
import base64
import copy
import hashlib
import json
import subprocess
import tempfile
from pathlib import Path
from check_release_gate import ROOT, EXPECTED_GATES, audit
from release_attestation import REQUIRED_SCENARIOS

config=json.loads((ROOT/"config/release-gates.json").read_text())
assert set(REQUIRED_SCENARIOS)==EXPECTED_GATES,"unreviewed release gate lacks mandatory test set"
assert all(len(items)>=4 for items in REQUIRED_SCENARIOS.values())

with tempfile.TemporaryDirectory(prefix="hars-fake-review-") as td:
    root=Path(td)
    (root/"docs/certifications").mkdir(parents=True)
    (root/"config").mkdir()
    gate_name="authenticated_host_transport"
    evidence=root/"docs/certifications"/(gate_name+".json")
    assert config["required_gates"][0]["id"]==gate_name
    proof=copy.deepcopy(config)
    gate=proof["required_gates"][0]
    gate["status"]="certified"

    def result(art):
        gate["certification_artifact"]=art
        return audit(proof,root=root)

    # RED regression: the previous release gate wrongly counted a forged
    # self-hashed arbitrary JSON file as a certified production gate.
    evidence.write_text('{"certified":true,"claim":"ALL GREEN"}\n')
    base={"name":evidence.name,
          "sha256":hashlib.sha256(evidence.read_bytes()).hexdigest()}
    assert result(base)["blocking_gates"]==15, "arbitrary locally forged JSON accepted"
    assert result({**base,"signatures":[]})["progress"]["certified"]==0

    product_evidence={
      "schema_version":1,
      "gate_id":gate_name,
      "scope":"production_end_to_end",
      "product_commit_sha":"7"*40,
      "validated_scenarios":["real_host_transport","actual_user_revocation",
                             "connection_recovery","privilege_separation"],
      "independent_ci_runs":[12000001,12000002],
      "recovery_verified":True,
    }
    evidence.write_text(json.dumps(product_evidence,sort_keys=True,separators=(",",":"))+"\n")
    reviewers=[]
    signatures=[]
    for reviewer in ("qa_independent","release_reviewer"):
        secret=root/(reviewer+".key")
        public=root/(reviewer+".pem")
        subprocess.run(["openssl","genpkey","-algorithm","ED25519","-out",str(secret)],
                       check=True,capture_output=True,timeout=10)
        subprocess.run(["openssl","pkey","-in",str(secret),"-pubout","-out",str(public)],
                       check=True,capture_output=True,timeout=10)
        signed=subprocess.run(["openssl","pkeyutl","-sign","-rawin",
                               "-inkey",str(secret),"-in",str(evidence)],
                              check=True,capture_output=True,timeout=10).stdout
        reviewers.append({"id":reviewer,"public_key_pem":public.read_text(),"active":True})
        signatures.append({"reviewer":reviewer,
                           "signature_b64":base64.b64encode(signed).decode()})
    (root/"config/release-attestors.json").write_text(json.dumps(
        {"schema_version":1,"reviewers":reviewers}))
    artifact={"name":evidence.name,
              "sha256":hashlib.sha256(evidence.read_bytes()).hexdigest(),
              "signatures":signatures}
    confirmed=result(artifact)
    assert confirmed["blocking_gates"]==14, confirmed
    assert confirmed["progress"]["certified"]==1, confirmed
    assert confirmed["independent_release_signoff"] is False
    assert confirmed["deployment_authorized"] is False

    # Separate reviewer labels must not be allowed to reuse one key.
    same_key=copy.deepcopy(reviewers)
    same_key[1]["public_key_pem"]=same_key[0]["public_key_pem"]
    (root/"config/release-attestors.json").write_text(json.dumps(
        {"schema_version":1,"reviewers":same_key}))
    assert result(artifact)["blocking_gates"]==15, "shared signing key accepted"
    (root/"config/release-attestors.json").write_text(json.dumps(
        {"schema_version":1,"reviewers":reviewers}))
    # Even authentic signatures must NOT waive a mandatory acceptance test.
    incomplete=copy.deepcopy(product_evidence)
    incomplete["validated_scenarios"].remove("privilege_separation")
    evidence.write_text(json.dumps(incomplete,sort_keys=True,separators=(",",":"))+"\n")
    still_signed=copy.deepcopy(artifact)
    still_signed["sha256"]=hashlib.sha256(evidence.read_bytes()).hexdigest()
    for idx,reviewer in enumerate(("qa_independent","release_reviewer")):
        signed=subprocess.run(["openssl","pkeyutl","-sign","-rawin",
                               "-inkey",str(root/(reviewer+".key")),
                               "-in",str(evidence)],check=True,capture_output=True,timeout=10).stdout
        still_signed["signatures"][idx]["signature_b64"]=base64.b64encode(signed).decode()
    assert result(still_signed)["blocking_gates"]==15,"signed incomplete gate evidence accepted"
    evidence.write_text(json.dumps(product_evidence,sort_keys=True,separators=(",",":"))+"\n")
    forged=copy.deepcopy(artifact)
    forged["signatures"]=[signatures[0],signatures[0]]
    assert result(forged)["blocking_gates"]==15,"duplicated reviewer counted twice"
    forged=copy.deepcopy(artifact)
    forged["signatures"][1]["signature_b64"]=signatures[0]["signature_b64"]
    assert result(forged)["blocking_gates"]==15,"one reviewer's signature accepted as another"
    forged=copy.deepcopy(artifact)
    forged["sha256"]="0"*64
    assert result(forged)["blocking_gates"]==15,"changed evidence SHA accepted"
    changed=copy.deepcopy(product_evidence)
    changed["scope"]="isolated_lab_only"
    evidence.write_text(json.dumps(changed,sort_keys=True,separators=(",",":"))+"\n")
    forged=copy.deepcopy(artifact)
    forged["sha256"]=hashlib.sha256(evidence.read_bytes()).hexdigest()
    assert result(forged)["blocking_gates"]==15,"changed scope with old signatures accepted"
print("PASS: two distinct signed reviews required; unsigned/duplicated/tampered proof fails closed")
