#!/usr/bin/env python3
"""Read-only, pinned upstream source-license evidence for six Cargo archives.

Some crates.io archives omit license text because it lives at the upstream
repository root. Record SHA-256 and git blob sha at an exact release commit;
NEVER misreport this as final legal compliance or signed SBOM.
"""
from __future__ import annotations
import argparse
import base64
import hashlib
import json
from pathlib import Path
import re
import subprocess
import tomllib

ROOT=Path(__file__).resolve().parent.parent
OUT=ROOT/"audits/UPSTREAM_ROOT_LICENSE_PINS_2026-10-23.json"
# Names and versions must be derived from the locked dependencies, not
# an allowlist of only the production environment or a sample inventory.
PINS=(
 ("mqttbytes-core-next","0.34.0","thehouseisonfire/rumqtt",
  "aa7a694f9b76b17d4c31200cf73d79616acae9b3","mqttbytes-core/Cargo.toml","LICENSE","Apache-2.0"),
 ("rumqttc-core-next","0.34.0","thehouseisonfire/rumqtt",
  "aa7a694f9b76b17d4c31200cf73d79616acae9b3","rumqttc-core/Cargo.toml","LICENSE","Apache-2.0"),
 ("rumqttc-v4-next","0.34.0","thehouseisonfire/rumqtt",
  "aa7a694f9b76b17d4c31200cf73d79616acae9b3","rumqttc-v4/Cargo.toml","LICENSE","Apache-2.0"),
 ("rumqttc-v5-next","0.34.0","thehouseisonfire/rumqtt",
  "aa7a694f9b76b17d4c31200cf73d79616acae9b3","rumqttc-v5/Cargo.toml","LICENSE","Apache-2.0"),
 ("r-efi","5.3.0","r-efi/r-efi",
  "97b55bed1c2c91dcbf787674849f05337ff80b33","Cargo.toml","AUTHORS","MIT OR Apache-2.0 OR LGPL-2.1-or-later"),
 ("r-efi","6.0.0","r-efi/r-efi",
  "7e1b0322d31d625f81a5656096330934f9cd835d","Cargo.toml","AUTHORS","MIT OR Apache-2.0 OR LGPL-2.1-or-later")
)
HEX40=re.compile(r"[a-f0-9]{40}\Z")
HEX64=re.compile(r"[a-f0-9]{64}\Z")

def locked_packages():
    lock=tomllib.loads((ROOT/"Cargo.lock").read_text())
    return {(row["name"],row["version"]) for row in lock["package"]}

def fetch_file(repo,sha,path):
    output=subprocess.run(["gh","api","-X","GET",f"repos/{repo}/contents/{path}",
                           "-f",f"ref={sha}"],text=True,capture_output=True,
                          timeout=30,check=True).stdout
    obj=json.loads(output)
    assert obj["type"]=="file" and obj["encoding"]=="base64"
    payload=base64.b64decode(obj["content"],validate=False)
    assert len(payload)<=131072
    return payload,obj["sha"]

def build():
    locked=locked_packages()
    rows=[]
    for name,version,repo,sha,manifest,source_license,expression in PINS:
        assert (name,version) in locked,(name,version)
        source,source_blob=fetch_file(repo,sha,manifest)
        pkg=tomllib.loads(source.decode("utf8"))["package"]
        assert pkg["name"]==name and pkg["version"]==version
        license_bytes,license_blob=fetch_file(repo,sha,source_license)
        text=license_bytes.decode("utf8")
        assert text and len(text)>100
        if source_license=="AUTHORS":
            assert "Apache" in text and "LGPL" in text and "MIT" in text
        else:
            assert "Apache License" in text and "Version 2.0" in text
        rows.append({
          "name":name,"version":version,"upstream_repository":repo,
          "pinned_commit":sha,"manifest_path":manifest,
          "manifest_git_blob_sha1":source_blob,
          "license_text_path":source_license,
          "license_git_blob_sha1":license_blob,
          "license_text_sha256":hashlib.sha256(license_bytes).hexdigest(),
          "license_text_bytes":len(license_bytes),
          "declared_spdx_expression":expression,
          "source_url":f"https://github.com/{repo}/blob/{sha}/{source_license}",
          "provenance_verified":True,
          "redistribution_compatibility_certified":False
        })
    return {"schema_version":1,"source":"immutable_git_commit",
            "cargo_lock_sha256":hashlib.sha256((ROOT/"Cargo.lock").read_bytes()).hexdigest(),
            "archives_without_license_file":6,
            "pinned_upstream_source_texts_verified":6,
            "review_complete":False,"legal_compliance_certified":False,
            "signed_sbom_present":False,"rows":rows}

def validate(data):
    assert set(data)=={"schema_version","source","cargo_lock_sha256",
                       "archives_without_license_file","pinned_upstream_source_texts_verified",
                       "review_complete","legal_compliance_certified","signed_sbom_present","rows"}
    assert data["schema_version"]==1 and data["source"]=="immutable_git_commit"
    assert data["cargo_lock_sha256"]==hashlib.sha256((ROOT/"Cargo.lock").read_bytes()).hexdigest()
    assert data["archives_without_license_file"]==6
    assert data["pinned_upstream_source_texts_verified"]==6
    assert data["review_complete"] is False
    assert data["legal_compliance_certified"] is False
    assert data["signed_sbom_present"] is False
    rows=data["rows"]
    assert isinstance(rows,list) and len(rows)==6
    assert {(x["name"],x["version"]) for x in rows}=={(x[0],x[1]) for x in PINS}
    locked=locked_packages()
    for row in rows:
        pin=next(x for x in PINS if (x[0],x[1])==(row["name"],row["version"]))
        name,version,repo,sha,manifest,license_path,spdx=pin
        assert (name,version) in locked
        for key,value in (
          ("upstream_repository",repo),("pinned_commit",sha),
          ("manifest_path",manifest),("license_text_path",license_path),
          ("declared_spdx_expression",spdx),
          ("source_url",f"https://github.com/{repo}/blob/{sha}/{license_path}")
        ):
            assert row[key]==value,(name,key)
        for key in ("manifest_git_blob_sha1","license_git_blob_sha1"):
            assert isinstance(row[key],str) and HEX40.fullmatch(row[key])
        assert isinstance(row["license_text_sha256"],str) and HEX64.fullmatch(row["license_text_sha256"])
        assert type(row["license_text_bytes"]) is int and 100<row["license_text_bytes"]<131072
        assert row["provenance_verified"] is True
        assert row["redistribution_compatibility_certified"] is False
    return len(rows)

if __name__=="__main__":
    parser=argparse.ArgumentParser()
    parser.add_argument("--generate",action="store_true")
    parser.add_argument("--verify-upstream",action="store_true")
    args=parser.parse_args()
    if args.generate:
        data=build()
        OUT.parent.mkdir(exist_ok=True)
        OUT.write_text(json.dumps(data,indent=2,ensure_ascii=False)+"\n")
    data=json.loads(OUT.read_text())
    n=validate(data)
    if args.verify_upstream:
        assert data==build(),"pinned upstream source changed or receipt invalid"
    print(f"UPSTREAM_ROOT_LICENSE_PINS_PASS: {n} version-matched source texts verified; legal clearance FALSE")
