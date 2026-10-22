#!/usr/bin/env bash
# Exact official HACS Card Builder 2.6.0 release, isolated genuine HA Core.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
[[ "$(id -u)" != "0" && ( -n "$(printenv CODESPACES || true)" || -n "$(printenv GITHUB_ACTIONS || true)" ) ]] || {
  echo "Unprivileged GitHub Codespaces/Actions lab only" >&2; exit 2
}
mkdir -p references runtime
REPO="$ROOT/references/card-builder"
SOURCE_SHA=2ae8e5152a5f6af46307130a045133f900f4397e
LICENSE_SHA=8486a10c4393cee1c25392769ddd3b2d6c242d6ec7928e1414efff7dfb2f07ef
ASSET_SHA=75ca0cc33397b16f2d2af4c2aad40391a84f013786c264a22b7186fe309fb7a7
if [[ ! -d "$REPO/.git" ]]; then
  git clone --quiet --depth 1 --branch v2.6.0 https://github.com/studiobts/home-assistant-card-builder.git "$REPO"
fi
[[ "$(git -C "$REPO" rev-parse HEAD)" == "$SOURCE_SHA" ]] || { echo "Card Builder source SHA mismatch" >&2; exit 3; }
echo "$LICENSE_SHA  $REPO/LICENSE" | sha256sum -c - >/dev/null
ASSET="$ROOT/runtime/card_builder.zip"
if [[ ! -f "$ASSET" ]]; then
  python3 - "$ASSET" <<'PY'
import hashlib,sys,urllib.request
url="https://github.com/studiobts/home-assistant-card-builder/releases/download/v2.6.0/card_builder.zip"
blob=urllib.request.urlopen(url,timeout=45).read()
assert len(blob)<5000000
assert hashlib.sha256(blob).hexdigest()=="75ca0cc33397b16f2d2af4c2aad40391a84f013786c264a22b7186fe309fb7a7"
open(sys.argv[1],"wb").write(blob)
PY
fi
echo "$ASSET_SHA  $ASSET" | sha256sum -c - >/dev/null
TMP="$(mktemp -d "$ROOT/runtime/hars-cardbuilder-XXXXXXXX")"
chmod 700 "$TMP"
NET="hars-cb-offline-$(basename "$TMP" | tr -cd 'a-zA-Z0-9-')"
NET_CREATED=0
cleanup() {
  if [[ "$NET_CREATED" == "1" ]]; then docker network rm "$NET" >/dev/null 2>&1 || true; fi
  rm -rf -- "$TMP"
}
trap cleanup EXIT
docker network create --internal --driver bridge "$NET" >/dev/null
NET_CREATED=1
test "$(docker network inspect -f '{{.Internal}}' "$NET")" = "true"
TARGET="$TMP/config/custom_components/card_builder"
mkdir -p "$TARGET"
python3 - "$ASSET" "$TARGET" <<'PY'
import json,stat,sys
from pathlib import PurePosixPath
from zipfile import ZipFile
with ZipFile(sys.argv[1]) as z:
  rows=z.infolist()
  assert len(rows)<500 and sum(x.file_size for x in rows)<10000000
  for info in rows:
    name=PurePosixPath(info.filename)
    assert not name.is_absolute() and ".." not in name.parts
    assert not stat.S_ISLNK(info.external_attr>>16)
  assert "frontend/dist/card-builder-panel.js" in z.namelist()
  z.extractall(sys.argv[2])
m=json.load(open(sys.argv[2]+"/manifest.json"))
assert m["domain"]=="card_builder" and m["version"]=="2.6.0"
assert m["config_flow"] is True and m["requirements"]==[] and m["dependencies"]==["http"]
PY
cat > "$TMP/config/configuration.yaml" <<'YAML'
homeassistant:
  name: Rusty Home Card Builder synthetic release test
  time_zone: Europe/Paris
http:
  server_host: 127.0.0.1
YAML
IMAGE='ghcr.io/home-assistant/home-assistant@sha256:130241f28d01fa80dfa3b44f7d89781c188d8642ecab9024f079f19f9f82baa0'
if ! docker image inspect "$IMAGE" >/dev/null 2>&1; then docker pull "$IMAGE" >/dev/null; fi
docker run --rm --network "$NET" --user "$(id -u):$(id -g)" --read-only \
 --cap-drop ALL --security-opt no-new-privileges --pids-limit 256 \
 --memory 2400m --cpus 2 --tmpfs /tmp:rw,nosuid,nodev,size=128m \
 --tmpfs /run:rw,nosuid,nodev,size=64m \
 --env PYTHONDONTWRITEBYTECODE=1 --env TZ=Europe/Paris --env P6G_DOCKER_INTERNAL=verified \
 --mount "type=bind,src=$TMP/config,dst=/config" \
 --mount "type=bind,src=$ROOT/scripts/test_p6g_real_card_builder_core.py,dst=/card-builder-test.py,readonly" \
 --entrypoint python3 "$IMAGE" /card-builder-test.py >"$TMP/run.log" 2>&1 || {
    echo "Genuine HACS release Card Builder failed Core lifecycle" >&2
    tail -130 "$TMP/run.log" >&2
    if [[ -f "$TMP/config/ha-http-debug.log" ]]; then tail -120 "$TMP/config/ha-http-debug.log" >&2; fi
    if [[ -f "$TMP/config/card-builder-error.txt" ]]; then tail -75 "$TMP/config/card-builder-error.txt" >&2; fi
    exit 1
 }
test -s "$TMP/config/card-builder-proof.json" || { echo "Real Card Builder proof absent" >&2; exit 1; }
python3 scripts/check_p6g_real_card_builder_report.py "$TMP/config/card-builder-proof.json"
python3 - "$TMP/config/card-builder-proof.json" <<'PY'
import json,sys
x=json.load(open(sys.argv[1]))
assert x["real_source"]=="studiobts/home-assistant-card-builder"
assert x["zip_release_sha256"]=="75ca0cc33397b16f2d2af4c2aad40391a84f013786c264a22b7186fe309fb7a7"
for key in ("core_config_flow_and_load","core_frontend_asset_registered","core_unload_and_reload","core_remove"):
  assert x[key] is True,key
assert x["real_user_dashboard_tested"] is False and x["production_ready"] is False
PY
echo P6G_REAL_HACS_CARD_BUILDER_RELEASE_CORE_LIFECYCLE_PASS
