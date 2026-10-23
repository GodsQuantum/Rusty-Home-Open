#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
cargo build --locked -q -p ha-rust-bridge
BIN="$ROOT/target/debug/ha-rust-bridge"
PORT=18793
env -u SUPERVISOR_TOKEN "$BIN" --supervisor-live --host future-third-party --serve "127.0.0.1:$PORT" >/dev/null 2>&1 &
pid=$!
trap 'kill "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true' EXIT
for _ in {1..30}; do
  if curl -fsS --max-time 1 "http://127.0.0.1:$PORT/health" >/dev/null 2>&1; then break; fi
  sleep .1
done
curl -fsS --max-time 3 "http://127.0.0.1:$PORT/api/apps" | python3 -c '
import json,sys
x=json.load(sys.stdin)
assert x["read_only"] is True
assert x["degraded"] is True
assert x["apps"]==[]
assert len(x["sources"])==1
s=x["sources"][0]
assert s["host"]=="future-third-party"
assert s["kind"]=="supervisor-live"
assert s["status"]=="unavailable"
assert s["live"] is False
assert "token" not in json.dumps(x).lower()
print("PASS: official live adapter fails closed without Supervisor token")
'
status="$(curl --silent --output /dev/null --write-out '%{http_code}' -X POST --max-time 3 "http://127.0.0.1:$PORT/api/apps")"
[[ "$status" == "404" ]] || { echo "FAIL: mutation request unexpectedly accepted $status"; exit 1; }
echo "PASS: Supervisor live dashboard is GET-only, never permits mutation"
