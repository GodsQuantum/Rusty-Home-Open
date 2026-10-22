#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
[[ "$(id -u)" != "0" && ( "${CODESPACES:-}" == "true" || "${GITHUB_ACTIONS:-}" == "true" ) ]] || { echo "GitHub isolated lab only" >&2; exit 2; }
IMAGE="ghcr.io/home-assistant/home-assistant@sha256:130241f28d01fa80dfa3b44f7d89781c188d8642ecab9024f079f19f9f82baa0"
ZIP_SHA="ec47b0114a5bf447ef1b7fa15cfa80829619f6785ece33ee4b5957ef906f7865"
mkdir -p runtime references
# The exact upstream tag is manifest v1.3.0 whereas its v1.9.0 RELEASE ZIP
# correctly declares v1.9.0. Track both immutable artifacts separately.
UPSTREAM="$ROOT/references/universal-player"
if [[ ! -d "$UPSTREAM/.git" ]]; then
  git clone --quiet --depth 1 --branch v1.9.0 https://github.com/bastgau/ha-custom-universal-media-player.git "$UPSTREAM"
fi
[[ "$(git -C "$UPSTREAM" rev-parse HEAD)" == "b07faa8840ce95567748bd798d3d9bbf30c2c433" ]] || {
  echo "upstream Git tag retargeted" >&2; exit 3;
}
echo "4e7f19b5dd0c8d63985a6bf36f04317ea55b4d0e77c464bcc8cd03a6f4711f2e  $UPSTREAM/LICENSE" | sha256sum -c - >/dev/null
python3 - "$UPSTREAM/custom_components/custom_universal_media_player/manifest.json" <<'PY'
import json,sys
m=json.load(open(sys.argv[1]))
assert m["version"]=="v1.3.0" and m["domain"]=="custom_universal_media_player"
PY
ZIP="$ROOT/runtime/custom_universal_media_player.zip"
if [[ ! -f "$ZIP" ]]; then
  gh release download v1.9.0 -R bastgau/ha-custom-universal-media-player -p custom_universal_media_player.zip -D "$ROOT/runtime"
fi
echo "$ZIP_SHA  $ZIP" | sha256sum -c - >/dev/null
TMP="$(mktemp -d "$ROOT/runtime/hars-universal-XXXXXXXX")"
chmod 700 "$TMP"
cleanup() { rm -rf -- "$TMP"; }
trap cleanup EXIT
DEST="$TMP/config/custom_components/custom_universal_media_player"
mkdir -p "$DEST"
python3 - "$ZIP" "$DEST" <<'PY'
import json,stat,sys,zipfile
from pathlib import Path, PurePosixPath
archive=Path(sys.argv[1]); out=Path(sys.argv[2])
with zipfile.ZipFile(archive) as z:
    names=z.namelist()
    assert len(names)==len(set(names)) and len(names)==11, "unexpected/duplicate archive entries"
    assert sum(x.file_size for x in z.infolist())<4*1024*1024
    for entry in z.infolist():
        name=PurePosixPath(entry.filename)
        assert len(name.parts)<5 and not name.is_absolute() and ".." not in name.parts
        assert not (stat.S_IFMT(entry.external_attr >> 16)==stat.S_IFLNK)
        assert entry.file_size<2*1024*1024
        target=out.joinpath(*name.parts)
        if entry.is_dir():target.mkdir(parents=True,exist_ok=True);continue
        target.parent.mkdir(parents=True,exist_ok=True)
        target.write_bytes(z.read(entry))
        target.chmod(0o644)
    m=json.loads((out/"manifest.json").read_text())
    assert m["domain"]=="custom_universal_media_player" and m["version"]=="v1.9.0"
    assert m["config_flow"] is True and m["requirements"]==[]
PY
cat > "$TMP/config/configuration.yaml" <<'YAML'
homeassistant:
  name: Rusty Home fictitious media config-flow test
  time_zone: Europe/Paris
YAML
if ! docker image inspect "$IMAGE" >/dev/null 2>&1; then docker pull "$IMAGE" >/dev/null; fi
docker run --rm --network none --user "$(id -u):$(id -g)" --read-only \
 --cap-drop ALL --security-opt no-new-privileges --pids-limit 256 \
 --memory 1900m --cpus 2 --tmpfs /tmp:rw,nosuid,nodev,size=96m \
 --tmpfs /run:rw,nosuid,nodev,size=48m --env PYTHONDONTWRITEBYTECODE=1 \
 --env TZ=Europe/Paris \
 --mount "type=bind,src=$TMP/config,dst=/config" \
 --mount "type=bind,src=$ROOT/scripts/test_p6h_real_universal_core.py,dst=/universal-test.py,readonly" \
 --entrypoint python3 "$IMAGE" /universal-test.py >"$TMP/core.log" 2>&1 || {
   echo "Genuine Core universal player lifecycle FAILED" >&2
   tail -30 "$TMP/core.log" >&2
   if test -f "$TMP/config/universal-error.txt"; then tail -65 "$TMP/config/universal-error.txt" >&2; fi
   exit 1
 }
python3 scripts/check_p6h_universal_report.py "$TMP/config/universal-proof.json"
echo P6H_REAL_PUBLIC_UNIVERSAL_MEDIA_PLAYER_1_9_0_LIFECYCLE_PASS
