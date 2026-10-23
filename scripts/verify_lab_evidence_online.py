#!/usr/bin/env python3
"""Independent online proof lookup for the pinned laboratory CI ledger.

Use official GitHub REST runs, exact source SHA, successful named jobs,
and merged PR provenance. This is NOT a production acceptance/signoff.
"""
from __future__ import annotations
import json
import subprocess
from pathlib import Path
from verify_lab_evidence import ROOT, REPO, validate

def gh_api(path:str)->dict:
    result=subprocess.run(
        ["gh","api","-X","GET",f"repos/{REPO}/{path}"],
        capture_output=True,text=True,check=True,timeout=35)
    return json.loads(result.stdout)

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
