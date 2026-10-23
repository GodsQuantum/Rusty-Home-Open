#!/usr/bin/env bash
# Genuine Home Assistant Core 2026.10.2 synthetic config-entry lifecycle proof.
# Codespaces / GitHub Actions only; no production hosts, files or devices.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
[[ "$(id -u)" != "0" && ( "${GITHUB_ACTIONS:-}" == "true" || "${CODESPACES:-}" == "true" ) ]] || {
  echo "Only unprivileged GitHub Codespaces / isolated GitHub Actions" >&2
  exit 2
}
IMAGE='ghcr.io/home-assistant/home-assistant@sha256:33afb0659d786213264c2211f8d91c5fe87461c56eba6a3a546eb05fc71e89f4'
if ! docker image inspect "$IMAGE" >/dev/null 2>&1; then
    docker pull "$IMAGE" >/dev/null
fi
mkdir -p runtime
TMP="$(mktemp -d "$ROOT/runtime/hars-config-lifecycle-XXXXXXXX")"
chmod 700 "$TMP"
cleanup() { rm -rf -- "$TMP"; }
trap cleanup EXIT
TARGET="$TMP/config/custom_components/hars_lifecycle_probe"
mkdir -p "$TARGET"
chmod 700 "$TMP/config" "$TMP/config/custom_components" "$TARGET"
cp scripts/fixtures/ha-config-lifecycle/{manifest.json,__init__.py,config_flow.py} "$TARGET/"
cat > "$TMP/config/configuration.yaml" <<'YAML'
homeassistant:
  name: Rusty Home synthetic lifecycle
  time_zone: Europe/Paris
YAML
chmod 600 "$TMP/config/configuration.yaml" "$TARGET/"{manifest.json,__init__.py,config_flow.py}
docker run --rm --network none --user "$(id -u):$(id -g)" --read-only \
  --cap-drop ALL --security-opt no-new-privileges --pids-limit 192 \
  --memory 1800m --cpus 2 \
  --tmpfs /tmp:rw,nosuid,nodev,size=96m \
  --tmpfs /run:rw,nosuid,nodev,size=48m \
  --env PYTHONDONTWRITEBYTECODE=1 --env TZ=Europe/Paris \
  --mount "type=bind,src=$TMP/config,dst=/config" \
  --mount "type=bind,src=$ROOT/scripts/test_config_entry_lifecycle_core.py,dst=/hars-lifecycle.py,readonly" \
  --entrypoint python3 "$IMAGE" /hars-lifecycle.py >"$TMP/docker-output.log" 2>&1 || {
    echo 'Official pinned HA Core lifecycle FAILED in disposable Codespace' >&2
    tail -75 "$TMP/docker-output.log" >&2
    if test -f "$TMP/config/p6c_launch"; then echo 'CORE_PYTHON_LAUNCHED' >&2; fi
    if test -f "$TMP/config/p6c_debug.log"; then
      tail -75 "$TMP/config/p6c_debug.log" >&2
    fi
    exit 1
  }
python3 scripts/check_config_entry_lifecycle_report.py "$TMP/config/p6c-proof.json"
cat "$TMP/docker-output.log"
echo 'P6C_GENUINE_CORE_CONFIG_ENTRY_LIFECYCLE_PASS'
