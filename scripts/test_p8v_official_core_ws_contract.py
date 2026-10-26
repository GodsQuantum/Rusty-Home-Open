#!/usr/bin/env python3
"""Static fail-closed source contract for true Core WS -> Rust Moli CI proof."""
from pathlib import Path
import json

R=Path(__file__).resolve().parent.parent
script=(R/"scripts/run_p8v_genuine_core_ws_to_moli.sh").read_text()
core=(R/"scripts/test_p8v_genuine_core_ws.py").read_text()
workflow=(R/".github/workflows/core-interop-public.yml").read_text()
pins=json.loads((R/"config/ha-core-pins.json").read_text())
assert f'home-assistant@{pins["stable"]["oci_index"]}' in script
assert "docker run -d --rm --name" in script and "--network none" in script
assert "--network \"container:$CONTAINER\"" in script, "no host routing or published port needed"
assert "-p \"127.0.0.1" not in script and "docker network create" not in script
assert "HostPort" in script and ".HostConfig.NetworkMode" in script
assert "P8W_CORE_AUTHENTICATED_WS_8_UPDATES_AND_INVALID_LOGIN_DENIED_PASS" in script
assert "ws://127.0.0.1:8123/api/websocket \"$ENTITY\" on 8" in script
assert "d[\"changes_observed\"]>=8" in script
assert "p8w-auth-denied" in core and "p8w-auth-denied" in script
assert "auth_invalid" in core and "p8w-invalid-fictional-token" in core
assert "HARS_LAB_WS_TOKEN" in script and "p8v-token" in script
assert 'HARS_LAB_WS_TOKEN="$TOKEN"' not in script
assert "--lab-ha-ws-to-moli" in script
# New actual Core reconnection is a distinct bounded laboratory behavior.
# It must revoke the former native Hub, reject invalid login, resubscribe
# and resnapshot into an independently constructed Hub without commands.
native=(R/"crates/native-kernel/src/ha_ws_runtime_lab.rs").read_text()
for key in ("prove_core_reauth", "into_revoked_session", "resync_loopback",
            "EventConverger::start", "failed_epoch", "new_hub_verified",
            "fresh.invalidate().await"):
    assert key in native,key
assert "--lab-ha-ws-reauth-recovery" in script
assert "P8BH_REAL_CORE_WS_REVOKE_BAD_AUTH_NEW_EPOCH_NATIVE_HUB_PASS" in script
assert 'd["recovered_epoch"]==3' in script
assert "bounded_retry_policy" in native
assert "resync_loopback_bounded(url, token, 3)" in native
session=(R/"crates/ha-compat/src/session.rs").read_text()
for required in ("pub async fn resync_loopback_bounded", "explicitly revoke the old Core session",
                 "bounded Core loopback reconnect exhausted",
                 "bounded_retry_survives_real_loopback_tcp_outage_with_new_auth_epoch",
                 "bounded_retry_denies_live_session_and_exhaustion_leaves_no_state"):
    assert required in session,required
assert 'd["bounded_retry_policy"]' in script
assert "write_commands: 0" in native

assert "--cap-drop ALL" in script and "no-new-privileges" in script and "--read-only" in script
assert "P8V_GENUINE_OFFICIAL_CORE_WEBSOCKET_AUTH_TO_ACTUAL_RUST_MOLI_PASS" in script
assert "P8V_GENUINE_CORE_TO_RUST_ROLLBACK_PASS" in script
assert "cargo build --locked --quiet -p hars-native-kernel" in script
assert "ha_registry_stager" in script and "rollback" in script
for part in ("from homeassistant", "hass.auth.async_create_access_token",
             "hass.auth.async_create_refresh_token", "hass.config_entries.flow.async_init",
             "registry.async_get_or_create", "hass.states.async_set",
             ".storage/core.entity_registry", "P8V_NETWORK_NONE",
             "P8V_REAL_CORE_HTTP_WEBSOCKET_REGISTRY_AND_AUTH_READY"):
    assert part in core,part
for part in ("--network none", "test_p8v_official_core_ws_contract.py",
             "run_p8v_genuine_core_ws_to_moli.sh","timeout-minutes: 45"):
    assert part in workflow+script,part
assert "run_p8v_genuine_core_ws_to_moli.sh" not in (R/".github/workflows/p8c2-haos-qemu.yml").read_text()
assert not (R/"scripts/test_p8v_genuine_core_ws.py").is_symlink()
print("PASS: immutable official Core, genuine Core user WS auth, Docker no-egress namespace,")
print("      private synthetic token, native Moli identity/state proof, durable rollback; no home")
