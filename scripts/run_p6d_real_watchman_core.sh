#!/usr/bin/env bash
# Exact public Watchman 0.8.4 sources and dependency, genuine isolated Core.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
if [[ "$(id -u)" == "0" || ( "${CODESPACES:-}" != "true" && "${GITHUB_ACTIONS:-}" != "true" ) ]]; then
  echo "Requires unprivileged Codespaces or GitHub Actions" >&2
  exit 2
fi
UPSTREAM="2eab21b7aeb4d4e20c71c3e2b3e449c42c4a3b35"
LICENSE_SHA="8c81a95dbbcaccd99769c7fd5c2bee891ff8b7fdca45f70d046adcb22120d6ff"
WHEEL_SHA="77ca0ad1c435b6e363d7e8623d7cc4fcf2cf15513bf77a1c1b2e814930ac57cc"
SOURCE="$ROOT/references/watchman"
mkdir -p references runtime
if [[ ! -d "$SOURCE/.git" ]]; then
  git clone --quiet --depth 1 --branch v0.8.4 https://github.com/dummylabs/thewatchman.git "$SOURCE"
fi
test "$(git -C "$SOURCE" rev-parse HEAD)" = "$UPSTREAM" || { echo "Watchman git SHA mismatch" >&2; exit 3; }
echo "$LICENSE_SHA  $SOURCE/LICENSE" | sha256sum -c - >/dev/null
python3 - "$SOURCE/custom_components/watchman/manifest.json" <<'PY'
import json,sys
m=json.load(open(sys.argv[1]))
assert m["domain"]=="watchman" and m["version"]=="0.8.4"
assert m["config_flow"] is True and m["requirements"]==["prettytable==3.12.0"]
PY
WHEEL="$ROOT/runtime/prettytable-3.12.0-py3-none-any.whl"
if [[ ! -f "$WHEEL" ]]; then
  python3 - "$WHEEL" <<'PY'
import hashlib,json,sys,urllib.request
data=json.load(urllib.request.urlopen("https://pypi.org/pypi/prettytable/3.12.0/json",timeout=30))
candidates=[x for x in data["urls"] if x["filename"]=="prettytable-3.12.0-py3-none-any.whl"]
assert len(candidates)==1 and candidates[0]["url"].startswith("https://files.pythonhosted.org/packages/")
payload=urllib.request.urlopen(candidates[0]["url"],timeout=45).read()
assert hashlib.sha256(payload).hexdigest()=="77ca0ad1c435b6e363d7e8623d7cc4fcf2cf15513bf77a1c1b2e814930ac57cc"
open(sys.argv[1],"wb").write(payload)
PY
fi
echo "$WHEEL_SHA  $WHEEL" | sha256sum -c - >/dev/null
TMP="$(mktemp -d "$ROOT/runtime/hars-watchman-XXXXXXXX")"
chmod 700 "$TMP"
cleanup() { rm -rf -- "$TMP"; }
trap cleanup EXIT
mkdir -p "$TMP/config/custom_components" "$TMP/config/vendor"
chmod 700 "$TMP/config" "$TMP/config/custom_components" "$TMP/config/vendor"
cp -a "$SOURCE/custom_components/watchman" "$TMP/config/custom_components/watchman"
cp "$WHEEL" "$TMP/config/vendor/"
cat > "$TMP/config/configuration.yaml" <<'YAML'
homeassistant:
  name: Rusty Home Watchman offline proof
  time_zone: Europe/Paris
YAML
IMAGE="ghcr.io/home-assistant/home-assistant@sha256:130241f28d01fa80dfa3b44f7d89781c188d8642ecab9024f079f19f9f82baa0"
if ! docker image inspect "$IMAGE" >/dev/null 2>&1; then docker pull "$IMAGE" >/dev/null; fi
docker run --rm --network none --user "$(id -u):$(id -g)" --read-only \
  --cap-drop ALL --security-opt no-new-privileges --pids-limit 256 \
  --memory 2200m --cpus 2 --tmpfs /tmp:rw,nosuid,nodev,size=128m \
  --tmpfs /run:rw,nosuid,nodev,size=64m \
  --env PYTHONDONTWRITEBYTECODE=1 --env TZ=Europe/Paris \
  --env PYTHONPATH=/config/vendor/prettytable-3.12.0-py3-none-any.whl \
  --mount "type=bind,src=$TMP/config,dst=/config" \
  --mount "type=bind,src=$ROOT/scripts/test_p6d_real_watchman_core.py,dst=/watchman-proof.py,readonly" \
  --entrypoint python3 "$IMAGE" /watchman-proof.py >"$TMP/run.log" 2>&1 || {
    echo "Real pinned Watchman Core lifecycle failed" >&2
    tail -45 "$TMP/run.log" >&2
    if test -f "$TMP/config/watchman-failure.txt"; then tail -65 "$TMP/config/watchman-failure.txt" >&2; fi
    exit 1
  }
test -s "$TMP/config/watchman-proof.json" || { echo "Watchman full proof missing" >&2; exit 1; }
python3 scripts/check_p6d_real_watchman_report.py "$TMP/config/watchman-proof.json"
