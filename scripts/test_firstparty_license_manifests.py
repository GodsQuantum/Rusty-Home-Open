#!/usr/bin/env python3
"""Audit eight actual product workspace crates PLUS any explicit isolated lab.

Never silently accept an unexpected ninth publishable first-party crate.
The experimental MQTT lab is a separate nonproduct Rust workspace with its
own lockfile, and is nevertheless required to declare MIT and publish=false.
"""
from pathlib import Path
import tomllib

root=Path(__file__).resolve().parent.parent
license_path=root/"LICENSE"
text=license_path.read_text()
assert text.startswith("MIT License\n")
assert "Copyright (c) 2026 Rusty Home contributors" in text
workspace=tomllib.loads((root/"Cargo.toml").read_text())["workspace"]
members=workspace["members"]
assert isinstance(members,list) and len(members)==8
assert len(members)==len(set(members))
product={root/member/"Cargo.toml" for member in members}
all_manifests=set((root/"crates").glob("*/Cargo.toml"))
assert len(product)==8 and product<=all_manifests
assert all(member.startswith("crates/") for member in members)
unexpected=all_manifests-product
expected_lab=root/"crates/mqtt-supervisor-auth-lab/Cargo.toml"
assert unexpected=={expected_lab}, ("unexpected unreviewed Rust crate",unexpected)
for manifest in sorted(all_manifests):
    parsed=tomllib.loads(manifest.read_text())
    package=parsed["package"]
    assert package["license"]=="MIT",(manifest,"unreviewed license")
    assert package["publish"] is False,(manifest,"unreviewed publish")
    assert package["name"] and package["version"]
    if manifest==expected_lab:
        assert "workspace" in parsed and parsed["workspace"]["resolver"]=="2"
        assert (manifest.parent/"Cargo.lock").is_file()
        for entry in parsed.get("dependencies",{}).values():
            assert not (isinstance(entry,dict) and "path" in entry), "lab imports live product"
print("FIRST_PARTY_RUST_MIT_LICENSE_EIGHT_PRODUCT_PLUS_ISOLATED_LAB_PASS")
