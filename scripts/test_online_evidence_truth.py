#!/usr/bin/env python3
"""Remote CI evidence can be rejected for forged run, job, SHA or merged PR."""
import copy,json
from pathlib import Path
from verify_lab_evidence import ROOT,validate,REPO
from verify_lab_evidence_online import verify, gh_api, merged_pr_from_graphql
from unittest.mock import patch
import subprocess
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


# REST is incapable of serving a historic oversized diff (#59, actual HTTP 422).
# GraphQL returns the same SHA-bound merged/base facts. No 403, 404, changed
# merge branch, SHA corruption or unsigned fake PR may trigger a bypass.
graph={"number":59,"state":"MERGED","baseRefName":"main",
       "headRefOid":"a"*40,"mergeCommit":{"oid":"b"*40}}
normal=merged_pr_from_graphql(graph,59)
assert normal["merged"] is True and normal["head"]["sha"]=="a"*40
assert normal["merge_commit_sha"]=="b"*40 and normal["base"]["ref"]=="main"
for broken in (
    {**graph,"number":58},
    {**graph,"state":"OPEN"},
    {**graph,"baseRefName":"attacker"},
    {**graph,"headRefOid":"0"*39},
    {**graph,"mergeCommit":None},
):
    try:merged_pr_from_graphql(broken,59)
    except (AssertionError,KeyError,TypeError):pass
    else:raise AssertionError("Invalid oversized-PR GraphQL attestation accepted")
err=subprocess.CalledProcessError(
    returncode=1,cmd=["gh","api"],stderr="too many files changed (HTTP 422)")
ok=subprocess.CompletedProcess(args=["gh","pr"],returncode=0,
                               stdout=json.dumps(graph),stderr="")
with patch("verify_lab_evidence_online.subprocess.run",side_effect=[err,ok]) as mock:
    assert gh_api("pulls/59")==normal
    assert mock.call_count==2
    assert mock.call_args_list[1].args[0][:4]==["gh","pr","view","59"]
for path,code in (("pulls/59","not found (HTTP 404)"),
                  ("pulls/59","rate limited (HTTP 403)"),
                  ("actions/runs/59","too many files changed (HTTP 422)")):
    bad=subprocess.CalledProcessError(1,["gh","api"],stderr=code)
    with patch("verify_lab_evidence_online.subprocess.run",side_effect=bad) as mock:
        try:gh_api(path)
        except subprocess.CalledProcessError:pass
        else:raise AssertionError("Unauthorized GraphQL fallback accepted")
        assert mock.call_count==1
print("GH_OVERSIZED_PR_422_STRICT_GRAPHQL_SHA_FALLBACK_PASS")
