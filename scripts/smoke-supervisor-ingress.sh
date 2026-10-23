#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
cargo build --locked -q -p ha-rust-bridge
PORT=18794
SUPERVISOR_TOKEN="test_only_credential" "$ROOT/target/debug/ha-rust-bridge" \
  --supervisor-live --host any-vendor --ingress --serve "0.0.0.0:$PORT" >/dev/null 2>&1 &
pid=$!
trap 'kill "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true' EXIT
for _ in {1..30}; do
  if curl --silent --output /dev/null --max-time 1 "http://127.0.0.1:$PORT/health"; then break; fi
  sleep .1
done
for route in / /health /api/apps /api/session /api/session/core /api/store/apps/core_ssh/operations /api/readiness; do
  result="$(curl --silent --output /dev/null --write-out '%{http_code}' \
    -H "X-Remote-User-Id: system-admin" -H "Host: ha.example.test" \
    --max-time 4 "http://127.0.0.1:$PORT$route")"
  [[ "$result" == "403" ]] || { echo "FAIL: untrusted TCP source accepted $route status=$result"; exit 1; }
done
result="$(curl --silent --output /dev/null --write-out '%{http_code}' \
    -X POST -H "X-Remote-User-Id: system-admin" --max-time 3 "http://127.0.0.1:$PORT/api/apps")"
[[ "$result" == "403" ]] || { echo "FAIL: untrusted mutation route accepted $result"; exit 1; }
echo "PASS: Supervisor ingress rejects forged admin headers from untrusted loopback TCP"
