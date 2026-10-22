#!/usr/bin/env bash
# Explicit, opt-in network research. Public hacs/default lists only.
# No packages are installed, code executed or user credentials consulted.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
[[ "$(id -u)" != 0 ]] || { echo 'Run unprivileged' >&2; exit 2; }
cd "$ROOT"
mkdir -p runtime
TMP="$(mktemp -d "$ROOT/runtime/hacs-catalog-audit-XXXXXX")"
cleanup() {
  case "$TMP" in "$ROOT"/runtime/hacs-catalog-audit-*) rm -rf -- "$TMP" ;; esac
}
trap cleanup EXIT
for kind in integration plugin theme template; do
  curl -fsSL --max-time 18 --retry 1 --proto '=https' \
    "https://raw.githubusercontent.com/hacs/default/master/$kind" -o "$TMP/$kind.json"
  echo "== $kind :: sha256 $(sha256sum "$TMP/$kind.json" | awk '{print $1}') =="
  cargo run --quiet --locked -p hars-community-packages --example check_catalog -- "$TMP/$kind.json"
done
echo "PASS: four official public HACS/Marketplace catalog categories structurally audited, zero installs"
