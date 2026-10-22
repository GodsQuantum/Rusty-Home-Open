# Contributing to Rusty Home

Rusty Home is experimental, not a production smart-home controller.

1. Open an issue with a synthetic, reproducible example; **never upload
   personal backups, real IP addresses, device IDs, private credentials or
   personal automation history**.
2. Work in an isolated Rust/Linux test workspace. Do not touch a real
   household's Zigbee/Thread/Matter radios or an operational HAOS instance.
3. Run `cargo fmt --all -- --check`, `cargo test --locked --all`,
   `cargo clippy --locked --workspace --all-targets -- -D warnings` and
   `python3 scripts/check_release_gate.py --expect-blocked`.
4. Keep source provenance, pinned revisions and license metadata intact.
   New dependencies require a license review.
5. A laboratory test is evidence for that *specific test only*, not an
   authorization to remove a production release gate.

Follow [security](docs/SECURITY.md) and [production gates](docs/PRODUCTION_READINESS_2026-10-18.md).
