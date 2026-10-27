#!/usr/bin/env python3
"""P8df CI must execute official real image on restored HAOS bytes, no host ports."""
from pathlib import Path
r=Path(__file__).resolve().parents[1]
w=(r/'.github/workflows/p8df-official-app-oci-runtime.yml').read_text()
code=(r/'scripts/test_p8df_official_app_runtime.py').read_text()
for expected in (
"haos_ova-18.3.qcow2.xz",
"fae6a728768cc10aff60d4820bfcd40d64cd77fab82c8bd92af13b3d9d414090",
"90fa213c3441e585d5a60387e9244d0ed38efaf8",
"-snapshot -accel kvm","runs-on: ubuntu-24.04",
"test_p8cw_real_app_archive_native.py",
"test_p8cy_real_supervisor_app_to_native_files.py",
"test_p8df_official_app_runtime.py",
"podman uidmap slirp4netns fuse-overlayfs",
"P8DF_REAL_HAOS_APP_DATA_TO_OFFICIAL_ROOTLESS_IMAGE_RUNTIME_PASS"):
 assert expected in w,expected
for expected in (
"ghcr.io/home-assistant/app-configurator:6.3.0",
"app-source-ingest","app-source-verify","app-source-stage-inactive",
"--network","none","--read-only","--cap-drop","ALL",
"--security-opt","no-new-privileges","--pids-limit",
"--memory","--mount","destination=/data",
"podman","run","podman","exec","podman","rm",
"HostConfig.NetworkMode","HostConfig.PortBindings","HostConfig.Privileged",
"P8DF_OFFICIAL_HAOS_FILE_EDITOR_CONTAINER_ROOTLESS_NO_NETWORK_START_PASS",
"P8DF_REAL_RESTORED_FILE_EDITOR_MARKER_VISIBLE_INSIDE_RUNNING_OFFICIAL_IMAGE_PASS",
):
 assert expected in code,expected
for forbidden in ("pull_request_target:", "workflow_dispatch:", "sudo podman",
                  "--network=host", "--privileged", "-p 8099", "192.168.","pct exec","qm stop 100"):
 assert forbidden not in w+code,forbidden
assert "GITHUB_ACTIONS" in code and "RUNNER_TEMP" in code and "os.geteuid()!=0" in code
print("P8DF_OFFICIAL_IMAGE_ROOTLESS_APP_DATA_CI_SCOPE_CONTRACT_PASS")
