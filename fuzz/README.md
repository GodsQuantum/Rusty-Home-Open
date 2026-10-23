# Rusty Home · instrumented Rust parser fuzzing

Scope: genuine Core user WebSocket decoder and actual Rusty Home Supervisor ingress HTTP parser. This is a **limited security test**, not full penetration testing or production certification.

Requires nightly Rust and cargo-fuzz (pinned in CI) on a Linux x86_64 Codespace; no synthetic VM, devices, tokens or network required by the target.

```bash
rustup toolchain install nightly-2026-10-15 --profile minimal
cargo install cargo-fuzz --version 0.13.2 --locked
cd fuzz
cargo +nightly-2026-10-15 fuzz run core_user_snapshot -- -max_total_time=60 -timeout=10 -rss_limit_mb=2048
cargo +nightly-2026-10-15 fuzz run ingress_http -- -max_total_time=60 -timeout=10 -rss_limit_mb=2048
```

The crate is a separate workspace: it does not change the production crate's root Cargo.lock. Seed corpus includes only fake synthetic user IDs. Avoid committing crash artifacts containing genuine secrets (none should enter this corpus). CI performs a time-bounded, sanitizer-instrumented libFuzzer smoke; the `security_stress_fuzz` release gate stays BLOCKED until extended fuzzing, more attack surfaces, vulnerability review, fault injection and penetration tests have actually passed.

Official guidance: https://github.com/rust-fuzz/cargo-fuzz and https://rust-fuzz.github.io/book/cargo-fuzz.html
