#!/usr/bin/env bash
# Public Epson SNMP v0.1.3 negative Core lifecycle. No actual printer.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
[[ "$(id -u)" != "0" && ( "${CODESPACES:-}" == "true" || "${GITHUB_ACTIONS:-}" == "true" ) ]] || { echo "GitHub isolated nonroot only" >&2; exit 2; }
REPO="$ROOT/references/epson-snmp"
mkdir -p references runtime
if [[ ! -d "$REPO/.git" ]]; then git clone --quiet --depth 1 --branch v0.1.3 https://github.com/thizanchettin/home-assistant-epson-snmp.git "$REPO"; fi
[[ "$(git -C "$REPO" rev-parse HEAD)" == "3a4f2c0120de7bfbf149041d484eefcc0b0cad0c" ]] || { echo "Epson tag mismatch" >&2; exit 3; }
echo "949d1dfcd9a67213bf414b0741fe08319bd65ecf1fde5c56bad412a320bca24e  $REPO/LICENSE" | sha256sum -c - >/dev/null
python3 - "$REPO/custom_components/epson_snmp/manifest.json" <<'PY'
import json,sys
m=json.load(open(sys.argv[1]))
assert m["domain"]=="epson_snmp" and m["version"]=="0.1.3" and m["config_flow"] is True
assert m["requirements"]==["pysnmp==7.1.22"]
PY
WHEEL="$ROOT/runtime/pysnmp-7.1.22-py3-none-any.whl"
if [[ ! -f "$WHEEL" ]]; then
 python3 - "$WHEEL" <<'PY'
import sys,urllib.request,json,hashlib
d=json.load(urllib.request.urlopen("https://pypi.org/pypi/pysnmp/7.1.22/json",timeout=30))
urls=[x for x in d["urls"] if x["filename"]=="pysnmp-7.1.22-py3-none-any.whl"]
assert len(urls)==1 and urls[0]["url"].startswith("https://files.pythonhosted.org/packages/")
b=urllib.request.urlopen(urls[0]["url"],timeout=40).read()
assert hashlib.sha256(b).hexdigest()=="57e704a6ba2bbf571d16cd5dc08b89bb3fa0ebeb5f4f26b87fececad3b3de7a6"
open(sys.argv[1],"wb").write(b)
PY
fi
echo "57e704a6ba2bbf571d16cd5dc08b89bb3fa0ebeb5f4f26b87fececad3b3de7a6  $WHEEL" | sha256sum -c - >/dev/null
TMP="$(mktemp -d "$ROOT/runtime/p6j-epson-XXXXXXXX")"
chmod 700 "$TMP"
trap 'rm -rf -- "$TMP"' EXIT
mkdir -p "$TMP/config/custom_components" "$TMP/config/vendor"
cp -a "$REPO/custom_components/epson_snmp" "$TMP/config/custom_components/epson_snmp"
cp "$WHEEL" "$TMP/config/vendor/"
cat > "$TMP/config/configuration.yaml" <<'YAML'
homeassistant:
  name: Rusty Home Epson SNMP offline test
  time_zone: Europe/Paris
YAML
IMAGE="ghcr.io/home-assistant/home-assistant@sha256:130241f28d01fa80dfa3b44f7d89781c188d8642ecab9024f079f19f9f82baa0"
if ! docker image inspect "$IMAGE" >/dev/null 2>&1; then docker pull "$IMAGE" >/dev/null; fi
docker run --rm --network none --user "$(id -u):$(id -g)" --read-only \
  --cap-drop ALL --security-opt no-new-privileges --pids-limit 192 \
  --memory 1900m --cpus 2 --tmpfs /tmp:rw,nosuid,nodev,size=96m \
  --tmpfs /run:rw,nosuid,nodev,size=48m --env PYTHONDONTWRITEBYTECODE=1 \
  --env PYTHONPATH=/config/vendor/pysnmp-7.1.22-py3-none-any.whl \
  --mount "type=bind,src=$TMP/config,dst=/config" \
  --mount "type=bind,src=$ROOT/scripts/test_p6j_real_epson_offline_core.py,dst=/epson-test.py,readonly" \
  --entrypoint python3 "$IMAGE" /epson-test.py >"$TMP/result.log" 2>&1 || {
   echo "Genuine pinned Epson SNMP Core failure" >&2
   tail -35 "$TMP/result.log" >&2
   if [[ -f "$TMP/config/epson-error.txt" ]]; then tail -65 "$TMP/config/epson-error.txt" >&2; fi
   exit 1
  }
python3 scripts/check_p6j_epson_report.py "$TMP/config/epson-proof.json"
