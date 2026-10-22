#!/usr/bin/env bash
# Official SecureTar 2026.10 v3 fixture, executed ONLY in temporary venv.
# No real backup, keys, secrets or Home Assistant devices. Refuses root.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
[[ "$(id -u)" != 0 ]] || { echo "Run unprivileged" >&2; exit 2; }
mkdir -p "$ROOT/runtime"
TMP="$(mktemp -d "$ROOT/runtime/securetar-crypto-XXXXXX")"
cleanup() { rm -rf -- "$TMP"; }
trap cleanup EXIT
python3 -m venv "$TMP/venv"
"$TMP/venv/bin/python" -m pip install --disable-pip-version-check --only-binary=:all:   -r "$ROOT/scripts/requirements-securetar-test.txt" > "$TMP/pip-install.log" 2>&1 || {
    echo "SecureTar pinned test dependencies failed (see ignored ephemeral logs)" >&2
    exit 3
  }
"$TMP/venv/bin/python" "$ROOT/scripts/test_securetar_v3_synthetic.py"
echo "PASS: temporary SecureTar venv will be removed"
