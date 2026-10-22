#!/usr/bin/env bash
# True HA Core StateMachine -> Python UDS -> Rust HTTP API integration, all synthetic.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
[[ "$(id -u)" != 0 && ( "$(hostname)" == "dev-build" || "${GITHUB_ACTIONS:-}" == "true" ) ]] || exit 2
cd "$ROOT"
IMAGE="ghcr.io/home-assistant/home-assistant@sha256:130241f28d01fa80dfa3b44f7d89781c188d8642ecab9024f079f19f9f82baa0"
cargo build --quiet --locked -p ha-rust-bridge
mkdir -p runtime
DIR="$(mktemp -d "$ROOT/runtime/ha-core-ui-XXXXXX")"
chmod 700 "$DIR"
CONTAINER="hars-core-ui-$$"
UI_PID=""
cleanup() {
  if [[ -n "$UI_PID" ]]; then kill "$UI_PID" 2>/dev/null || true; wait "$UI_PID" 2>/dev/null || true; fi
  docker rm -f "$CONTAINER" >/dev/null 2>&1 || true
  rm -rf -- "$DIR"
}
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
  if [[ "$(docker inspect --format "{{.State.Running}}" "$CONTAINER")" == false ]]; then break; fi
  sleep .1
done
[[ "$ready" == 1 ]] || { echo "Official Core UDS failed to start" >&2; exit 3; }
PORT="$(python3 -c 'import socket;s=socket.socket();s.bind(("127.0.0.1",0));print(s.getsockname()[1]);s.close()')"
target/debug/ha-rust-bridge --fixture fixtures/docker-ps.jsonl --host lab-node \
 --python-socket "$DIR/mock.sock" --python-capability "$DIR/capability" \
 --serve "127.0.0.1:$PORT" > "$DIR/ui.out" 2> "$DIR/ui.err" &
UI_PID=$!
healthy=0
for _ in $(seq 1 80); do
  if curl -fsS --max-time 1 -o /dev/null "http://127.0.0.1:$PORT/health"; then healthy=1; break; fi
  if ! kill -0 "$UI_PID" 2>/dev/null; then break; fi
  sleep .05
done
[[ "$healthy" == 1 ]] || { echo "Rust UI failed to start" >&2; exit 4; }
curl -fsS --max-time 6 "http://127.0.0.1:$PORT/api/python-core" | python3 -c 'import json,sys;x=json.load(sys.stdin); assert x["entities_count"]==1232 and x["generation"]==1; assert x["read_only"] and not x["production_ready"] and x["contains_entity_values"] is False; print("PASS: authentic Core StateMachine -> UDS -> Rust HTTP with 1232 simulated states")'
EXITCODE="$(docker wait "$CONTAINER")"
[[ "$EXITCODE" == 0 ]] || { echo "Official Core server exited abnormally" >&2; exit 5; }
STATUS="$(curl -sS -o "$DIR/stale.json" -w "%{http_code}" --max-time 5 "http://127.0.0.1:$PORT/api/python-core")"
[[ "$STATUS" == 503 ]] || { echo "FAIL: stale Core connection must return 503, got $STATUS" >&2; exit 6; }
python3 -c 'import json,sys; x=json.load(open(sys.argv[1])); assert x["status"]=="unavailable" and x["read_only"]' "$DIR/stale.json"
echo "PASS: second read after sidecar shutdown fails closed with HTTP 503"
curl -fsS --max-time 3 "http://127.0.0.1:$PORT/api/apps" | python3 -c 'import json,sys;x=json.load(sys.stdin);assert x["read_only"] and len(x["apps"])==4;print("PASS: isolated Docker Apps fixture remains available without Python sidecar")'
echo "PASS: no external network, real credentials, HAOS devices, or persistent test services"
