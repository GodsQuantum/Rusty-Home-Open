#!/usr/bin/env bash
# Official CUP HACS 1.5.0 release asset with real Core sensor exercise.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
[[ "$(id -u)" != 0 && ( "$(printenv CODESPACES || true)" == true || "$(printenv GITHUB_ACTIONS || true)" == true ) ]] || { echo "GitHub unprivileged only" >&2; exit 2; }
mkdir -p references runtime
SOURCE="$ROOT/references/cup-component"
if [[ ! -d "$SOURCE/.git" ]]; then
 git clone --quiet --depth 1 --branch v1.5.0 https://github.com/bastgau/ha-cup-component.git "$SOURCE"
fi
[[ "$(git -C "$SOURCE" rev-parse HEAD)" == ca9b767cb9bdd83d9e279947cd89fa742648d1df ]] || { echo "CUP Git tag unexpected"; exit 3; }
echo "4e7f19b5dd0c8d63985a6bf36f04317ea55b4d0e77c464bcc8cd03a6f4711f2e  $SOURCE/LICENSE" | sha256sum -c - >/dev/null
ASSET="$ROOT/runtime/cup_component_1.5.0.zip"
if [[ ! -f "$ASSET" ]]; then
 python3 - "$ASSET" <<'PYGET'
import hashlib,sys,urllib.request
blob=urllib.request.urlopen("https://github.com/bastgau/ha-cup-component/releases/download/v1.5.0/cup_component.zip",timeout=60).read()
assert len(blob)<1000000
assert hashlib.sha256(blob).hexdigest()=="52f6c367c8b039bb5d0b467206e357dc620c221ec2b255bd09c247b6205e0f51"
open(sys.argv[1],"wb").write(blob)
PYGET
fi
echo "52f6c367c8b039bb5d0b467206e357dc620c221ec2b255bd09c247b6205e0f51  $ASSET" | sha256sum -c - >/dev/null
TMP="$(mktemp -d "$ROOT/runtime/cup-p6l-XXXXXXXX")"
chmod 700 "$TMP"
NET="hars-p6l-$(basename "$TMP" | tr -cd a-zA-Z0-9-)"
NET_CREATED=false
cleanup(){
 if [[ "$NET_CREATED" == true ]]; then docker network rm "$NET" >/dev/null 2>&1 || true; fi
 rm -rf -- "$TMP"
}
trap cleanup EXIT
mkdir -p "$TMP/config/custom_components/cup_component"
python3 - "$ASSET" "$TMP/config/custom_components/cup_component" "$SOURCE/custom_components/cup_component/manifest.json" <<'PYZIP'
import json,stat,sys
from pathlib import PurePosixPath
from zipfile import ZipFile
src,tgt,dev=sys.argv[1:]
# Critical: source tag has version 0.0.0 placeholder. Must use actual release.
assert json.load(open(dev))["version"]=="0.0.0"
with ZipFile(src) as z:
 info=z.infolist()
 assert 5<len(info)<250 and sum(x.file_size for x in info)<3000000
 for f in info:
  name=PurePosixPath(f.filename)
  assert not name.is_absolute() and ".." not in name.parts
  assert not stat.S_ISLNK(f.external_attr>>16)
 z.extractall(tgt)
m=json.load(open(tgt+"/manifest.json"))
assert m["domain"]=="cup_component" and m["version"]=="v1.5.0"
assert m["requirements"]==[] and m["config_flow"] is True
PYZIP
cat > "$TMP/config/configuration.yaml" <<'YAML'
homeassistant:
  name: Rusty Home synthetic CUP v1.5.0
  time_zone: Europe/Paris
http:
  server_host: 127.0.0.1
YAML
docker network create --internal --driver bridge "$NET" >/dev/null
NET_CREATED=true
[[ "$(docker network inspect -f '{{.Internal}}' "$NET")" == true ]]
IMAGE="ghcr.io/home-assistant/home-assistant@sha256:33afb0659d786213264c2211f8d91c5fe87461c56eba6a3a546eb05fc71e89f4"
if ! docker image inspect "$IMAGE" >/dev/null 2>&1; then docker pull "$IMAGE" >/dev/null; fi
if ! docker run --rm --network "$NET" --user "$(id -u):$(id -g)" --read-only  --cap-drop ALL --security-opt no-new-privileges --pids-limit 256  --memory 2400m --cpus 2 --tmpfs /tmp:rw,nosuid,nodev,size=128m  --tmpfs /run:rw,nosuid,nodev,size=64m  --env PYTHONDONTWRITEBYTECODE=1 --env TZ=Europe/Paris --env P6L_DOCKER_INTERNAL=verified  --mount "type=bind,src=$TMP/config,dst=/config"  --mount "type=bind,src=$ROOT/scripts/test_p6l_real_cup_core.py,dst=/p6l-cup.py,readonly"  --entrypoint python3 "$IMAGE" /p6l-cup.py > "$TMP/run.log" 2>&1; then
 echo "P6L genuine CUP release Core sensor test failed" >&2
 tail -65 "$TMP/run.log" >&2
 if [[ -f "$TMP/config/p6l-error.txt" ]]; then tail -70 "$TMP/config/p6l-error.txt" >&2; fi
 exit 1
fi
test -s "$TMP/config/p6l-proof.json" || { echo "P6L proof absent" >&2; tail -65 "$TMP/run.log" >&2; exit 1; }
python3 scripts/check_p6l_cup_report.py "$TMP/config/p6l-proof.json"
echo P6L_GENUINE_CUP_RELEASE_CORE_SENSOR_PASS
