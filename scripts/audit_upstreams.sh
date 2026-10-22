#!/usr/bin/env bash
set -uo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
mkdir -p "$ROOT/references" "$ROOT/audits"
out="$ROOT/audits/upstreams.tsv"
printf 'name\turl\tcommit\tcommit_date\tsparse\tlicense_file\tstatus\n' > "$out"
clone_one() {
  local name="$1" url="$2" sparse="$3"
  local p="$ROOT/references/$name" stat="ok"
  if [[ ! -d "$p/.git" ]]; then
    if ! timeout 125 git clone -q --depth 1 --filter=blob:none --sparse "$url" "$p"; then
      stat="clone_failed"; rm -rf "$p"
    fi
  fi
  if [[ "$stat" == ok && -n "$sparse" ]]; then
    # reference snapshots: no build trees and no giant multimedia datasets
    if ! git -C "$p" sparse-checkout set $sparse >/dev/null 2>&1; then
      stat="sparse_failed"
    fi
  elif [[ "$stat" == ok ]]; then
    git -C "$p" sparse-checkout disable >/dev/null 2>&1 || true
  fi
  local hash="n/a" date="n/a" lic="n/a"
  if [[ "$stat" != clone_failed ]]; then
    hash="$(git -C "$p" rev-parse HEAD 2>/dev/null || echo n/a)"
    date="$(git -C "$p" log -1 --format=%cs 2>/dev/null || echo n/a)"
    lic="$(find "$p" -maxdepth 1 -iname 'LICENSE*' -type f -printf '%f,' 2>/dev/null | sed 's/,$//')"
  fi
  printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$name" "$url" "$hash" "$date" "$sparse" "$lic" "$stat" >> "$out"
  printf '%-20s %s %s\n' "$name" "$stat" "$hash"
}
clone_one "moli-os" "https://github.com/rudy-io/moli-os.git" ""
clone_one "homeCore-io" "https://github.com/homeCore-io/homeCore.git" ""
clone_one "RuView-HOMECORE" "https://github.com/ruvnet/RuView.git" "docs/adr v2/crates/homecore v2/crates/homecore-api v2/crates/homecore-plugins v2/crates/homecore-automation v2/crates/homecore-migrate v2/crates/homecore-recorder v2/crates/homecore-assist"
clone_one "home-assistant-core" "https://github.com/home-assistant/core.git" "homeassistant/components/hassio homeassistant/components/websocket_api homeassistant/components/api"
clone_one "home-assistant-supervisor" "https://github.com/home-assistant/supervisor.git" "supervisor/api supervisor/addons supervisor/docker supervisor/backups supervisor/homeassistant docs"
clone_one "hacs-integration" "https://github.com/hacs/integration.git" ""
clone_one "home-assistant-apps" "https://github.com/home-assistant/addons.git" "matter_server ssh configurator"
printf '\nMANIFEST: %s\n' "$out"
