#!/usr/bin/env bash
# Real Smart Announcements 0.8.3, exact MIT upstream, pinned Core, no network.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
if [[ "$(id -u)" == "0" || ( "${CODESPACES:-}" != "true" && "${GITHUB_ACTIONS:-}" != "true" ) ]]; then
  echo "Run only in an unprivileged GitHub Codespace or Actions" >&2; exit 2
fi
UPSTREAM=e5a843be497e5591233896035a4a20fd60e939bb
LICENSE_SHA=fe01560703189f7d340f91b594ac9555805a3be41b0a51b97e77a05bc69aa24d
SOURCE="$ROOT/references/smart-announcements"
mkdir -p references runtime
if [[ ! -d "$SOURCE/.git" ]]; then
  git clone --quiet --depth 1 --branch v0.8.3 https://github.com/mike-nott/smart-announcements.git "$SOURCE"
fi
test "$(git -C "$SOURCE" rev-parse HEAD)" = "$UPSTREAM" || { echo "Upstream tag/commit mismatch" >&2; exit 3; }
echo "$LICENSE_SHA  $SOURCE/LICENSE" | sha256sum -c - >/dev/null
python3 - "$SOURCE/custom_components/smart_announcements/manifest.json" <<'PY'
import json,sys
manifest=json.load(open(sys.argv[1]))
assert manifest["domain"]=="smart_announcements"
assert manifest["version"]=="0.8.3" and manifest["requirements"]==[]
assert manifest["config_flow"] is True
PY
TMP="$(mktemp -d "$ROOT/runtime/hars-smart-XXXXXXXX")"
chmod 700 "$TMP"
NET="hars-smart-internal-$(basename "$TMP" | tr -cd 'a-zA-Z0-9-')"
NET_CREATED=0
cleanup() {
  if [[ "$NET_CREATED" == "1" ]]; then docker network rm "$NET" >/dev/null 2>&1 || true; fi
  rm -rf -- "$TMP"
}
trap cleanup EXIT
docker network create --internal --driver bridge "$NET" >/dev/null
NET_CREATED=1
test "$(docker network inspect -f '{{.Internal}}' "$NET")" = "true"
mkdir -p "$TMP/config/custom_components"
chmod 700 "$TMP/config" "$TMP/config/custom_components"
cp -a "$SOURCE/custom_components/smart_announcements" "$TMP/config/custom_components/"
cat > "$TMP/config/configuration.yaml" <<'YAML'
homeassistant:
  name: Fictional Smart Announcements Core lab
  time_zone: Europe/Paris
YAML
IMAGE='ghcr.io/home-assistant/home-assistant@sha256:130241f28d01fa80dfa3b44f7d89781c188d8642ecab9024f079f19f9f82baa0'
if ! docker image inspect "$IMAGE" >/dev/null 2>&1; then docker pull "$IMAGE" >/dev/null; fi
docker run --rm --network "$NET" --user "$(id -u):$(id -g)" --read-only \
  --cap-drop ALL --security-opt no-new-privileges --pids-limit 256 \
  --memory 2200m --cpus 2 --tmpfs /tmp:rw,nosuid,nodev,size=128m \
  --tmpfs /run:rw,nosuid,nodev,size=64m \
  --env PYTHONDONTWRITEBYTECODE=1 --env TZ=Europe/Paris --env P6F_INTERNAL_BRIDGE=verified \
  --mount "type=bind,src=$TMP/config,dst=/config" \
  --mount "type=bind,src=$ROOT/scripts/test_p6f_real_smart_core.py,dst=/p6f-test.py,readonly" \
  --entrypoint python3 "$IMAGE" /p6f-test.py >"$TMP/run.log" 2>&1 || {
    echo "Real Smart Announcements Core test failed" >&2
    tail -60 "$TMP/run.log" >&2
    if test -f "$TMP/config/smart-traceback.txt"; then tail -75 "$TMP/config/smart-traceback.txt" >&2; fi
    exit 1
  }
test -s "$TMP/config/smart-proof.json" || { echo 'Missing native Core proof' >&2; exit 1; }
python3 scripts/check_p6f_real_smart_report.py "$TMP/config/smart-proof.json"
echo P6F_REAL_SMART_ANNOUNCEMENTS_PINNED_CORE_PASS
