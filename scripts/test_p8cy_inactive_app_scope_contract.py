#!/usr/bin/env python3
"""P8cy evidence must prove native inactive files, never App activation."""
from pathlib import Path
root=Path(__file__).resolve().parents[1]
w=(root/'.github/workflows/p8cy-official-app-native-inactive.yml').read_text()
rust=(root/'crates/interop-moli/src/app_backup_stage.rs').read_text()
cli=(root/'crates/interop-moli/src/bin/ha_registry_stager.rs').read_text()
qemu=(root/'scripts/test_p8cy_real_supervisor_app_to_native_files.py').read_text()
syn=(root/'scripts/test_p8cy_native_inactive_app_stage.py').read_text()
for required in (
 "haos_ova-18.3.qcow2.xz",
 "fae6a728768cc10aff60d4820bfcd40d64cd77fab82c8bd92af13b3d9d414090",
 "90fa213c3441e585d5a60387e9244d0ed38efaf8",
 "-snapshot -accel kvm", "runs-on: ubuntu-24.04",
 "p8cu_official_file_editor_partial_qemu_pytest.py",
):
 assert required in w,required
for required in ("test_p8cy_inactive_app_scope_contract.py",
 "test_p8cy_native_inactive_app_stage.py",
 "test_p8cy_real_supervisor_app_to_native_files.py",
 "cargo clippy --locked -p hars-interop-moli --all-targets -- -D warnings",
 "P8CY_REAL_OFFICIAL_HAOS_APP_ONLY_TO_INACTIVE_NATIVE_DATA_CERTIFIED_LAB_PASS"):
 assert required in w,required
for required in ("O_NOFOLLOW","renameat2","RENAME_NOREPLACE",
                 "fs::set_permissions(temp.path(), Permissions::from_mode(0o700))",
                 "private_owned_directory","verify_content_address",
                 "app_runtime_started: false","production_ready: false",
                 "HARS_P8CY_FAIL_BEFORE_COMMIT"):
 assert required in rust,required
for required in ("app-source-stage-inactive","stage_inactive_app",
                 "app_runtime_started","automation_executed"):
 assert required in cli,required
for required in ("P8CY_REAL_OFFICIAL_FILE_EDITOR_APP_BYTES_INACTIVE_PRIVATE_NATIVE_STAGE_PASS",
 "P8CY_REAL_SUPERVISOR_NATIVE_APP_DATA_INJECTED_FAULT_ATOMIC_ZERO_LEAK_PASS",
 "P8CY_REAL_SUPERVISOR_PRIVATE_NATIVE_FILES_PERSIST_AFTER_ESCROW_TAMPER_PASS",
 "P8CY_REAL_APP_DATA_STAGED_ONLY_NOT_EXECUTED_OR_CONNECTED_PASS"):
 assert required in qemu,required
for required in ("P8CY_NATIVE_APP_STAGE_INJECTED_FAILURE_ROLLS_BACK_ZERO_FILES_PASS",
 "P8CY_NATIVE_APP_INACTIVE_PRIVATE_ATOMIC_PUBLISH_NO_OVERWRITE_PASS",
 "P8CY_NATIVE_APP_SHA_REFUSAL_BAD_UID_TARGET_SYMLINK_ORIGINAL_STAGE_UNCHANGED_PASS"):
 assert required in syn,required
for forbidden in ("pull_request_target:", "workflow_dispatch:", "sudo podman",
                  "qm stop 100", "pct exec", "192.168."):
 assert forbidden not in w+qemu+syn,forbidden
print("P8CY_REAL_OFFICIAL_APP_TO_ATOMIC_INACTIVE_PRIVATE_NATIVE_FILES_CONTRACT_PASS")
