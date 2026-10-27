#!/usr/bin/env python3
"""P8co source guard: this test cannot be redirected to Cloud9/HAOS."""
from pathlib import Path

root = Path(__file__).resolve().parents[1]
script = (root / "scripts/test_p8co_native_dependency_fencing.sh").read_text()
manager = (root / "crates/native-hub/src/bin/oci_app_manager_lab.rs").read_text()
quadlet = (root / "crates/native-hub/src/oci_quadlet.rs").read_text()
workflow = (root / ".github/workflows/p8co-native-dependency-fencing.yml").read_text()
for required in (
    "GITHUB_ACTIONS", "RUNNER_TEMP", "HARS_DISPOSABLE_OCI_LAB=1",
    "podman pull --quiet docker.io/library/redis:7.4-alpine",
    "P8CO_MISSING_PROVIDER_REFUSED_WITHOUT_UNIT_CREATION_PASS",
    "P8CO_NATIVE_PROVIDER_STOP_CASCADES_DEPENDENT_STOP_PASS",
    "P8CO_PROVIDER_REMOVAL_REFUSED_WITH_DEPENDENT_LIVE_PASS",
    "P8CO_DEPENDENCY_RESTART_PRESERVES_BOTH_REAL_DATABASES_PASS",
):
    assert required in script, required
for required in (
    "lock_dependency_graph", "validate_dependency_chain",
    "provider_has_dependents", "--install-dependent-lab",
    "bind_native_service_dependency",
):
    assert required in manager, required
for required in ("BindsTo=rh-{provider}.service", "After=rh-{provider}.service", "ReadOnly=true", "Network=none"):
    assert required in quadlet, required
assert "runs-on: ubuntu-24.04" in workflow
assert "actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1" in workflow
for forbidden in ("pct ", "VM100", "/dev/tty", "sudo systemctl", "host network", "--privileged"):
    assert forbidden not in script, forbidden
print("P8CO_SCOPE_AND_NATIVE_DEPENDENCY_CONTRACT_PASS")
