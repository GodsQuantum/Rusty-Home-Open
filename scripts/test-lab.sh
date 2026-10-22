#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
cargo fmt --all -- --check
cargo test --locked --all
# Multi-source inventory is synthetic Docker + sanitized Supervisor snapshot.
cargo run --quiet --locked -p ha-rust-bridge -- --fixture fixtures/docker-ps.jsonl --host lab-node --haos-snapshot audits/haos-apps.json --host lab-guest |
  python3 -c 'import json,sys; x=json.load(sys.stdin); assert x["read_only"] and not x["degraded"]; assert len(x["apps"])==13; assert sorted((a["host"],a["backend"]) for a in x["apps"]).count(("lab-guest","supervisor"))==9; print("PASS: namespaced inventory with 4 Docker fixture + 9 HAOS snapshot Apps")'
# A missing host must not mask nine healthy Apps; the process exits non-zero.
set +e
out="$(cargo run --quiet --locked -p ha-rust-bridge -- --haos-snapshot audits/haos-apps.json --host lab-guest --haos-snapshot ./fixtures/missing.json --host missing 2>/dev/null)"
rc=$?
set -e
[[ "$rc" -ne 0 ]] || { echo "FAIL: degraded CLI incorrectly reports success"; exit 1; }
printf '%s' "$out" |
  python3 -c 'import json,sys; x=json.load(sys.stdin); assert x["degraded"] and len(x["apps"])==9; assert x["sources"][1]["status"]=="unavailable"; print("PASS: one source failure isolated")'
if cargo run --quiet --locked -p ha-rust-bridge -- --fixture fixtures/docker-ps.jsonl --serve 0.0.0.0:18790 >/dev/null 2>&1; then
 echo "FAIL: public bind should be prohibited"; exit 1
fi
echo "PASS: network binding restricted to loopback"
echo "PASS: lab suite without production writes"
