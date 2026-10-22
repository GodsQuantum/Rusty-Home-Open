#!/usr/bin/env python3
"""Actual Cargo source-license file digest evidence. Not legal compliance."""
from __future__ import annotations
import argparse
import hashlib
import json
from pathlib import Path

PREFIXES = ("LICENSE", "LICENCE", "COPYING", "NOTICE")
MAX_FILE_BYTES = 1024 * 1024
MAX_FILES = 32

def matching_files(folder: Path) -> list[Path]:
    if not folder.is_dir() or folder.is_symlink():
        return []
    return sorted(
        (x for x in folder.iterdir() if x.name.upper().startswith(PREFIXES) and
         x.is_file() and not x.is_symlink()),
        key=lambda x:x.name.lower()
    )

def candidate_files(package: dict) -> tuple[list[Path], bool]:
    package_root=Path(package["manifest_path"]).parent
    source=package.get("source")
    if matching_files(package_root):
        return matching_files(package_root),False
    # Git crates often license the *whole checkout* once at its root.
    # Limit traversal to the pinned checkout; never walk into Cargo home.
    if not isinstance(source,str) or not source.startswith("git+https://"):
        return [],False
    ancestor=package_root
    for _ in range(5):
        if ancestor.parent == ancestor:
            break
        ancestor=ancestor.parent
        # Stop when looking at Cargo checkout machinery, not repository.
        if ancestor.name in ("checkouts","git",".cargo"):
            break
        found=matching_files(ancestor)
        if found:
            return found,True
    return [],False

def report(metadata:dict)->dict:
    packages=metadata.get("packages")
    if not isinstance(packages,list) or not packages:
        raise ValueError("invalid Cargo metadata")
    seen=set()
    rows=[]
    for p in packages:
        name,version,source=p.get("name"),p.get("version"),p.get("source")
        if not isinstance(name,str) or not name or not isinstance(version,str) or not version:
            raise ValueError("bad package identity")
        typ=("workspace" if source is None else
             "git" if isinstance(source,str) and source.startswith("git+https://") else
             "registry" if isinstance(source,str) and source.startswith("registry+https://") else None)
        if typ is None: raise ValueError("unsupported source type")
        key=(name,version,source)
        if key in seen: raise ValueError("duplicate package identity")
        seen.add(key)
        if not isinstance(p.get("manifest_path"),str):
            raise ValueError("no manifest path in metadata")
        files, inherited=candidate_files(p)
        if len(files)>MAX_FILES: raise ValueError("too many license files")
        evidence=[]
        for f in files:
            size=f.stat().st_size
            if size>MAX_FILE_BYTES:
                raise ValueError("oversized license file")
            digest=hashlib.sha256(f.read_bytes()).hexdigest()
            evidence.append({"filename":f.name,"bytes":size,"sha256":digest})
        rows.append({
            "name":name,"version":version,"source_type":typ,
            "from_git_checkout_root":inherited,
            "license_and_notice_files":evidence,
        })
    rows.sort(key=lambda x:(x["name"],x["version"],x["source_type"]))
    unresolved=[{"name":r["name"],"version":r["version"],"source_type":r["source_type"],
                 "issue":"no_license_or_notice_file_in_package_or_pinned_git_root"}
                for r in rows if not r["license_and_notice_files"]]
    return {
        "format":"hars-cargo-license-text-sha-1",
        "packages_total":len(rows),
        "external_packages":sum(p["source_type"]!="workspace" for p in rows),
        "external_with_source_license_files":sum(
            p["source_type"]!="workspace" and bool(p["license_and_notice_files"]) for p in rows),
        "unresolved":unresolved,
        "packages":rows,
        "notice_asset_review_complete":False,
        "license_text_interpretation_complete":False,
        "redistribution_compatibility_certified":False,
        "third_party_licenses_gate_certified":False,
    }
def main()->None:
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument("--input",type=Path,required=True)
    p.add_argument("--output",type=Path,required=True)
    a=p.parse_args()
    payload=report(json.loads(a.input.read_text(encoding="utf-8")))
    a.output.write_text(json.dumps(payload,ensure_ascii=True,indent=2)+"\n",encoding="utf-8")
    print(json.dumps({k:payload[k] for k in (
        "packages_total","external_packages","external_with_source_license_files",
        "third_party_licenses_gate_certified"
    )}))
if __name__=="__main__":main()
