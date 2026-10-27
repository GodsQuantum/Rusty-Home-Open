#!/usr/bin/env python3
from pathlib import Path
r=Path(__file__).resolve().parents[1]
manager=(r/"crates/native-hub/src/bin/oci_app_manager_lab.rs").read_text()
script=(r/"scripts/test_p8cr_atomic_pair_install.sh").read_text()
workflow=(r/".github/workflows/p8cr-atomic-pair-install.yml").read_text()
for marker in (
    "lock_dependency_graph", "lock_app_install", "fn publish_pair", "fn install_pair",
    "podman(&[\"image\", \"exists\", dependent_image])",
    "written.push(path.clone())", "create_new(true)", "File::open(dir)?.sync_all()?",
    "reload()?;", "--install-pair-dependent-lab",
    "atomic_pair_real_filesystem_and_reload_failure",
):
    assert marker in manager, marker
for marker in (
    "GITHUB_ACTIONS", "RUNNER_TEMP", "HARS_DISPOSABLE_OCI_LAB=1",
    "P8CR_SECOND_IMAGE_PREFLIGHT_REFUSES_BOTH_APP_DESCRIPTORS_PASS",
    "P8CR_ATOMIC_TWO_APP_NATIVE_DESCRIPTORS_INERT_AND_PRIVATE_PASS",
    "P8CR_DEPENDENT_ONLY_START_TWO_REAL_ISOLATED_REDIS_APPS_PASS",
    "P8CR_ORDERED_UNINSTALL_RETAINS_BOTH_PRIVATE_DATABASE_VOLUMES_PASS",
):
    assert marker in script, marker
for forbidden in ("pct ", "/dev/tty", "sudo systemctl", "--privileged", "host network"):
    assert forbidden not in script, forbidden
assert "runs-on: ubuntu-24.04" in workflow
assert "actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1" in workflow
print("P8CR_ATOMIC_PAIR_SCOPE_CONTRACT_PASS")
