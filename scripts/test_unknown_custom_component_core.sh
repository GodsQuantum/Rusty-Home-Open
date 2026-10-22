#!/usr/bin/env bash
# Local optional P6a validation in sample lab host isolated Linux lab only (never CI network dependency).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
[[ "$(id -u)" != "0" && ( "$(hostname)" == "dev-build" || "${GITHUB_ACTIONS:-}" == "true" ) ]] || { echo 'isolated Linux lab unprivileged or isolated GitHub CI only'; exit 2; }
IMAGE='ghcr.io/home-assistant/home-assistant@sha256:130241f28d01fa80dfa3b44f7d89781c188d8642ecab9024f079f19f9f82baa0'
if ! docker image inspect "$IMAGE" >/dev/null 2>&1; then
  if [[ "${GITHUB_ACTIONS:-}" == "true" ]]; then
    docker pull "$IMAGE" >/dev/null
  else
    echo 'PINNED CORE IMAGE MISSING — DO NOT REPLACE IMAGE' >&2
    exit 4
  fi
fi
mkdir -p runtime
TMP="$(mktemp -d "$ROOT/runtime/hars-unknown-core-XXXXXX")"
chmod 700 "$TMP"
cleanup() { rm -rf -- "$TMP"; }
trap cleanup EXIT
mkdir -p "$TMP/config/custom_components/hars_future_probe"
chmod 700 "$TMP/config" "$TMP/config/custom_components" "$TMP/config/custom_components/hars_future_probe"
cat > "$TMP/config/configuration.yaml" <<'YAML'
homeassistant:
  name: Synthetic no-network test
  time_zone: Europe/Paris
YAML
cat > "$TMP/config/custom_components/hars_future_probe/manifest.json" <<'JSON'
{
  "domain": "hars_future_probe",
  "name": "Fictional never-before-seen custom integration",
  "version": "0.0.1",
  "documentation": "https://example.invalid/future_probe",
  "issue_tracker": "https://example.invalid/issues",
  "codeowners": [],
  "requirements": [],
  "dependencies": [],
  "iot_class": "local_push"
}
JSON
cat > "$TMP/config/custom_components/hars_future_probe/__init__.py" <<'PY'
"""A fictitious unknown integration running inside the genuine Core runtime only."""
from homeassistant.core import HomeAssistant
async def async_setup(hass: HomeAssistant, config: dict) -> bool:
    hass.states.async_set("sensor.hars_future_probe","4242",{"hars_synthetic":True})
    async def hello(_call):
        return None
    hass.services.async_register("hars_future_probe","hello",hello)
    return True
PY
chmod 600 "$TMP/config/configuration.yaml" "$TMP/config/custom_components/hars_future_probe/"{manifest.json,__init__.py}
docker run --rm --network none --user "$(id -u):$(id -g)" --read-only \
 --cap-drop ALL --security-opt no-new-privileges --pids-limit 192 \
 --memory 1500m --cpus 2 \
 --tmpfs /tmp:rw,nosuid,nodev,size=96m \
 --tmpfs /run:rw,nosuid,nodev,size=48m \
 --env PYTHONDONTWRITEBYTECODE=1 --env TZ=Europe/Paris \
 --mount "type=bind,src=$TMP/config,dst=/config" \
 --mount "type=bind,src=$ROOT/scripts/test_unknown_custom_component_core.py,dst=/hars-test.py,readonly" \
 --entrypoint python3 "$IMAGE" /hars-test.py
echo 'P6a CORE UNKNOWN INTEGRATION LAB: completed; disposable fixture removed by trap'
