#!/usr/bin/env python3
"""P8cp fail-closed scope: real three-App crash, not P8co normal stop."""
from pathlib import Path

root = Path(__file__).resolve().parent.parent
script = (root / "scripts/test_p8cp_three_app_crash_chain.sh").read_text()
workflow = (root / ".github/workflows/p8cp-three-app-crash-chain.yml").read_text()
manager = (root / "crates/native-hub/src/bin/oci_app_manager_lab.rs").read_text()
for value in (
    "GITHUB_ACTIONS", "RUNNER_TEMP", "podman kill --signal KILL rh-p8cp-root",
    "systemctl --user show -P NRestarts", "--install-dependent-lab",
    "--remove-lab p8cp-middle", "--start-lab p8cp-leaf",
    "p8cp-root", "p8cp-middle", "p8cp-leaf",
    "NetworkMode", "ReadonlyRootfs", "redis-cli SAVE",
    "P8CP_UNEXPECTED_ROOT_SIGKILL_CASCADES_TWO_DEPENDENTS_PASS",
    "P8CP_THREE_APP_RECOVERY_PRESERVES_DISTINCT_REDIS_RDB_PASS",
):
    assert value in script, value
for value in ("ubuntu-24.04", "test_p8cp_three_app_crash_chain.sh", "cargo test --locked",
              "cargo clippy --locked", "timeout-minutes: 13"):
    assert value in workflow, value
for value in ("lock_dependency_graph", "walk_dependency_chain",
              "bind_native_service_dependency", "provider_has_dependents"):
    assert value in manager, value
for forbidden in ("pct ", "qm ", "systemctl restart ", "192.168.1.", "sudo podman",
                  "podman volume rm", "podman system prune", "HAOS_TOKEN"):
    assert forbidden not in script, forbidden
print("P8CP_ISOLATED_THREE_APP_CRASH_SCOPE_CONTRACT_PASS")
