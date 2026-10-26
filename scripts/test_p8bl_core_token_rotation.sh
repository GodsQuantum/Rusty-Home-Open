#!/usr/bin/env bash
# Sourced by the official Core P8v fixture, with TMP/STORE/CONTAINER set.
# No outbound network or real tokens/devices. Core AuthManager does revocation.
set -euo pipefail
touch "$TMP/config/p8bl-rotate-request"
ROTATED=0
for _ in $(seq 1 120); do
  if [[ -s "$TMP/config/p8bl-rotate-ready" && -s "$TMP/config/p8bl-token" ]]; then ROTATED=1; break; fi
  [[ "$(docker inspect -f '{{.State.Running}}' "$CONTAINER" 2>/dev/null || true)" == true ]] || {
     echo "Genuine Core exited before rotation" >&2; exit 1;
  }
  sleep 0.1
done
[[ "$ROTATED" == 1 ]] || { echo "Genuine Core token rotation timed out" >&2; exit 1; }
[[ "$(stat -c %a "$TMP/config/p8bl-token")" == 600 ]]
ROTATION="$(docker run --rm --network "container:$CONTAINER" \
   --user "$(id -u):$(id -g)" --read-only --cap-drop ALL \
   --security-opt no-new-privileges --pids-limit 96 --memory 512m --cpus 1 \
   --tmpfs /tmp:rw,nosuid,nodev,size=32m --workdir /lab \
   --mount "type=bind,src=$ROOT/target/debug/hars-native-kernel,dst=/lab/hars-native-kernel,readonly" \
   --mount "type=bind,src=$ROOT/runtime,dst=/lab/runtime" \
   --mount "type=bind,src=$TMP/config,dst=/config,readonly" \
   --env "HARS_LAB_STORE=$(basename "$STORE")" \
   --entrypoint /bin/sh ubuntu:24.04 -ec '
       export HARS_LAB_WS_TOKEN="$(cat /config/p8v-token)"
       export HARS_LAB_WS_REPLACEMENT_TOKEN="$(cat /config/p8bl-token)"
       ENTITY="$(cat /config/p8v-entity-id)"
       exec /lab/hars-native-kernel --lab-ha-ws-core-token-rotation \
         "/lab/runtime/$HARS_LAB_STORE" \
         ws://127.0.0.1:8123/api/websocket "$ENTITY" on
   ')"
printf '%s' "$ROTATION" | python3 -c 'import json,sys; d=json.load(sys.stdin); assert d["genuine_core_old_refresh_revoked"] and d["old_access_token_refused_by_core"] and d["next_access_token_authenticated"] and d["new_native_hub_verified"] and d["no_old_state_reexposed"] and d["revoked_epoch"]==1 and d["fresh_epoch"]>=2 and d["write_commands"]==0 and not d["production_ready"]'
echo P8BL_GENUINE_CORE_TOKEN_REVOKED_NEW_ACCESS_AUTHENTICATED_ACTUAL_MOLI_PASS
