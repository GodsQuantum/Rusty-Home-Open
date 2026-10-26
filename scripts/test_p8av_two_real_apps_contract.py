#!/usr/bin/env python3
from pathlib import Path
r=Path(__file__).resolve().parent.parent
lab=(r/"scripts/test_p8av_real_two_distinct_apps.sh").read_text()
workflow=(r/".github/workflows/p8av-real-two-oci-apps.yml").read_text()
manager=(r/"crates/native-hub/src/bin/oci_app_manager_lab.rs").read_text()
for v in ("docker.io/library/memcached:1.6-alpine",
          "docker.io/library/redis:7.4-alpine",
          "HARS_DISPOSABLE_OCI_LAB=1",
          "--install-lab memorycache", "--install-lab redis-two",
          "P8AV_TWO_LIVE_UNRELATED_ROOTLESS_OCI_APPS_ISOLATED_PASS",
          "P8AV_INDEPENDENT_APP_LIFECYCLE_AND_DATA_DURABILITY_PASS",
          "P8AV_TWO_GENERIC_APPS_UNINSTALL_WITHOUT_DATA_LOSS_PASS",
          "podman volume exists rh-memorycache-data"):
 assert v in lab,v
for v in ("ubuntu-24.04","test_p8av_real_two_distinct_apps.sh",
          "test_p8av_two_real_apps_contract.py","podman uidmap"):
 assert v in workflow,v
assert "memcached" not in manager and "redis" not in manager.split("#[cfg(test)]")[0]
assert "sudo podman" not in lab and "hostfwd" not in lab and "192.168." not in lab
print("P8AV_TWO_DISTINCT_OFFICIAL_OCI_APPS_UNIVERSAL_MANAGER_CONTRACT_PASS")
