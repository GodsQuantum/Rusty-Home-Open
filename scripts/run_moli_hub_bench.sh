#!/usr/bin/env bash
# Synthetic Moli runtime, one in-process fake driver; NO real I/O.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
S="$ROOT/runtime/bench-moli-hub"
mkdir -p "$S/src"
cat > "$S/Cargo.toml" <<'TOML'
[workspace]

[package]
name = "hars-bench-moli-hub"
version = "0.0.1"
edition = "2024"
publish = false

[dependencies]
moli-core = { git = "https://github.com/rudy-io/moli-os.git", rev = "d54c3de46aa453691f0c96ddc42e853a8a964178" }
moli-runtime = { git = "https://github.com/rudy-io/moli-os.git", rev = "d54c3de46aa453691f0c96ddc42e853a8a964178" }
anyhow = "1"
tokio = { version = "1", features = ["rt-multi-thread", "macros", "sync"] }
tokio-util = "0.7"
tempfile = "3"
serde_json = "1"
TOML
cp "$ROOT/scripts/moli_hub_bench_main.rs" "$S/src/main.rs"
export CARGO_TARGET_DIR="$ROOT/runtime/bench-builds/moli-hub"
export CARGO_BUILD_JOBS=4
cd "$S"
cargo run --release --quiet
