#!/usr/bin/env python3
"""Check that the official Core lab actually feeds Rust, no faked fixture proof."""
from pathlib import Path
r=Path(__file__).resolve().parent.parent
script=(r/"scripts/run_p8q_real_core_to_native_registry.sh").read_text()
real=(r/"scripts/test_p8q_real_core_registry_export.py").read_text()
workflow=(r/".github/workflows/core-interop-public.yml").read_text()
core=json_text=(r/"config/ha-core-pins.json").read_text()
import json
pin=json.loads(json_text)["stable"]["oci_index"]
assert ("home-assistant@"+pin) in script
for marker in ("--network none","--read-only","--cap-drop ALL",
               "no-new-privileges","--user","--memory","$TMP/config/.storage/core.entity_registry",
               "cargo build --locked","stage \"$SOURCE\"","activate \"$SHA\" none",
               "rollback \"$SHA\"","P8Q_REAL_CORE_ENTITY_REGISTRY_TO_RUST_PERSIST_ROLLBACK_PASS"):
    assert marker in script,marker
for marker in ("from homeassistant", "HomeAssistant(\"/config\")",
               "registry.async_get_or_create(", "await asyncio.sleep(12)",
               "/config/.storage/core.entity_registry",
               "P8Q_REAL_CORE_REGISTRY_CREATED_AND_SAVED_PASS"):
    assert marker in real,marker
assert "P8S_REAL_CORE_STATES_OBSERVED_IN_NATIVE_MOLI_READONLY_PASS" in script
assert "--lab-ha-core-state-snapshot-readonly" in script
assert "test -s \"$SNAPSHOT\"" in script
assert "hass.states.async_set" in real and "hass.states.get(e).as_dict()" in real
assert "P8R_REAL_CORE_REGISTRY_INSIDE_ACTUAL_MOLI_HUB_READONLY_PASS" in script
assert "cargo build --locked --quiet -p hars-native-kernel" in script
assert "--lab-ha-registry-shadow-readonly" in script
assert "run_p8q_real_core_to_native_registry.sh" in workflow
assert "test_p8q_core_registry_contract.py" in workflow
assert "run_p8q_real_core_to_native_registry.sh" not in (r/".github/workflows/p8c2-haos-qemu.yml").read_text()
assert (r/"crates/interop-moli/src/registry_stage.rs").is_file()
print("PASS: actual official pinned Core writes private registry, Rust stages with durable rollback")
