# Rusty Home · instrumented Rust parser fuzzing

Scope: genuine Core user WebSocket decoder, actual Rusty Home Supervisor ingress HTTP parser, untrusted official Supervisor Store V1/V2 catalog decoder and seven-operation
native lifecycle preflight across conflicting Store/Detail/Inventory views. This is a **limited security test**, not full penetration testing or production certification.

Requires nightly Rust and cargo-fuzz (pinned in CI) on a Linux x86_64 Codespace; no synthetic VM, devices, tokens or network required by the target.

```bash
rustup toolchain install nightly-2026-10-15 --profile minimal
cargo install cargo-fuzz --version 0.13.2 --locked
cd fuzz
cargo +nightly-2026-10-15 fuzz run core_user_snapshot -- -max_total_time=60 -timeout=10 -rss_limit_mb=2048
cargo +nightly-2026-10-15 fuzz run ingress_http -- -max_total_time=60 -timeout=10 -rss_limit_mb=2048
cargo +nightly-2026-10-15 fuzz run supervisor_store -- -max_total_time=60 -timeout=10 -rss_limit_mb=2048
cargo +nightly-2026-10-15 fuzz run lifecycle_preflight -- -max_total_time=60 -timeout=10 -rss_limit_mb=2048
```

The crate is a separate workspace: it does not change the production crate's root Cargo.lock. Seed corpus includes only fake synthetic user IDs. Avoid committing crash artifacts containing genuine secrets (none should enter this corpus). CI performs a time-bounded, sanitizer-instrumented libFuzzer smoke; the `security_stress_fuzz` release gate stays BLOCKED until extended fuzzing, more attack surfaces, vulnerability review, fault injection and penetration tests have actually passed.

Official guidance: https://github.com/rust-fuzz/cargo-fuzz and https://rust-fuzz.github.io/book/cargo-fuzz.html


The GitHub Actions pull-request job runs all four sanitizers for 45 seconds each.
Weekly scheduled and manually dispatched runs additionally execute 180 seconds
per target, still subject to the GitHub runner time limit. These are bounded
laboratory tests, not a 24-hour soak or release certification.

### P15 fifth source: actual HAOS archive import parser (Oct 26)
The bounded unprotected official-format Supervisor full TAR and nested Core TAR.GZ
are fuzzed by invoking the actual hars_interop_moli::ha_backup_import importer,
including archive entry/path policy, duplicate registry and automation names,
UTF-8, gzip validation, and the strict Core entity registry. The corpus
contains only fictitious entity identities and inert YAML. No HAOS/VM100,
network, real tokens, or devices are ever opened. A passing 45s/180s libFuzzer
run is a limited instrumented lab result, not a penetration test, soak,
HAOS migration certification, or production authorization.
