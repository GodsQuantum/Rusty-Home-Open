# Upstream dependencies

Rusty Home is not a Home Assistant fork.

The source tree uses pinned external Rust libraries including Moli and
Home Assistant WebSocket compatibility clients. See Cargo.lock and Cargo
manifests. Library presence does not constitute redistribution clearance.

## Home Assistant Core 2026.11 Marketplace (upcoming)\n\nOfficial Core dev code pinned for source-based ABI tests: commit\n3bec44d0470b044bb41febd5c7f4046f02bd2fb1 under\nhomeassistant/components/marketplace/websocket/. This is not a vendored\nPython distribution. Real installs, OAuth and updates stay in official Core.\nSee MARKETPLACE_CORE_2026_11_ARCHITECTURE.md.\n
## Latest verified stable Core test baseline, October 2026

Official Home Assistant Core **2026.10.2** (released 16 October 2026) is now
required for the isolated Core bridge and all selected HACS lifecycle labs,
pinned by OCI digest in [config/ha-core-pins.json](../config/ha-core-pins.json).
The independent 2026.11 development image is pinned separately for the
upcoming official Core Marketplace. The pin consistency test fails CI on
any accidental mixing or silent regression to older test images.
