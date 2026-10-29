# Contributing to Rusty Home

## Development paused — community contributions welcome

Rusty Home has no active maintainer-sponsored development, support commitment or release date. It is **not installable or production-ready** (0/15 mandatory production gates). You may fork, open issues or propose PRs, but **responses, reviews and merges are not guaranteed**. Never request installation in a live household or submit real home backups, credentials, device identifiers or residential addresses.

Start with a small reproducible issue tied to an unmet gate in [project status](docs/PROJECT_STATUS.md). Preserve any existing independent work: [PR #264](https://github.com/GodsQuantum/Rusty-Home-Open/pull/264), [#266](https://github.com/GodsQuantum/Rusty-Home-Open/pull/266), [#268](https://github.com/GodsQuantum/Rusty-Home-Open/pull/268) and [#269](https://github.com/GodsQuantum/Rusty-Home-Open/pull/269) remain unmerged and are not a ready release. A volunteer can continue development independently in a fork.


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
