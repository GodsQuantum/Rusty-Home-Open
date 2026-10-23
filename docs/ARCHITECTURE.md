# Architecture (experimental)

Native Rust kernel prototype, local-only read-only control plane, optional
isolated Home Assistant bridges and strictly synthetic Supervisor adapters.

This is not certified for live device control or migration. No external
device is contacted by the synthetic default test suites.

## 2026.11 Core Marketplace and Rust-first boundary

See [the tested Marketplace-first architectural decision](MARKETPLACE_CORE_2026_11_ARCHITECTURE.md).
Native Rust is the home automation engine, while actual third-party Python integration management belongs to the official Core Marketplace sidecar (on 2026.11+) or HACS (2026.10). Supervisor Apps are separate. No parallel installer and no production radio ownership.
