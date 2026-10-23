#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
BIN="$ROOT/target/debug/ha-rust-bridge"
if [[ ! -x "$BIN" ]]; then cargo build --locked -p ha-rust-bridge; fi
"$BIN" --fixture fixtures/docker-ps.jsonl --host lab-node --haos-snapshot audits/haos-apps.json --host lab-guest --serve 127.0.0.1:18790 > /dev/null 2>&1 &
pid=$!
trap 'kill "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true' EXIT
for _ in {1..30}; do
 if curl -fsS --max-time 1 http://127.0.0.1:18790/health >/dev/null 2>&1; then break; fi
 sleep .1
done
curl -fsS --max-time 3 http://127.0.0.1:18790/health | python3 -c 'import json,sys; x=json.load(sys.stdin); assert x["status"]=="ok" and x["read_only"]'
curl -fsS --max-time 3 http://127.0.0.1:18790/api/apps | python3 -c 'import json,sys; x=json.load(sys.stdin); assert len(x["apps"])==13 and len(x["sources"])==2 and x["read_only"]; print("PASS: read-only multi-host /api/apps")'
curl -fsS --max-time 3 http://127.0.0.1:18790/api/sources | python3 -c 'import json,sys; x=json.load(sys.stdin); assert len(x["sources"])==2 and x["sources"][1]["live"]==False; print("PASS: /api/sources identifies offline snapshot")'
curl -fsS --max-time 3 http://127.0.0.1:18790/api/readiness | python3 -c 'import json,sys; r=json.load(sys.stdin); assert r["read_only"] and not r["production_ready"] and not r["deployment_authorized"]; assert r["release_gates_total"]==15 and r["release_gates_blocked"]==15 and "haos_apps_total" not in r and "haos_apps_not_production_qualified" not in r; print("PASS: /api/readiness blocks production with release gates and no fixed App count")'
code="$(curl -s -o /dev/null -w '%{http_code}' --max-time 3 http://127.0.0.1:18790/api/python-core)"
[[ "$code" == 404 ]] || { echo "FAIL: missing Python sidecar should return HTTP 404"; exit 1; }
echo "PASS: no fake Python Core connection reported without configuration"
curl -fsS --max-time 3 http://127.0.0.1:18790/ | python3 -c 'import sys; html=sys.stdin.read(); assert "<title>Rusty Home" in html and "<h1>Rusty Home</h1>" in html and "Domotique native Rust" in html and "<html" in html.lower()' # consume full response, avoid grep -q/Curl SIGPIPE
echo "PASS: localized HTML dashboard"
code="$(curl -s -o /dev/null -w '%{http_code}' -X POST --max-time 3 http://127.0.0.1:18790/api/apps)"
[[ "$code" == 404 ]] || { echo "FAIL: unexpected POST route: $code"; exit 1; }
echo "PASS: no mutation route"
echo "PASS: loopback-only smoke; process terminated by trap"
