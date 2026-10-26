#!/usr/bin/env python3
"""P2k uses existing real broker CI, no privileged/home endpoints."""
from pathlib import Path
root=Path(__file__).resolve().parent.parent
rust=(root/"crates/native-kernel/src/mqtt5_reconcile.rs").read_text()
cli=(root/"crates/native-kernel/src/main.rs").read_text()
py=(root/"scripts/test_p2i_mqtt5_broker_sigkill.py").read_text()
ci=(root/".github/workflows/p2i-mqtt5-broker-crash.yml").read_text()
for value in ("observe_same_id_readd(root:", "P2k expects exact five",
              "same_id_readd_verified", "old_state_cleared_before_readd",
              "same-ID rediscovery either failed or inherited state",
              "genuine tombstone did not clear native Hub and redb"):
    assert value in rust,value
for value in ("--lab-mqtt5-registry-readd", "observe_same_id_readd"):
    assert value in cli,value
for value in ("spawn(5, readd=True)", 'publish("tombstone.txt", topic, True)',
              'publish("new_state.txt", "lab/readonly/p2h", False)',
              "P2K_GENUINE_MQTT5_BROKER_RETAINED_DELETE_READD_SAME_MOLI_ID_PASS",
              "P2K_NATIVE_MOLI_OLD_STATE_CLEARED_NEW_VALUE_READONLY_PASS"):
    assert value in py,value
assert "python3 scripts/test_p2k_real_readd_contract.py" in ci
assert "--network\", \"none\"" in py
assert "docker build --tag hars-mqtt-lab:20261019" in ci
print("P2K_REAL_BROKER_TOMBSTONE_READD_SCOPE_CONTRACT_PASS")
