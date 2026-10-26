#!/usr/bin/env python3
from pathlib import Path
r=Path(__file__).resolve().parent.parent
s=(r/"crates/native-hub/src/oci_quadlet.rs").read_text()
cli=(r/"crates/native-hub/src/bin/oci_quadlet_lab.rs").read_text()
w=(r/".github/workflows/p8ao-oci-quadlet.yml").read_text()
for token in ("Network=none","Pull=never","NoNewPrivileges=true",
              "DropCapability=all","ReadOnly=true","PidsLimit=","MemoryMax=",
              "VolumeName=","@sha256:"):
 assert token in s,token
assert "[Install]" not in s.split("#[cfg(test)]",1)[0]
for token in ("HARS_DISPOSABLE_OCI_LAB","create_new(true)","symlink_metadata",
              "P8AO_GENERIC_DIGEST_PINNED_ROOTLESS_QUADLET_GENERATED_PASS"):
 assert token in cli,token
for token in ("podman-system-generator","QUADLET_UNIT_DIRS",
              "P8AO_OFFICIAL_PODMAN_QUADLET_GENERATOR_PASS"):
 assert token in w,token
print("P8AO_GENERIC_OCI_QUADLET_LAB_ISOLATION_CONTRACT_PASS")
