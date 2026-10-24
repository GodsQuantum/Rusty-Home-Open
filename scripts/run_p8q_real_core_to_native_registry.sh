#!/usr/bin/env bash
# Genuine official Core 2026.10.2 registry -> private Rust read-only stage.
# Disposable GitHub Actions CI only, isolated Docker network none, no user data.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
if [[ "$(id -u)" == "0" || "$GITHUB_ACTIONS" != "true" ]]; then
  echo "Only GitHub Actions non-root" >&2; exit 2
fi
IMAGE="ghcr.io/home-assistant/home-assistant@sha256:33afb0659d786213264c2211f8d91c5fe87461c56eba6a3a546eb05fc71e89f4"
if ! docker image inspect "$IMAGE" >/dev/null 2>&1; then docker pull "$IMAGE" >/dev/null; fi
mkdir -p runtime
TMP="$(mktemp -d "$ROOT/runtime/hars-real-core-stage-XXXXXXXX")"
chmod 700 "$TMP"
trap 'rm -rf -- "$TMP"' EXIT
mkdir -p "$TMP/config/custom_components/hars_lifecycle_probe" "$TMP/rust-private"
chmod 700 "$TMP/config" "$TMP/config/custom_components" "$TMP/config/custom_components/hars_lifecycle_probe" "$TMP/rust-private"
cp scripts/fixtures/ha-config-lifecycle/manifest.json scripts/fixtures/ha-config-lifecycle/__init__.py scripts/fixtures/ha-config-lifecycle/config_flow.py "$TMP/config/custom_components/hars_lifecycle_probe/"
cat > "$TMP/config/configuration.yaml" <<'YAML'
homeassistant:
  name: Synthetic Core registry-to-Rust staging
  time_zone: Europe/Paris
YAML
docker run --rm --network none --user "$(id -u):$(id -g)" --read-only \
  --cap-drop ALL --security-opt no-new-privileges --pids-limit 192 \
  --memory 1800m --cpus 2 \
  --tmpfs /tmp:rw,nosuid,nodev,size=96m \
  --tmpfs /run:rw,nosuid,nodev,size=48m \
  --env PYTHONDONTWRITEBYTECODE=1 --env TZ=Europe/Paris \
  --mount "type=bind,src=$TMP/config,dst=/config" \
  --mount "type=bind,src=$ROOT/scripts/test_p8q_real_core_registry_export.py,dst=/hars-core-registry-export.py,readonly" \
  --entrypoint python3 "$IMAGE" /hars-core-registry-export.py >"$TMP/core.log" 2>&1 || {
    tail -60 "$TMP/core.log" >&2
    echo P8Q_REAL_CORE_EXPORT_FAILED >&2
    exit 1
  }
grep -q P8Q_REAL_CORE_REGISTRY_CREATED_AND_SAVED_PASS "$TMP/core.log"
SOURCE="$TMP/config/.storage/core.entity_registry"
test -s "$SOURCE"
cargo build --locked --quiet -p hars-interop-moli --bin ha_registry_stager
RUST="$ROOT/target/debug/ha_registry_stager"
STAGED="$("$RUST" --lab-readonly "$TMP/rust-private" stage "$SOURCE")"
echo "$STAGED" | python3 -c 'import sys,json;a=json.load(sys.stdin);assert a["entities"]>=2 and a["stable_identities"]>=2 and not a["production_ready"]'
SHA="$(echo "$STAGED" | python3 -c 'import json,sys;print(json.load(sys.stdin)["sha256"])')"
"$RUST" --lab-readonly "$TMP/rust-private" activate "$SHA" none >/dev/null
"$RUST" --lab-readonly "$TMP/rust-private" status | python3 -c 'import json,sys;a=json.load(sys.stdin);assert a["active"] and a["entities"]>=2 and not a["state_known"]'
"$RUST" --lab-readonly "$TMP/rust-private" rollback "$SHA" | python3 -c 'import json,sys;assert json.load(sys.stdin)["restored"] is None'
"$RUST" --lab-readonly "$TMP/rust-private" status | python3 -c 'import json,sys;assert json.load(sys.stdin)["active"] is None'
echo P8Q_REAL_CORE_ENTITY_REGISTRY_TO_RUST_PERSIST_ROLLBACK_PASS
