#!/usr/bin/env bash
# Genuine public Adaptive Lighting v1.32.0 on pinned official Core.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
[[ "$(id -u)" != "0" && ( -n "$(printenv CODESPACES || true)" || -n "$(printenv GITHUB_ACTIONS || true)" ) ]] || {
  echo "GitHub Codespaces/Actions unprivileged lab only" >&2; exit 2
}
REPO="$ROOT/references/adaptive-lighting"
SOURCE_SHA=7f7fec4e34d331fa5fd84b95514ddb1baf2a401e
LICENSE_SHA=c71d239df91726fc519c6eb72d318ec65820627232b2f796219e87dcf35d0ab4
mkdir -p "$ROOT/references" "$ROOT/runtime"
if [[ ! -d "$REPO/.git" ]]; then
  git clone --quiet --depth 1 --branch v1.32.0 https://github.com/basnijholt/adaptive-lighting.git "$REPO"
fi
[[ "$(git -C "$REPO" rev-parse HEAD)" == "$SOURCE_SHA" ]] || {
  echo "upstream Adaptive Lighting source SHA mismatch" >&2;exit 3
}
echo "$LICENSE_SHA  $REPO/LICENSE" | sha256sum -c - >/dev/null
python3 - "$REPO/custom_components/adaptive_lighting/manifest.json" <<'PY'
import json,sys
m=json.load(open(sys.argv[1]))
assert m["domain"]=="adaptive_lighting" and m["version"]=="1.32.0"
assert m["config_flow"] is True and m["requirements"]==["ulid-transform"]
assert m["dependencies"]==["light"]
PY
TMP="$(mktemp -d "$ROOT/runtime/hars-adaptive-XXXXXXXX")"
chmod 700 "$TMP"
cleanup() { rm -rf -- "$TMP"; }
trap cleanup EXIT
mkdir -p "$TMP/config/custom_components"
cp -a "$REPO/custom_components/adaptive_lighting" "$TMP/config/custom_components/adaptive_lighting"
cat > "$TMP/config/configuration.yaml" <<'YAML'
homeassistant:
  name: Rusty Home Adaptive Lighting synthetic lab
  time_zone: Europe/Paris
YAML
IMAGE='ghcr.io/home-assistant/home-assistant@sha256:33afb0659d786213264c2211f8d91c5fe87461c56eba6a3a546eb05fc71e89f4'
if ! docker image inspect "$IMAGE" >/dev/null 2>&1; then docker pull "$IMAGE" >/dev/null; fi
docker run --rm --network none --user "$(id -u):$(id -g)" --read-only \
  --cap-drop ALL --security-opt no-new-privileges --pids-limit 256 \
  --memory 2000m --cpus 2 --tmpfs /tmp:rw,nosuid,nodev,size=128m \
  --tmpfs /run:rw,nosuid,nodev,size=64m \
  --env PYTHONDONTWRITEBYTECODE=1 --env TZ=Europe/Paris \
  --mount "type=bind,src=$TMP/config,dst=/config" \
  --mount "type=bind,src=$ROOT/scripts/test_p6f_real_adaptive_core.py,dst=/adaptive-proof.py,readonly" \
  --entrypoint python3 "$IMAGE" /adaptive-proof.py >"$TMP/container.log" 2>&1 || {
    echo "Official Core actual Adaptive Lighting lifecycle failed" >&2
    tail -30 "$TMP/container.log" >&2
    if [[ -f "$TMP/config/adaptive-error.txt" ]]; then tail -75 "$TMP/config/adaptive-error.txt" >&2; fi
    exit 1
  }
test -s "$TMP/config/adaptive-proof.json" || { echo "Real Core adaptive proof missing" >&2; exit 1; }
python3 scripts/check_p6i_adaptive_core_report.py "$TMP/config/adaptive-proof.json"
echo P6F_REAL_ADAPTIVE_LIGHTING_V1_32_0_CORE_LIFECYCLE_PASS
