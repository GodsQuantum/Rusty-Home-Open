#!/usr/bin/env python3
"""Fail-closed scope for real PostgreSQL crash/WAL recovery on disposable CI."""
from pathlib import Path
root=Path(__file__).resolve().parent.parent
test=(root/"scripts/test_p8ay_postgresql17_independent_restore.sh").read_text()
workflow=(root/".github/workflows/p8aw-custom-volume-registry.yml").read_text()
for marker in (
    "INSERT INTO rusty_p8ay_checkpoint VALUES (3,'committed-before-sigkill')",
    "INSERT INTO rusty_p8ay_checkpoint VALUES (4,'must-never-commit')",
    "SELECT pg_sleep(45); COMMIT;",
    "FROM pg_stat_activity WHERE pid <> pg_backend_pid()",
    "podman kill --signal KILL rh-pg-sql-proof",
    'if wait "$open_txn_pid"; then',
    'restart_count="$(systemctl --user show -p NRestarts --value rh-pg-sql-proof.service)"',
    '"$restart_count" -ge 1',
    "P8CA_REAL_PG17_UNCOMMITTED_TRANSACTION_OPEN_AT_KILL_PASS",
    "P8CA_GENUINE_PG17_CONTAINER_SIGKILL_DURING_SQL_TRANSACTION_PASS",
    "P8CA_REAL_POSTGRESQL_WAL_COMMITTED_DURABLE_UNCOMMITTED_ROLLED_BACK_PASS",
    "P8CA_NATIVE_SYSTEMD_RESTART_OFFLINE_READONLY_ORIGINAL_AND_RESTORE_ISOLATED_PASS",
):
    assert marker in test, marker
for marker in (
    '[[ "$(printenv GITHUB_ACTIONS || true)" == true ]] || exit 2',
    '--network none --read-only --cap-drop ALL',
    'test "$(podman inspect rh-pg-sql-proof --format \'{{.HostConfig.NetworkMode}}\')" = none',
    'test "$(podman inspect rh-pg-sql-proof --format \'{{.HostConfig.ReadonlyRootfs}}\')" = true',
    'test "$(podman inspect rh-pg-sql-proof --format \'{{.Config.User}}\')" = 70',
    'test "$(sql rh-pg-sql-restore \'SELECT count(*) FROM rusty_p8ay_checkpoint;\')" = 1',
):
    assert marker in test, marker
assert "sudo podman" not in test and "192.168." not in test
assert "scripts/test_p8ca_postgres_wal_crash_contract.py" in workflow
assert "python3 scripts/test_p8ca_postgres_wal_crash_contract.py" in workflow
assert "bash scripts/test_p8ay_postgresql17_independent_restore.sh" in workflow
print("P8CA_GENUINE_PG17_WAL_CRASH_RECOVERY_CI_ISOLATION_CONTRACT_PASS")
