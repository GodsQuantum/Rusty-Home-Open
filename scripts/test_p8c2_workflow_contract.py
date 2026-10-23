#!/usr/bin/env python3
"""Safeguard: P8c2 must test REAL disposable HAOS in CI, never user's synthetic VM."""
from pathlib import Path
ROOT = Path(__file__).resolve().parent.parent
action = ROOT / ".github/workflows/p8c2-haos-qemu.yml"
test = ROOT / "scripts/p8c2_real_haos_supervisor_pytest.py"
assert action.is_file(), "missing QEMU CI workflow"
assert test.is_file(), "missing real HAOS guest test"
config = action.read_text()
code = test.read_text()
assert "workflow_dispatch:" in config, "expensive image boot must be opt-in"
assert "group: hars-p8c2-real-haos-" + chr(36) + "{{ github.ref }}" in config, "QEMU CI must isolate independent pull requests"
assert "90fa213c3441e585d5a60387e9244d0ed38efaf8" in config
assert "fae6a728768cc10aff60d4820bfcd40d64cd77fab82c8bd92af13b3d9d414090" in config
assert "haos_ova-18.3.qcow2.xz" in config
assert "qemu-img resize -f qcow2 haos-upstream/tests/haos.qcow2 64G" in config
assert 'jq -e \'.["virtual-size"] >= 68719476736\'' in config
assert "test_hars_manager_role_app" in config
assert "sha256sum -c" in config
assert "qemu-system-x86" in config
assert "-snapshot" in config
assert "scripts/p8c2_real_haos_supervisor_pytest.py" in config
assert "test_start_supervisor" in config
for lifecycle in ("test_check_supervisor", "test_supervisor_is_updated",
                  "test_app_install", "test_create_backup",
                  "test_app_uninstall", "test_restore_backup"):
    assert "supervisor_test/test_supervisor.py::" + lifecycle in config, lifecycle
assert "ha store info --no-progress --raw-json" in code
assert "jq -nr --rawfile" in code
assert "gzip -c | base64" not in code
assert (ROOT / "scripts/p8c2_store_codec.py").is_file()
assert "https://github.com/zigbee2mqtt/hassio-zigbee2mqtt" in code
assert "ha store add {new_repository}" in code
assert "HARS_SUPERVISOR_PARSER" in code
assert "parse_official_store" in config
assert "test_hars_real_supervisor_store" in code
assert "test_hars_rust_supervisor_direct" in code
assert "P8H_BACKUP_SLUG=" in code
assert "restore-partial core_ssh" in code
assert "guest_supervisor_managed_preview" in config
assert (ROOT / "scripts/test_p8h_operations_contract.py").is_file()
assert "P8G_REAL_RUST_MANAGER_STOP_PASS" in code
assert "P8G_REAL_RUST_MANAGER_START_PASS" in code
assert "guest_supervisor_lab_lifecycle" in config
assert (ROOT / "scripts/test_p8g_mutation_contract.py").is_file()

assert "test_hars_real_product_app_install_boot" in code
assert '"ha apps restart " + app' in code, "actual product must survive real Supervisor restart"
assert 'docker inspect -f \'{{.State.StartedAt}}\' app_local_rusty_home' in code, "restart must verify a new container start"
assert 'REAL_RUSTY_HOME_RESTART_HAOS_PASS' in code, "restart success evidence must be explicit"

assert "test_hars_real_product_app_install_boot" in config
assert "cargo build --release --locked -p ha-rust-bridge" in config
assert "packaging/product-app/**" in config
assert '"rusty-home"' in code
assert "test_hars_rust_supervisor_direct" in config
assert "guest_supervisor_live" in config
assert "x86_64-unknown-linux-musl" in config
assert "P8E_GUEST_PROOF" in code
assert "scripts/check_p8d_junit.py" in config
assert (ROOT / "scripts/check_p8d_junit.py").is_file()
assert "qm" not in config and "pct " not in config
assert "lab-guest" not in config.lower()
print("PASS P8c2 guard: pinned official HAOS image and harness, opt-in isolated QEMU only")
