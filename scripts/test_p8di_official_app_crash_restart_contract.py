#!/usr/bin/env python3
"""P8di scope: real rootless OCI SIGKILL + fresh image process, never VM100."""
from pathlib import Path
r=Path(__file__).resolve().parents[1]
w=(r/".github/workflows/p8df-official-app-oci-runtime.yml").read_text()
base=(r/"scripts/test_p8df_official_app_runtime.py").read_text()
code=(r/"scripts/test_p8di_official_app_crash_restart.py").read_text()
for token in ("test_p8di_official_app_crash_restart.py",
              "test_p8di_official_app_crash_restart_contract.py",
              "haos_ova-18.3.qcow2.xz", "test_p8df_official_app_runtime.py",
              "test_p8cu_genuine_file_editor_app_only_archive.py"):
    assert token in w,token
for token in ("verify_restart(run,name,launch,data_dir,markers[0],marker,archive,sha,identity)",
              "ghcr.io/home-assistant/app-configurator:6.3.0",
              '"--network","none"', '"--read-only"', '"--cap-drop","ALL"',
              '"--security-opt","no-new-privileges"', "destination=/data",
              "podman", "rm"):
    assert token in base,token
for token in ("GITHUB_ACTIONS", "RUNNER_TEMP", "os.geteuid() != 0",
              '"podman", "kill"', '"KILL"', '"podman", "start"',
              '"podman", "rm"', "run(launch", "marker_path.read_bytes()",
              "hashlib.file_digest", "umask 077;", "P8DI_REAL_OFFICIAL_APP_PRIVATE_RW_VOLUME_WRITE_PASS",
              "P8DI_GENUINE_OFFICIAL_APP_SIGKILL_PROCESS_STOPPED_VOLUME_INTACT_PASS",
              "P8DI_GENUINE_OFFICIAL_APP_RESTART_RESTORED_AND_NEW_BYTES_PERSIST_PASS",
              "P8DI_NEW_ROOTLESS_OFFLINE_APP_CONTAINER_SAME_PINNED_IMAGE_DATA_SURVIVES_PASS",
              "P8DI_ORIGINAL_GENUINE_HAOS_APP_BACKUP_UNMODIFIED_NO_INGRESS_CLAIM_PASS"):
    assert token in code,token
for forbidden in ("pull_request_target:", "workflow_dispatch:", "sudo podman",
                  "--network=host", "--privileged", "192.168.", "pct exec",
                  "qm stop", "qm start", "systemctl reboot"):
    assert forbidden not in w+base+code,forbidden
assert "real_app_image_boot" not in code
assert "production_ready = True" not in code
print("P8DI_GENUINE_APP_SIGKILL_RESTART_OFFLINE_SCOPE_CONTRACT_PASS")
