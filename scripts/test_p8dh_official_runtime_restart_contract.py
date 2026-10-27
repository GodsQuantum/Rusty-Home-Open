#!/usr/bin/env python3
"""P8dh rootless official App durable file write/restart without false UI claim."""
from pathlib import Path
root=Path(__file__).resolve().parents[1]
flow=(root/'.github/workflows/p8dh-official-app-oci-restart.yml').read_text()
source=(root/'scripts/test_p8df_official_app_runtime.py').read_text()
for token in ("haos_ova-18.3.qcow2.xz", "-snapshot -accel kvm",
              "runs-on: ubuntu-24.04",
              "scripts/test_p8df_official_app_runtime.py",
              "--verify-restart", "P8DH_REAL_HAOS_OFFICIAL_APP_NATIVE_DATA_DURABLE_RESTART_PASS"):
    assert token in flow,token
for token in (
 "P8DH_REAL_OFFICIAL_APP_CONTAINER_WRITE_NATIVE_DATA_PASS",
 "P8DH_REAL_OFFICIAL_APP_STOP_RESTART_RESTORED_AND_WRITTEN_DATA_SURVIVE_PASS",
 "podman", "stop", "podman", "start",
 "state_path.read_text().strip()==payload",
 "state_path=data_dir", "podman", "exec", "--network", "none",
 "P8DF_REAL_RESTORED_FILE_EDITOR_MARKER_VISIBLE_INSIDE_RUNNING_OFFICIAL_IMAGE_PASS",
 "Never promote milestone 6"):
    assert token in source,token
for banned in ("pull_request_target:","workflow_dispatch:",
               "sudo podman","--privileged","--network=host","192.168.",
               "pct exec","qm stop 100"):
    assert banned not in flow+source,banned
assert 'P8DH_REAL_HAOS_OFFICIAL_APP_NATIVE_DATA_DURABLE_RESTART_PASS' not in source
print("P8DH_REAL_OFFICIAL_APP_NATIVE_DATA_WRITE_AND_RESTART_CONTRACT_PASS")
