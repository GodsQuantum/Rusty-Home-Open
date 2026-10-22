#!/usr/bin/env bash
# Genuine pinned HA Core Python container -> native Rust UDS client.
# Offline fixture only, never production devices/config or credentials.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
[[ "$(id -u)" != "0" && ( "$(hostname)" == "dev-build" || "${GITHUB_ACTIONS:-}" == "true" ) ]] || exit 2
cd "$ROOT"
IMAGE="ghcr.io/home-assistant/home-assistant@sha256:130241f28d01fa80dfa3b44f7d89781c188d8642ecab9024f079f19f9f82baa0"
cargo build --locked --quiet -p hars-python-bridge
mkdir -p runtime
DIR="$(mktemp -d "$ROOT/runtime/ha-core-ipc-XXXXXX")"
chmod 700 "$DIR"
CONTAINER="hars-core-ipc-$$"
cleanup() { docker rm -f "$CONTAINER" >/dev/null 2>&1 || true; rm -rf -- "$DIR"; }
trap cleanup EXIT
printf "%048d" 0 | tr 0 A > "$DIR/capability"
chmod 600 "$DIR/capability"
docker run --detach --name "$CONTAINER" --network none \
 --user "$(id -u):$(id -g)" --read-only --cap-drop ALL \
 --security-opt no-new-privileges --pids-limit 128 --memory 1g --cpus 2 \
 --tmpfs /tmp:rw,nosuid,nodev,size=64m \
 --mount "type=bind,src=$DIR,dst=/bridge" \
 --mount "type=bind,src=$ROOT/scripts/python_sidecar_fixture.py,dst=/fixture.py,readonly" \
 --entrypoint python3 "$IMAGE" /fixture.py /bridge/mock.sock /bridge/capability --core-states >/dev/null
ready=0
for _ in $(seq 1 120); do
 if [[ -S "$DIR/mock.sock" ]]; then ready=1; break; fi
 if [[ "$(docker inspect --format "{{.State.Running}}" "$CONTAINER")" == "false" ]]; then break; fi
 sleep .1
done
[[ "$ready" == "1" ]] || { echo "HA Core IPC fixture could not start" >&2; exit 3; }
RESULT="$(target/debug/hars-python-bridge "$DIR/mock.sock" "$DIR/capability")"
[[ "$RESULT" == *"states=1232"* && "$RESULT" == *"generation=1"* ]] || exit 4
echo "PASS: real HA Core 2026.10.1 StateMachine created 1232 synthetic states via UDS to Rust"
EXITCODE="$(docker wait "$CONTAINER")"
[[ "$EXITCODE" == "0" ]] || exit 5
echo "PASS: no network, root, radio, production config or persistent sidecar"
