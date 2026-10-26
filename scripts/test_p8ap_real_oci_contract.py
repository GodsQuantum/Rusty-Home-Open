#!/usr/bin/env python3
from pathlib import Path
r=Path(__file__).resolve().parent.parent
src=(r/"crates/native-hub/src/oci_quadlet.rs").read_text()
cli=(r/"crates/native-hub/src/bin/oci_quadlet_lab.rs").read_text()
s=(r/"scripts/test_p8ap_real_redis_quadlet.sh").read_text()
w=(r/".github/workflows/p8ap-real-oci-app.yml").read_text()
for key in ("run_uid", "User={uid}", ".volume:/data{mount_flags}", "uid == 0 || uid > 65533"):
 assert key in src,key
for key in ("--uid", "HARS_DISPOSABLE_OCI_LAB","create_new(true)"):
 assert key in cli,key
for key in ("systemctl --user start", "systemctl --user stop", "redis-cli SAVE",
            "redis-cli GET", "ReadonlyRootfs", "NetworkMode",
            "P8AP_REAL_ROOTLESS_REDIS_QUADLET_START_STOP_PERSISTENT_DATA_PASS"):
 assert key in s,key
assert "sudo podman" not in s
assert "hostfwd" not in s
assert "192.168." not in s
for key in ("ubuntu-24.04", "test_p8ap_real_redis_quadlet.sh","podman uidmap"):
 assert key in w,key
print("P8AP_REAL_GENERIC_ROOTLESS_OCI_APPLICATION_LIFECYCLE_CONTRACT_PASS")
