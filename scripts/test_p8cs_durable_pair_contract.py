#!/usr/bin/env python3
from pathlib import Path
root = Path(__file__).resolve().parents[1]
code = (root / "crates/native-hub/src/bin/oci_app_manager_lab.rs").read_text()
flow = (root / ".github/workflows/p8cs-durable-pair-start.yml").read_text()
shell = (root / "scripts/test_p8cs_durable_pair_start.sh").read_text()
for marker in (
    "PAIR_START_JOURNAL", "fn read_pair_journal(", "fn recover_pair_locked(",
    "fn start_pair(", "fn recover_pair(", "O_NOFOLLOW", "SIGKILL",
    "ensure_no_pair_journal(dir)?", "--recover-start-pair-lab",
    "--start-pair-transaction-lab",
):
    assert marker in code, marker
for marker in (
    "ubuntu-24.04", "scripts/test_p8cs_durable_pair_start.sh",
    "cargo test --locked -p hars-native-hub --all-targets",
    "cargo clippy --locked -p hars-native-hub --all-targets",
):
    assert marker in flow, marker
for marker in (
    "P8CS_GENUINE_COORDINATOR_SIGKILL_LEAVES_DURABLE_INTENT_AND_PARTIAL_RUNTIME_PASS",
    "P8CS_NEW_PROCESS_RECOVERS_INCOMPLETE_PAIR_WITHOUT_REMOVING_VOLUMES_PASS",
    "P8CS_RETRY_RESTORES_BOTH_PERSISTED_REDIS_DATABASES_PASS",
    "P8CS_FORGED_SYMLINK_JOURNAL_REFUSED_WITHOUT_TOUCHING_LIVE_PAIR_PASS",
):
    assert marker in shell, marker
for forbidden in ("pct ", "VM100", "192.168.", "systemctl --system", "sudo podman"):
    assert forbidden not in shell, forbidden
print("P8CS_LAB_SCOPE_AND_DURABLE_JOURNAL_CONTRACT_PASS")
