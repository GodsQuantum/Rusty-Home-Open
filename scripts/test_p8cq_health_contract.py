"""P8cq contract: health readiness is native, opt-in, rootless, isolated."""
from pathlib import Path

manager = Path("crates/native-hub/src/bin/oci_app_manager_lab.rs").read_text()
test = Path("scripts/test_p8cq_health_readiness.sh").read_text()
workflow = Path(".github/workflows/p8cq-native-health-readiness.yml").read_text()
for text in (
    "Notify=healthy", "HealthOnFailure=kill", "HealthCmd={cmd}",
    '"--install-health-dependent-ready-lab"', "bind_native_service_dependency",
    'env::var("GITHUB_ACTIONS")', 'env::var("HARS_DISPOSABLE_OCI_LAB")',
):
    assert text in manager, text
for text in (
    'GITHUB_ACTIONS:-', 'RUNNER_TEMP:-', 'redis-cli -p 6399 ping',
    'podman healthcheck run', 'CLIENT PAUSE 60000 ALL',
    'ActiveEnterTimestampMonotonic', 'NRestarts',
    'P8CQ_REAL_RUNNING_BUT_UNHEALTHY_PROVIDER_BLOCKS_DEPENDENT_START_PASS',
    'P8CQ_REAL_LIVE_REDIS_HEALTH_FAILURE_KILLS_PROVIDER_AND_FENCES_CHILD_PASS',
    'P8CQ_NATIVE_HEALTH_RECOVERY_PRESERVES_TWO_PRIVATE_RDB_KEYS_PASS',
):
    assert text in test, text
assert "runs-on: ubuntu-26.04" in workflow
assert "test_p8cq_health_readiness.sh" in workflow
assert "test_p8cq_health_contract.py" in workflow
print("P8CQ_NATIVE_HEALTH_SCOPE_CONTRACT_PASS")
