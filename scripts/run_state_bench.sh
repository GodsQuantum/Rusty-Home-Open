#!/usr/bin/env bash
# Compare real RuView HOMECORE in-memory state and homeCore-io persisted state
# with 1232 entirely synthetic entities. Different durability semantics.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
[[ "$(git -C "$ROOT/references/RuView-HOMECORE" rev-parse HEAD)" == "a1ca023ee872683d160250d47ae3d4d228af55a0" ]] || exit 2
[[ "$(git -C "$ROOT/references/homeCore-io" rev-parse HEAD)" == "cb18ee2b1b67a235e30cdaa04adaa61ac1e7fc38" ]] || exit 2
python3 "$ROOT/scripts/generate_state_bench_workspace.py"
cp "$ROOT/scripts/state_bench_main.rs" "$ROOT/runtime/bench-equal/src/main.rs"
export CARGO_TARGET_DIR="$ROOT/runtime/bench-builds/comparison" CARGO_BUILD_JOBS=4
cd "$ROOT/runtime/bench-equal"
cargo run --release --quiet
