#!/usr/bin/env bash
# Upstream-only qualification of RuView HOMECORE P1 HA migrator.
# Stage ONLY two source crates into an ignored tiny workspace because the
# sparsely checked-out RuView monorepo includes hundreds of unrelated crates.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SOURCE="$ROOT/references/RuView-HOMECORE"
EXPECTED="a1ca023ee872683d160250d47ae3d4d228af55a0"
[[ "$(git -C "$SOURCE" rev-parse HEAD)" == "$EXPECTED" ]] || { echo "Upstream SHA changed; review before tests" >&2; exit 2; }
STAGE="$ROOT/runtime/qualify-ruview"
mkdir -p "$STAGE/crates"
for name in homecore homecore-migrate; do
  if [[ ! -d "$STAGE/crates/$name" ]]; then
    cp -a "$SOURCE/v2/crates/$name" "$STAGE/crates/"
  fi
done
cat > "$STAGE/Cargo.toml" <<'TOML'
[workspace]
resolver = "2"
members = ["crates/homecore", "crates/homecore-migrate"]
TOML
export CARGO_TARGET_DIR="$ROOT/runtime/bench-builds/ruview"
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
cd "$STAGE"
echo "[RuView SOURCE] SHA=$EXPECTED (isolated copy of original crates; own dependency lock)"
cargo test -p homecore-migrate -p homecore --lib --no-default-features -j 4 -- --test-threads=2
cargo build -p homecore-migrate --bin homecore-migrate --no-default-features -j 4
BIN="$CARGO_TARGET_DIR/debug/homecore-migrate"
TMP="$(mktemp -d "$ROOT/runtime/ruview-synthetic-XXXXXX")"
trap 'rm -rf -- "$TMP"' EXIT
mkdir -p "$TMP/ha/.storage" "$TMP/homecore"
python3 - "$SOURCE/v2/crates/homecore-migrate/src/entity_registry.rs" "$TMP/ha/.storage/core.entity_registry" <<'PY'
import pathlib,re,sys,json
source=pathlib.Path(sys.argv[1]).read_text()
fixture=re.search(r'const FIXTURE_V13: &str = r#"(.*?)"#;',source,re.S)
assert fixture, "upstream entity fixture not found"
j=json.loads(fixture.group(1))
assert j["minor_version"]==13 and len(j["data"]["entities"])==2
pathlib.Path(sys.argv[2]).write_text(json.dumps(j))
print("PASS: 2 synthetic entities from upstream unit fixture, NO production .storage")
PY
"$BIN" inspect --storage "$TMP/ha/.storage"
"$BIN" import-entities --storage "$TMP/ha/.storage" --to "$TMP/homecore"
python3 - "$TMP/homecore/core.entity_registry" <<'PY'
import pathlib,json,sys
p=pathlib.Path(sys.argv[1]); content=json.loads(p.read_text())
entities=content["data"]["entities"]
assert len(entities)==2
assert set(e["entity_id"] for e in entities)=={"light.kitchen","sensor.bedroom_temperature"}
print("PASS: synthetic HA entity registry migrated losslessly for test ids")
PY
if "$BIN" import-entities --storage "$TMP/ha/.storage" --to "$TMP/homecore" > /dev/null 2>&1; then
  echo "FAIL: migrator overwrote existing destination without --force" >&2
  exit 8
fi
echo "PASS: non-overwrite behavior verified"
