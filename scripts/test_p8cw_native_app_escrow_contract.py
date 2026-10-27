#!/usr/bin/env python3
"""Scope: only disposable genuine HAOS App-only archive -> inert Rust escrow."""
from pathlib import Path
r=Path(__file__).resolve().parents[1]
w=(r/".github/workflows/p8cw-official-app-native-escrow.yml").read_text()
rust=(r/"crates/interop-moli/src/app_backup_import.rs").read_text()
cli=(r/"crates/interop-moli/src/bin/ha_registry_stager.rs").read_text()
p=(r/"scripts/test_p8cw_real_app_archive_native.py").read_text()
s=(r/"scripts/test_p8cw_native_app_backup_stage.py").read_text()
for mark in (
 "p8cu_official_file_editor_partial_qemu_pytest.py",
 "p8p_cross_vm_backup_relay.py", "haos_ova-18.3.qcow2.xz",
 "fae6a728768cc10aff60d4820bfcd40d64cd77fab82c8bd92af13b3d9d414090",
 "90fa213c3441e585d5a60387e9244d0ed38efaf8",
 "-snapshot -accel kvm","runs-on: ubuntu-24.04",
 "cargo clippy --locked -p hars-interop-moli --all-targets -- -D warnings",
 "cargo build --locked -p hars-interop-moli --bin ha_registry_stager",
 "scripts/test_p8cw_native_app_backup_stage.py",
 "scripts/test_p8cw_real_app_archive_native.py",
 "P8CW_REAL_OFFICIAL_SUPERVISOR_APP_ONLY_INERT_NATIVE_RUST_ESCROW_PASS",
):
 assert mark in w,mark
for mark in ("O_NOFOLLOW", "MAX_DECOMPRESSED", "MultiGzDecoder",
             "canonical(", "StrictValue(", "duplicate", "protected",
             "manifest_partial","core_migrated","app_runtime_migrated",
             "actions_executable","production_ready"):
 assert mark in rust,mark
for mark in ("app-source-ingest","app-source-verify",
             "BackupEscrow::open", "inspect_unprotected_app_only",
             "private_app_backup_escrow_ingest",
             "private_app_backup_escrow_verify"):
 assert mark in cli,mark
for mark in ("sha256", "app_count","app-source-ingest","app-source-verify",
             "P8CW_GENUINE_SUPERVISOR_APP_ONLY_PRIVATE_RUST_SHA_INGEST_PASS",
             "P8CW_APP_ONLY_GENUINE_HAOS_ESCROW_TAMPER_FAIL_CLOSED_ORIGINAL_UNCHANGED_PASS"):
 assert mark in p,mark
for mark in ("inner_symlink","inner_traversal","inner_duplicate",
             "truncated_gzip","outer_duplicate","private.mkdir(mode=0o700)"):
 assert mark in s,mark
for banned in ("pull_request_target:", "workflow_dispatch:", "sudo podman",
               "qm stop 100","pct exec","192.168.","GITHUB_TOKEN"):
 assert banned not in w+p+s,banned
print("P8CW_OFFICIAL_GENUINE_HAOS_APP_TO_EXISTING_NATIVE_ESCROW_CONTRACT_PASS")
