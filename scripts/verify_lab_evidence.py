#!/usr/bin/env python3
"""Validate pinned CI lab evidence; not a production-release authority.

External GH run/job success was observed independently before registration.
This offline check prevents schema and source cross-wiring; CI success on
this validator alone does NOT prove GitHub results or release readiness.
"""
import json
import re
from pathlib import Path
ROOT=Path(__file__).resolve().parent.parent
SHA=re.compile(r"^[0-9a-f]{40}$")
REPO="GodsQuantum/Rusty-Home-Open"
def validate(data):
    assert data.get("schema_version")==1
    main=data.get("base_main_sha")
    assert isinstance(main,str) and SHA.fullmatch(main)
    rows=data.get("claims")
    assert isinstance(rows,list) and len(rows)>=10
    gates=json.loads((ROOT/"config/release-gates.json").read_text())["required_gates"]
    allowed={g["id"] for g in gates}
    seen=set()
    for r in rows:
        assert isinstance(r,dict) and re.fullmatch(r"[a-z][a-z0-9_]{2,60}",r.get("id",""))
        assert r["id"] not in seen
        seen.add(r["id"])
        assert r.get("release_gate") in allowed
        assert r.get("status")=="verified_lab_only"
        assert type(r.get("run_id")) is int and r["run_id"]>0
        assert isinstance(r.get("head_sha"),str) and SHA.fullmatch(r["head_sha"])
        assert all(isinstance(r.get(k),str) and 5<len(r[k])<=256 for k in ("workflow","job","scope"))
        assert r.get("url")==f"https://github.com/{REPO}/actions/runs/{r['run_id']}"
        if "merged_pr" in r:
            assert type(r["merged_pr"]) is int and 1<=r["merged_pr"]<=100000
    assert len({r["run_id"] for r in rows})>=6
    assert any(r["id"]=="apt_signed_tamper_refusal" and r["head_sha"]==main for r in rows)
    # Claims from the same known successful main release run cannot
    # disagree with the pinned exact main commit.
    assert all(r["head_sha"]==main for r in rows if r["run_id"]==40004128778)
    assert all(g["status"]!="certified" for g in gates)
    return rows

if __name__=="__main__":
    data=json.loads((ROOT/"config/lab-evidence.json").read_text())
    rows=validate(data)
    print(f"PASS: {len(rows)} pinned individual laboratory jobs, no production release certification")
