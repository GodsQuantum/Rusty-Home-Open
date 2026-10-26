#!/usr/bin/env python3
"""Scope guard: third genuine stateful PostgreSQL restoration, rootless/offline."""
from pathlib import Path
r=Path(__file__).resolve().parent.parent
unit=(r/"crates/native-hub/src/oci_quadlet.rs").read_text()
manager=(r/"crates/native-hub/src/bin/oci_app_manager_lab.rs").read_text()
test=(r/"scripts/test_p8ay_postgresql17_independent_restore.sh").read_text()
ci=(r/".github/workflows/p8aw-custom-volume-registry.yml").read_text()
for v in ("fn bind_native_runtime_tmpfs", 'strip_prefix("/var/run/")',
          'strip_prefix("/run/")', "Tmpfs=", "native_runtime_tmpfs_is_run_only"):
    assert v in unit,v
for v in ('"--install-path-tmpfs-lab"', "bind_native_runtime_tmpfs", "runtime_tmpfs: Some(&args[6])"):
    assert v in manager,v
for v in ("postgres:17-alpine", 'POSTGRES_PASSWORD="p8ay-disposable-not-real"',
          "--network none --read-only --cap-drop ALL",
          "/var/lib/postgresql/data", "/var/run/postgresql", "pg_isready",
          "SELECT count(*) FROM rusty_p8ay_checkpoint", "podman volume export --output",
          'podman volume import "$restore_vol" "$ARCHIVE"', "--install-path-tmpfs-lab",
          "P8AY_THIRD_DISTINCT_REAL_PG17_APP_SQL_ROLLBACK_POINT_IN_TIME_PASS",
          "P8AY_PG17_INDEPENDENT_DATABASE_APP_REMOVE_PRESERVES_OWN_DATA_PASS"):
    assert v in test,v
assert "HARS_DISPOSABLE_OCI_LAB=1" in test
for v in ("rh-pg-sql-proof", "rh-pg-sql-restore", '{{.HostConfig.NetworkMode}}'):
    assert v in test,v
assert "sudo podman" not in test and "192.168." not in test
for v in ("test_p8ay_postgres_sql_restore_contract.py",
          "bash -n scripts/test_p8ay_postgresql17_independent_restore.sh",
          "bash scripts/test_p8ay_postgresql17_independent_restore.sh"):
    assert v in ci,v
print("P8AY_THIRD_REAL_SQL_DATABASE_APP_ROOTLESS_NO_NETWORK_RESTORE_SCOPE_PASS")
