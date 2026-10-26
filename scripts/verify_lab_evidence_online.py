#!/usr/bin/env python3
"""Independent online proof lookup for the pinned laboratory CI ledger.

Use official GitHub REST runs, exact source SHA, successful named jobs,
and merged PR provenance. This is NOT a production acceptance/signoff.
"""
from __future__ import annotations
import json
import re
import subprocess
from pathlib import Path
from verify_lab_evidence import ROOT, REPO, validate

def merged_pr_from_graphql(raw: dict, number: int) -> dict:
    """Strict adapter: GitHub REST /pulls/59 returns HTTP 422 with huge diffs.

    gh pr view uses GraphQL and returns the precise immutable PR merge facts,
    not any unverified user-supplied metadata. Keep the ORIGINAL full SHA
    checks in verify(); missing/null fields always fail closed.
    """
    assert isinstance(number, int) and number > 0
    assert raw["number"] == number
    assert raw["state"] == "MERGED"
    assert raw["baseRefName"] == "main"
    head = raw["headRefOid"]
    merge = raw["mergeCommit"]["oid"]
    assert isinstance(head, str) and re.fullmatch(r"[0-9a-f]{40}", head)
    assert isinstance(merge, str) and re.fullmatch(r"[0-9a-f]{40}", merge)
    return {"merged": True, "base": {"ref": "main"},
            "head": {"sha": head}, "merge_commit_sha": merge}


def gh_api(path:str)->dict:
    try:
        result=subprocess.run(
            ["gh","api","-X","GET",f"repos/{REPO}/{path}"],
            capture_output=True,text=True,check=True,timeout=35)
        return json.loads(result.stdout)
    except subprocess.CalledProcessError as exc:
        # Known GitHub REST limitation, not an exception to provenance checks:
        # REST cannot render enormous historical PR diffs (HTTP 422). Fallback
        # solely for that exact error using official gh GraphQL PR metadata.
        hit=re.fullmatch(r"pulls/([1-9][0-9]*)", path)
        stderr=(exc.stderr or "").lower()
        if not hit or "422" not in stderr or "too many files changed" not in stderr:
            raise
        number=int(hit.group(1))
        fallback=subprocess.run(
            ["gh","pr","view",str(number),"-R",REPO,
             "--json","number,state,baseRefName,headRefOid,mergeCommit"],
            capture_output=True,text=True,check=True,timeout=35)
        return merged_pr_from_graphql(json.loads(fallback.stdout), number)

def verify(rows,fetch=gh_api):
    cached={}
    prs={}
    for row in rows:
        run_id=row["run_id"]
        if run_id not in cached:
            run=fetch(f"actions/runs/{run_id}")
            jobs=fetch(f"actions/runs/{run_id}/jobs?per_page=100")
            assert run["id"]==run_id and run["repository"]["full_name"]==REPO
            assert run["status"]=="completed" and run["conclusion"]=="success"
            assert type(jobs["total_count"]) is int and jobs["total_count"]<=100
            assert len(jobs["jobs"])==jobs["total_count"]
            cached[run_id]=(run,jobs["jobs"])
        run,jobs=cached[run_id]
        assert run["head_sha"]==row["head_sha"],row["id"]
        assert run["name"]==row["workflow"],row["id"]
        qualified=[
            j for j in jobs if j.get("name")==row["job"]
            and j.get("status")=="completed"
            and j.get("conclusion")=="success"
        ]
        assert len(qualified)==1,row["id"]
        if "merged_pr" in row:
            number=row["merged_pr"]
            if number not in prs:
                prs[number]=fetch(f"pulls/{number}")
                assert prs[number]["merged"] is True
                assert prs[number]["base"]["ref"]=="main"
            pr=prs[number]
            assert row["head_sha"] in (pr["head"]["sha"],pr["merge_commit_sha"]),row["id"]
    return len(rows),len(cached)

if __name__=="__main__":
    rows=validate(json.loads((ROOT/"config/lab-evidence.json").read_text()))
    count,runs=verify(rows)
    print(f"PASS: {count} exact-SHA successful CI jobs across {runs} run IDs independently checked; 0/15 production release gates")
