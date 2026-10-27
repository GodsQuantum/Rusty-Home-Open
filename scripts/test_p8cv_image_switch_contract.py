#!/usr/bin/env python3
"""Static safety scope for the genuinely executed P8cv CI test."""
from pathlib import Path
m=Path("crates/native-hub/src/bin/oci_app_manager_lab.rs").read_text()
t=Path("scripts/test_p8cv_image_switch_sigkill.sh").read_text()
w=Path(".github/workflows/p8cv-image-switch-crash.yml").read_text()
for term in ("SWITCH_JOURNAL", "read_switch_journal", "recover_switch_locked",
             "O_NOFOLLOW", "HARS_P8CV_KILL_AFTER_SWAP",
             "--recover-image-switch-lab", "ensure_no_pair_journal"):
    assert term in m, term
for term in ("GITHUB_ACTIONS", "RUNNER_TEMP", "id -u", "redis:7.4-alpine",
             "redis:7.4-bookworm", "SIGKILL", "P8CV_SEPARATE_PROCESS_RECOVERS",
             "NetworkMode", "ReadonlyRootfs", "symlink journal accepted"):
    assert term in t, term
assert "runs-on: ubuntu-24.04" in w
assert "scripts/test_p8cv_image_switch_sigkill.sh" in w
assert "VM100" not in t and "pct " not in t and "sudo " not in t
print("P8CV_STATIC_DISPOSABLE_SCOPE_PASS")
