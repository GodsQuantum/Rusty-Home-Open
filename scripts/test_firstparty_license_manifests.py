#!/usr/bin/env python3
"""All eight Rusty Home first-party crates declare root MIT redistribution terms."""
from pathlib import Path
import tomllib
root=Path(__file__).resolve().parent.parent
license_path=root/"LICENSE"
text=license_path.read_text()
assert text.startswith("MIT License\n")
assert "Copyright (c) 2026 Rusty Home contributors" in text
manifests=sorted((root/"crates").glob("*/Cargo.toml"))
assert len(manifests)==8
for file in manifests:
    content=tomllib.loads(file.read_text())
    package=content["package"]
    assert package["license"]=="MIT",(file,package.get("license"))
    assert package["publish"] is False,(file,"unreviewed publish")
    assert package["name"] and package["version"]
print("FIRST_PARTY_RUST_MIT_LICENSE_EIGHT_MANIFESTS_PASS")
