#!/usr/bin/env python3
"""Deterministic Cargo *declared* license inventory. NOT compliance or a SBOM."""
from __future__ import annotations
import argparse
import json
import re
from pathlib import Path

LEGAL_TEXT = re.compile(r"[A-Za-z0-9().+/_ -]{1,256}\Z")

def report(data: dict) -> dict:
    packages = data.get("packages")
    if not isinstance(packages,list) or not packages:
        raise ValueError("invalid cargo metadata")
    rows=[]
    seen=set()
    for p in packages:
        name, version, source = p.get("name"),p.get("version"),p.get("source")
        if not isinstance(name,str) or not isinstance(version,str) or not name or not version:
            raise ValueError("invalid cargo package identity")
        if source is None:
            origin="workspace"
        elif isinstance(source,str) and source.startswith("registry+https://"):
            origin="registry"
        elif isinstance(source,str) and source.startswith("git+https://"):
            origin="git"
        else:
            raise ValueError("unsupported cargo package source")
        key=(name,version,source)
        if key in seen:
            raise ValueError("duplicate cargo package identity")
        seen.add(key)
        declared=p.get("license")
        if declared is not None and (not isinstance(declared,str) or not LEGAL_TEXT.fullmatch(declared)):
            raise ValueError("unreviewed license declaration format")
        if declared is None and origin!="workspace":
            raise ValueError("external dependency lacks license declaration")
        row={"name":name,"version":version,"source_type":origin,"declared_license":declared}
        issues=[]
        if declared is None:
            issues.append("missing_workspace_license_declaration")
        elif "/" in declared:
            issues.append("legacy_slash_license_expression_review")
        if issues:
            row["issues"]=issues
        rows.append(row)
    rows.sort(key=lambda x:(x["name"],x["version"],x["source_type"]))
    unresolved=[
        {"name":p["name"],"version":p["version"],"issues":p["issues"]}
        for p in rows if "issues" in p
    ]
    return {
        "format":"hars-cargo-declared-license-1",
        "packages_total":len(rows),
        "registry_count":sum(r["source_type"]=="registry" for r in rows),
        "git_count":sum(r["source_type"]=="git" for r in rows),
        "workspace_count":sum(r["source_type"]=="workspace" for r in rows),
        "unresolved":unresolved,
        "packages":rows,
        "individual_source_license_verified":False,
        "notice_and_asset_review_complete":False,
        "license_compatibility_review_complete":False,
        "third_party_licenses_gate_certified":False,
    }

def main()->None:
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input",required=True,type=Path)
    parser.add_argument("--output",required=True,type=Path)
    a=parser.parse_args()
    content=json.loads(a.input.read_text(encoding="utf-8"))
    result=report(content)
    a.output.write_text(json.dumps(result,indent=2,ensure_ascii=True)+"\n",encoding="utf-8")
    print(json.dumps({k:result[k] for k in ("packages_total","registry_count","git_count","workspace_count","third_party_licenses_gate_certified")}))

if __name__=="__main__":main()
