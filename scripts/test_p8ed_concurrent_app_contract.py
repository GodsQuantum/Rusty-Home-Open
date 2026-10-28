#!/usr/bin/env python3
"""P8ed new functional real dual-App runtime contract. CI proves behavior."""
import ast
from pathlib import Path
r=Path(__file__).resolve().parents[1]
s=(r/"scripts/test_p8ed_genuine_two_apps_concurrent.py").read_text()
w=(r/".github/workflows/p8ed-two-genuine-apps-concurrent.yml").read_text()
ast.parse(s)
for t in (
    "APP_MARKERS", "source_boot_id", "inspect_backup(", "app-source-stage-pair-inactive",
    "atomic_private_pair_published", "generate(options,config,images[1])",
    "allow_anonymous false", "P8ED_TWO_REAL_APP_PROCESSES_SIMULTANEOUS_ROOTLESS_NO_EGRESS_PASS",
    "P8ED_ACTUAL_FILE_EDITOR_SIGKILL_MQTT_STILL_ALIVE_AUTH_RETAINED_PASS",
    "P8ED_REAL_EDITOR_REPLACEMENT_REATTACH_DURABLE_DATA_WITH_MQTT_RUNNING_PASS",
    "P8ED_SOURCE_ARCHIVES_UNCHANGED_BOTH_NATIVE_APP_STATES_PRIVATE_CLEAN_TEARDOWN_PASS",
    "SIGKILL", "sha256(src)==digest", "check_options({\"logins\":[]",
    "Rootless", "GITHUB_ACTIONS", '"--network","none"', '"--read-only"',
    '"--cap-drop","ALL"', '"--security-opt","no-new-privileges"'):
    assert t in s,t
for t in ("p8du_two_real_apps_qemu_pytest.py", "90fa213c3441e585d5a60387e9244d0ed38efaf8",
          "fae6a728768cc10aff60d4820bfcd40d64cd77fab82c8bd92af13b3d9d414090",
          "scripts/test_p8ed_genuine_two_apps_concurrent.py", "--port 18674",
          "--port 18675", "p8ed-two-genuine-apps-concurrent",
          "pexpect==4.9.0", "test -c /dev/kvm", "-snapshot -accel kvm"):
    assert t in w,t
for forbidden in ("192.168.", "qm stop", "pct exec", "SUPERVISOR_TOKEN", "VAULTWARDEN",
                  "pull_request_target:", "workflow_dispatch:", "secrets.", "--privileged",
                  '--network","host"', "sudo podman"):
    assert forbidden not in s+w,forbidden
assert "production_ready" in s
print("P8ED_NEW_OFFICIAL_PAIR_SIMULTANEOUS_RUNTIME_SCOPE_CONTRACT_PASS")
