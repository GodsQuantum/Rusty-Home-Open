#!/usr/bin/env bash
# Genuine pinned Core WS -> read-only Rust Moli, wholly within a Docker
# network namespace with NO network: no published ports or internet egress.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
[[ "$(id -u)" != "0" && "$(printenv GITHUB_ACTIONS || true)" == "true" ]] || {
  echo "Unprivileged GitHub Actions laboratory only" >&2; exit 2;
}
mkdir -p runtime
TMP="$(mktemp -d "$ROOT/runtime/hars-p8v-core-XXXXXXXX")"
STORE="$(mktemp -d "$ROOT/runtime/hars-p8v-shadow-XXXXXXXX")"
chmod 700 "$TMP" "$STORE"
CONTAINER=""
cleanup() {
  if [[ -n "$CONTAINER" ]]; then docker rm -f "$CONTAINER" >/dev/null 2>&1 || true; fi
  rm -rf -- "$TMP" "$STORE"
}
trap cleanup EXIT
mkdir -p "$TMP/config/custom_components/hars_lifecycle_probe"
chmod 700 "$TMP/config" "$TMP/config/custom_components" "$TMP/config/custom_components/hars_lifecycle_probe"
cp scripts/fixtures/ha-config-lifecycle/{manifest.json,__init__.py,config_flow.py} "$TMP/config/custom_components/hars_lifecycle_probe/"
cat > "$TMP/config/configuration.yaml" <<'YAML'
homeassistant:
  name: Genuine Core to Rust/Moli WS synthetic laboratory
  time_zone: Europe/Paris
http:
  server_host: 0.0.0.0
YAML
chmod 600 "$TMP/config/configuration.yaml"
IMAGE='ghcr.io/home-assistant/home-assistant@sha256:33afb0659d786213264c2211f8d91c5fe87461c56eba6a3a546eb05fc71e89f4'
if ! docker image inspect "$IMAGE" >/dev/null 2>&1; then docker pull "$IMAGE" >/dev/null; fi
if ! docker image inspect ubuntu:24.04 >/dev/null 2>&1; then docker pull ubuntu:24.04 >/dev/null; fi
# Build BEFORE Core startup so the short-lived Core fixture cannot expire
# while the Rust client is compiling.
cargo build --locked --quiet -p hars-native-kernel --bin hars-native-kernel
cargo build --locked --quiet -p hars-interop-moli --bin ha_registry_stager
CONTAINER="$(docker run -d --rm --name "$(basename "$TMP")" --network none \
    --user "$(id -u):$(id -g)" --read-only \
    --cap-drop ALL --security-opt no-new-privileges --pids-limit 256 \
    --memory 2400m --cpus 2 \
    --tmpfs /tmp:rw,nosuid,nodev,size=128m \
    --tmpfs /run:rw,nosuid,nodev,size=64m \
    -e PYTHONDONTWRITEBYTECODE=1 -e TZ=Europe/Paris \
    -e P8V_NETWORK_NONE=verified \
    --mount "type=bind,src=$TMP/config,dst=/config" \
    --mount "type=bind,src=$ROOT/scripts/test_p8v_genuine_core_ws.py,dst=/p8v-real-core.py,readonly" \
    --entrypoint python3 "$IMAGE" /p8v-real-core.py)"
[[ -n "$CONTAINER" ]]
# No network, no host ports, no external routes. A second ephemeral Ubuntu
# process joins ONLY this namespace to access Core via 127.0.0.1.
docker inspect "$CONTAINER" --format '{{json .HostConfig.NetworkMode}} {{json .NetworkSettings.Ports}}' | \
  python3 -c 'import sys; x=sys.stdin.read(); assert x.startswith("\"none\" ") and not "HostPort" in x'
READY=0
for _ in $(seq 1 100); do
  if [[ -s "$TMP/config/p8v-ready" ]]; then READY=1; break; fi
  [[ "$(docker inspect -f '{{.State.Running}}' "$CONTAINER" 2>/dev/null || true)" == true ]] || {
    echo "Official Core exited before WebSocket readiness" >&2
    if [[ -f "$TMP/config/p8v-error.txt" ]]; then tail -55 "$TMP/config/p8v-error.txt" >&2; fi
    exit 1
  }
  sleep 1
done
[[ "$READY" == 1 ]] || { echo "Official Core WebSocket readiness timeout" >&2; exit 1; }
[[ -s "$TMP/config/.storage/core.entity_registry" && -s "$TMP/config/p8v-token" && -s "$TMP/config/p8v-entity-id" && -s "$TMP/config/p8w-auth-denied" ]]
REGISTRY="$TMP/config/.storage/core.entity_registry"
STAGED="$("$ROOT/target/debug/ha_registry_stager" --lab-readonly "$STORE" stage "$REGISTRY")"
SHA="$(printf '%s' "$STAGED" | python3 -c 'import json,sys; d=json.load(sys.stdin);assert d["entities"]>=1 and d["read_only"] and not d["production_ready"];print(d["sha256"])')"
"$ROOT/target/debug/ha_registry_stager" --lab-readonly "$STORE" activate "$SHA" none >/dev/null

# RUNNER: no external network, no host ports, only the fixture and binary
# mounts. The synthetic token never appears in any Docker command argument
# or CLI argv: read it inside this isolated shell.
RESULT="$(docker run --rm --network "container:$CONTAINER" \
    --user "$(id -u):$(id -g)" --read-only --cap-drop ALL \
    --security-opt no-new-privileges --pids-limit 96 --memory 512m --cpus 1 \
    --tmpfs /tmp:rw,nosuid,nodev,size=32m \
    --workdir /lab \
    --mount "type=bind,src=$ROOT/target/debug/hars-native-kernel,dst=/lab/hars-native-kernel,readonly" \
    --mount "type=bind,src=$ROOT/runtime,dst=/lab/runtime" \
    --mount "type=bind,src=$TMP/config,dst=/config,readonly" \
    --env "HARS_LAB_STORE=$(basename "$STORE")" \
    --entrypoint /bin/sh ubuntu:24.04 -ec '
      test -s /config/p8v-token && test -s /config/p8v-entity-id
      export HARS_LAB_WS_TOKEN="$(cat /config/p8v-token)"
      ENTITY="$(cat /config/p8v-entity-id)"
      exec /lab/hars-native-kernel --lab-ha-ws-to-moli \
        "/lab/runtime/$HARS_LAB_STORE" \
        ws://127.0.0.1:8123/api/websocket "$ENTITY" on 8
    ')"
printf '%s' "$RESULT" | python3 -c 'import json,sys; d=json.load(sys.stdin); assert d["source"]=="loopback_hass_rs" and d["states_verified"]==1 and d["changes_observed"]>=8 and d["requested_state_matched"] and d["read_only"] and not d["production_ready"]'
# Second genuine-Core proof: revoke, deny one bad reauth, then rebuild an
# authenticated subscription, snapshot and native Moli Hub on a fresh epoch.
RECOVERY="$(docker run --rm --network "container:$CONTAINER" \
    --user "$(id -u):$(id -g)" --read-only --cap-drop ALL \
    --security-opt no-new-privileges --pids-limit 96 --memory 512m --cpus 1 \
    --tmpfs /tmp:rw,nosuid,nodev,size=32m \
    --workdir /lab \
    --mount "type=bind,src=$ROOT/target/debug/hars-native-kernel,dst=/lab/hars-native-kernel,readonly" \
    --mount "type=bind,src=$ROOT/runtime,dst=/lab/runtime" \
    --mount "type=bind,src=$TMP/config,dst=/config,readonly" \
    --env "HARS_LAB_STORE=$(basename "$STORE")" \
    --entrypoint /bin/sh ubuntu:24.04 -ec '
      export HARS_LAB_WS_TOKEN="$(cat /config/p8v-token)"
      ENTITY="$(cat /config/p8v-entity-id)"
      exec /lab/hars-native-kernel --lab-ha-ws-reauth-recovery \
        "/lab/runtime/$HARS_LAB_STORE" \
        ws://127.0.0.1:8123/api/websocket "$ENTITY" on
    ')"
printf '%s' "$RECOVERY" | python3 -c 'import json,sys; d=json.load(sys.stdin); assert d["first_epoch"]==1 and d["failed_epoch"]==2 and d["recovered_epoch"]==3 and d["old_state_revoked"] and d["new_hub_verified"] and d["bounded_retry_policy"] and d["write_commands"]==0 and not d["production_ready"]'
echo P8BH_REAL_CORE_WS_REVOKE_BAD_AUTH_NEW_EPOCH_NATIVE_HUB_PASS
"$ROOT/target/debug/ha_registry_stager" --lab-readonly "$STORE" rollback "$SHA" >/dev/null
STATUS="$("$ROOT/target/debug/ha_registry_stager" --lab-readonly "$STORE" status | python3 -c 'import json,sys; print(int(json.load(sys.stdin)["active"] is None))')"
test "$STATUS" == 1
touch "$TMP/config/p8v-stop"
echo P8W_CORE_AUTHENTICATED_WS_8_UPDATES_AND_INVALID_LOGIN_DENIED_PASS
echo P8V_GENUINE_OFFICIAL_CORE_WEBSOCKET_AUTH_TO_ACTUAL_RUST_MOLI_PASS
echo P8V_GENUINE_CORE_TO_RUST_ROLLBACK_PASS
