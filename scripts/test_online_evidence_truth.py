#!/usr/bin/env python3
"""Remote CI evidence can be rejected for forged run, job, SHA or merged PR."""
import copy,json
from pathlib import Path
from verify_lab_evidence import ROOT,validate,REPO
from verify_lab_evidence_online import verify
claims=validate(json.loads((ROOT/"config/lab-evidence.json").read_text()))
selected=[x for x in claims if x["id"] in (
    "fuzz_four_native_security_surfaces","github_oidc_cargo_source_sbom")]
assert len(selected)==2
def build_fake(mutation=None):
    def fetch(path):
        if path.startswith("pulls/"):
            ref=int(path.split("/")[-1])
            row=next(x for x in selected if x["merged_pr"]==ref)
            value={"merged":True,"base":{"ref":"main"},
                   "head":{"sha":row["head_sha"]},
                   "merge_commit_sha":row["head_sha"]}
        else:
            run_id=int(path.split("/")[2].split("?")[0])
            row=next(x for x in selected if x["run_id"]==run_id)
            if "/jobs?" in path:
                value={"total_count":1,"jobs":[{"name":row["job"],
                       "status":"completed","conclusion":"success"}]}
            else:
                value={"id":run_id,"status":"completed","conclusion":"success",
                       "head_sha":row["head_sha"],"name":row["workflow"],
                       "repository":{"full_name":REPO}}
        if mutation is not None:
            mutation(path,value)
        return value
    return fetch

assert verify(selected,fetch=build_fake())==(2,2)
def deny(mutation,description):
    try:verify(selected,fetch=build_fake(mutation))
    except (AssertionError,KeyError):pass
    else:raise AssertionError("Untrusted remote evidence accepted: "+description)
deny(lambda p,v:v.update(conclusion="failure") if p.startswith("actions/runs/") and
     "/jobs?" not in p else None,"failed workflow")
deny(lambda p,v:v.update(head_sha="0"*40) if p.startswith("actions/runs/") and
     "/jobs?" not in p else None,"wrong source SHA")
deny(lambda p,v:v["jobs"][0].update(conclusion="failure") if "/jobs?" in p else None,
     "failed named job")
deny(lambda p,v:v.update(merged=False) if p.startswith("pulls/") else None,
     "unmerged pull request")
deny(lambda p,v:v["repository"].update(full_name="unknown/not-this-repo") if
     p.startswith("actions/runs/") and "/jobs?" not in p else None,
     "mismatched repository")
print("GH_REMOTE_CI_EVIDENCE_FAILURE_REJECTION_PASS")
