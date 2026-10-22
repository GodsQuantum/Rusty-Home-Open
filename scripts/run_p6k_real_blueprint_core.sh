#!/usr/bin/env bash
# Blueprint Studio real 2.5.2 and actual HA Core HTTP bearer+RBAC fixture.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
[[ "$(id -u)" != 0 && ( "$(printenv CODESPACES || true)" == true || "$(printenv GITHUB_ACTIONS || true)" == true ) ]] || { echo "GitHub unprivileged only" >&2; exit 2; }
mkdir -p references runtime
SOURCE="$ROOT/references/blueprint-studio"
if [[ ! -d "$SOURCE/.git" ]]; then
  git clone -q --depth 1 --branch v2.5.2 https://github.com/ha-china/blueprint-studio.git "$SOURCE"
fi
[[ "$(git -C "$SOURCE" rev-parse HEAD)" == "943aed0be67f3a910b0f70a3864288d5e17e55ce" ]] || { echo "P6k upstream SHA drift"; exit 3; }
echo "7128bd3f5719878303d0d4f3d4d0239976c84977bb64ef2e18df2591b5cc2dfa  $SOURCE/LICENSE" | sha256sum -c - >/dev/null
python3 - "$SOURCE/custom_components/blueprint_studio/manifest.json" <<'PYMANIFEST'
import json,sys
m=json.load(open(sys.argv[1]))
assert m["domain"]=="blueprint_studio" and m["version"]=="2.5.2"
assert m["config_flow"] is True and m["integration_type"]=="service"
assert m["dependencies"]==["http","frontend"] and m["requirements"]==["paramiko>=3.0.0"]
PYMANIFEST
TMP="$(mktemp -d "$ROOT/runtime/blueprint-p6k-XXXXXXXX")"
chmod 700 "$TMP"
NET="hars-p6k-$(basename "$TMP" | tr -cd a-zA-Z0-9-)"
NET_CREATED=false
cleanup() {
  if [[ "$NET_CREATED" == true ]]; then docker network rm "$NET" >/dev/null 2>&1 || true; fi
  rm -rf -- "$TMP"
}
trap cleanup EXIT
mkdir -p "$TMP/config/custom_components"
cp -a "$SOURCE/custom_components/blueprint_studio" "$TMP/config/custom_components/"
cat > "$TMP/config/configuration.yaml" <<'YAML'
homeassistant:
  name: Rusty Home synthetic Blueprint Studio 2.5.2
  time_zone: Europe/Paris
http:
  server_host: 127.0.0.1
YAML
docker network create --internal --driver bridge "$NET" >/dev/null
NET_CREATED=true
[[ "$(docker network inspect -f '{{.Internal}}' "$NET")" == true ]]
IMAGE="ghcr.io/home-assistant/home-assistant@sha256:130241f28d01fa80dfa3b44f7d89781c188d8642ecab9024f079f19f9f82baa0"
if ! docker image inspect "$IMAGE" >/dev/null 2>&1; then docker pull "$IMAGE" >/dev/null; fi
if ! docker run --rm --network "$NET" --user "$(id -u):$(id -g)" --read-only   --cap-drop ALL --security-opt no-new-privileges --pids-limit 256   --memory 2400m --cpus 2 --tmpfs /tmp:rw,nosuid,nodev,size=128m   --tmpfs /run:rw,nosuid,nodev,size=64m   --env PYTHONDONTWRITEBYTECODE=1 --env TZ=Europe/Paris --env P6K_DOCKER_INTERNAL=verified   --mount "type=bind,src=$TMP/config,dst=/config"   --mount "type=bind,src=$ROOT/scripts/test_p6k_blueprint_real_core.py,dst=/p6k-blueprint.py,readonly"   --entrypoint python3 "$IMAGE" /p6k-blueprint.py > "$TMP/run.log" 2>&1; then
  echo "P6K genuine Blueprint Core lifecycle failed" >&2
  tail -65 "$TMP/run.log" >&2
  if [[ -f "$TMP/config/blueprint-error.txt" ]]; then tail -65 "$TMP/config/blueprint-error.txt" >&2; fi
  exit 1
fi
test -s "$TMP/config/blueprint-proof.json" || { echo "P6K real Core proof absent" >&2; tail -60 "$TMP/run.log" >&2; exit 1; }
python3 scripts/check_p6k_blueprint_report.py "$TMP/config/blueprint-proof.json"
echo P6K_GENUINE_RELEASED_BLUEPRINT_CORE_ROLE_PASS
