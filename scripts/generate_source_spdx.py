#!/usr/bin/env python3
"""Deterministic SPDX 2.3 SOURCE dependency SBOM from exact Cargo.lock.

This is NOT an installed-release binary inventory, legal clearance, or
cryptographic signature. All-platform lock dependency graph and complete
Cargo metadata are required. Absolute filesystem paths and tokens excluded.
"""
from __future__ import annotations
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import tomllib

ROOT=Path(__file__).resolve().parent.parent
OUTPUT=ROOT/"audits/CARGO_SOURCE_SPDX_2026-10-23.json"
HEX64=re.compile(r"^[a-f0-9]{64}$")
SUPPORTED=re.compile(r"^[A-Za-z0-9+(). _-]{1,256}$")

def cargo_metadata():
    result=subprocess.run(["cargo","metadata","--locked","--format-version","1"],
            cwd=ROOT,capture_output=True,check=True,timeout=120)
    return json.loads(result.stdout)

def build(data):
    lock_bytes=(ROOT/"Cargo.lock").read_bytes()
    lock_digest=hashlib.sha256(lock_bytes).hexdigest()
    lock=tomllib.loads(lock_bytes.decode("utf-8"))
    locked={(p["name"],p["version"]):p for p in lock["package"]}
    packages=data.get("packages")
    resolved=data.get("resolve")
    if not isinstance(packages,list) or not isinstance(resolved,dict):
        raise ValueError("Cargo metadata/resolve not complete")
    if len(packages)!=len(locked) or len(packages)<200:
        raise ValueError("incomplete package inventory")
    ids={}
    spdx_packages=[]
    for pkg in packages:
        name,version=pkg["name"],pkg["version"]
        if (name,version) not in locked:
            raise ValueError("metadata contains dependency not in Cargo.lock")
        src=pkg.get("source")
        typ=("workspace" if src is None else
             "registry" if str(src).startswith("registry+") else
             "git" if str(src).startswith("git+") else None)
        if typ is None:
            raise ValueError("unsupported dependency origin")
        pkid=pkg["id"]
        identifier="SPDXRef-Package-"+hashlib.sha256(pkid.encode()).hexdigest()[:24]
        if identifier in ids.values():
            raise ValueError("package SPDX identifier collision")
        ids[pkid]=identifier
        declared=pkg.get("license")
        license_expr=declared if isinstance(declared,str) and SUPPORTED.fullmatch(declared) and "/" not in declared else "NOASSERTION"
        row={
            "SPDXID":identifier,
            "name":name,
            "versionInfo":version,
            "downloadLocation":("https://static.crates.io/crates/"+name+"/"+name+"-"+version+".crate"
                                if typ=="registry" else "NOASSERTION"),
            "filesAnalyzed":False,
            "licenseConcluded":"NOASSERTION",
            "licenseDeclared":license_expr,
            "copyrightText":"NOASSERTION",
            "supplier":"NOASSERTION",
            "sourceInfo":"Cargo.lock all-target source dependency; not proven included in a release binary",
            "externalRefs":[{
                "referenceCategory":"PACKAGE-MANAGER",
                "referenceType":"purl",
                "referenceLocator":"pkg:cargo/"+name+"@"+version
            }]
        }
        if license_expr=="NOASSERTION":
            row["licenseComments"]="Original Cargo declaration requires review: "+str(declared or "MISSING")
        checksum=locked[(name,version)].get("checksum")
        if typ=="registry" and isinstance(checksum,str) and HEX64.fullmatch(checksum):
            row["checksums"]=[{"algorithm":"SHA256","checksumValue":checksum}]
        spdx_packages.append(row)
    graph=[]
    seen=set()
    for node in resolved.get("nodes",[]):
        src=ids.get(node.get("id"))
        if not src:raise ValueError("dependency graph points to missing package")
        for dep in node.get("deps",[]):
            dest=ids.get(dep.get("pkg"))
            if not dest:raise ValueError("dependency edge points to missing package")
            if src==dest:raise ValueError("self-dependent package")
            seen.add((src,dest))
    for a,b in sorted(seen):
        graph.append({"spdxElementId":a,"relatedSpdxElement":b,"relationshipType":"DEPENDS_ON"})
    workspace=[ids[p["id"]] for p in packages if p.get("source") is None]
    if len(workspace)!=8:
        raise ValueError("unknown workspace set: review Cargo.lock before SBOM update")
    graph+=[{"spdxElementId":"SPDXRef-DOCUMENT",
             "relatedSpdxElement":x,"relationshipType":"DESCRIBES"}
            for x in sorted(workspace)]
    spdx_packages.sort(key=lambda x:(x["name"],x["versionInfo"],x["SPDXID"]))
    return {
        "spdxVersion":"SPDX-2.3",
        "dataLicense":"CC0-1.0",
        "SPDXID":"SPDXRef-DOCUMENT",
        "name":"Rusty-Home-Open-all-target-Cargo-lock-source-inventory",
        "documentNamespace":"https://github.com/GodsQuantum/Rusty-Home-Open/spdx/source-lock/"+lock_digest,
        "creationInfo":{
            "creators":["Tool: RustyHome-Cargo-SPDX-source-inventory"],
            "created":"2026-10-23T00:00:00Z",
            "comment":"Source dependencies only; NOT signed, NOT legal clearance and NOT a deployment release."
        },
        "packages":spdx_packages,
        "relationships":graph
    }

def validate(output):
    assert output["spdxVersion"]=="SPDX-2.3"
    assert output["dataLicense"]=="CC0-1.0"
    assert output["SPDXID"]=="SPDXRef-DOCUMENT"
    lock_digest=hashlib.sha256((ROOT/"Cargo.lock").read_bytes()).hexdigest()
    assert output["documentNamespace"].endswith("/"+lock_digest)
    packages=output["packages"]
    assert len(packages)==217
    assert len({x["SPDXID"] for x in packages})==217
    assert sum(x["downloadLocation"]!="NOASSERTION" for x in packages)==205
    assert sum(x["downloadLocation"]=="NOASSERTION" for x in packages)==12
    assert all(x["licenseConcluded"]=="NOASSERTION" for x in packages)
    assert all(x["filesAnalyzed"] is False for x in packages)
    assert all(x["copyrightText"]=="NOASSERTION" for x in packages)
    assert all("manifest_path" not in json.dumps(x) for x in packages)
    refs={"SPDXRef-DOCUMENT"}|{x["SPDXID"] for x in packages}
    for rel in output["relationships"]:
        assert rel["spdxElementId"] in refs
        assert rel["relatedSpdxElement"] in refs
        assert rel["relationshipType"] in ("DEPENDS_ON","DESCRIBES")
    assert sum(x["relationshipType"]=="DESCRIBES" for x in output["relationships"])==8
    # Avoid accidentally claiming legal compliance, signing or shipping status.
    assert "NOT signed" in output["creationInfo"]["comment"]
    assert "NOT legal clearance" in output["creationInfo"]["comment"]

if __name__=="__main__":
    parser=argparse.ArgumentParser()
    parser.add_argument("--generate",action="store_true")
    args=parser.parse_args()
    if args.generate:
        OUTPUT.parent.mkdir(exist_ok=True)
        output=build(cargo_metadata())
        validate(output)
        OUTPUT.write_text(json.dumps(output,ensure_ascii=False,indent=2)+"\n")
    output=json.loads(OUTPUT.read_text())
    validate(output)
    print(f"SPDX_SOURCE_LOCK_PASS {len(output['packages'])} packages, {len(output['relationships'])} edges, unsigned/legal clearance false")
