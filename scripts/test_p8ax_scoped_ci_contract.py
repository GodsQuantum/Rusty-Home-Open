#!/usr/bin/env python3
"""Require shared native-hub edits to remain fully tested while separating QEMU from OCI-only code."""
from pathlib import Path
from fnmatch import fnmatchcase
import yaml
r=Path(__file__).resolve().parent.parent
HAOS=[
 "p8ac-rust-guest","p8ad-rust-health","p8ae-rust-persistent",
 "p8af-rust-reattach","p8ag-guest-service","p8ah-loopback-ui",
 "p8ai-real-core-frontend","p8aj-real-systemd",
 "p8ak-guest-debian","p8an-crash-restart"]
OCI=[
 "p8ao-oci-quadlet","p8ap-real-oci-app","p8aq-real-oci-restore",
 "p8ar-real-oci-manager","p8as-oci-localhost-tcp","p8at-image-rollback",
 "p8au-app-health-rollback","p8av-real-two-oci-apps"]
ALL=HAOS+OCI
SHARED=[
 "Cargo.lock","crates/native-hub/Cargo.toml","crates/native-hub/build.rs",
 "crates/native-hub/src/lib.rs","crates/native-hub/tests/integration.rs"]
HAOS_RUNTIME=[
 "crates/native-hub/src/haos_guest.rs",
 "crates/native-hub/src/bin/haos_guest_lab.rs",
 "crates/native-hub/src/bin/haos_guest_service_lab.rs"]
OCI_RUNTIME=[
 "crates/native-hub/src/oci_quadlet.rs",
 "crates/native-hub/src/bin/oci_app_manager_lab.rs",
 "crates/native-hub/src/bin/oci_quadlet_lab.rs"]
def watched(patterns,path):
 return any(fnmatchcase(path,pattern) for pattern in patterns)
for name in ALL:
 path=r/".github/workflows"/(name+".yml")
 doc=yaml.safe_load(path.read_text())
 trigger=doc.get("on",doc.get(True,{}))
 assert "pull_request" in trigger and "workflow_dispatch" in trigger,name
 paths=trigger["pull_request"]["paths"]
 assert all(isinstance(x,str) for x in paths)
 assert "crates/native-hub/**" not in paths,name
 assert watched(paths,f".github/workflows/{name}.yml"),name
 assert all(watched(paths,x) for x in SHARED),name
 own=HAOS_RUNTIME if name in HAOS else OCI_RUNTIME
 other=OCI_RUNTIME if name in HAOS else HAOS_RUNTIME
 assert all(watched(paths,x) for x in own),name
 assert not any(watched(paths,x) for x in other),name
 assert "jobs" in doc and len(doc["jobs"])>=1
 for job in doc["jobs"].values():
  assert job.get("runs-on")=="ubuntu-24.04",name
print(f"P8AX_SCOPED_CI_CONTRACT_PASS  HAOS={len(HAOS)} OCI={len(OCI)} "
      f"shared={len(SHARED)} all_exact_app_and_guest_paths_covered")
