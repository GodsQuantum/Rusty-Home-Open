# Public independent tests with genuine Home Assistant Core

This workflow runs on free standard GitHub-hosted `ubuntu-24.04`
runners. It downloads an immutable official Home Assistant Core 2026.10.2
container image at its pinned sha256 digest. It then executes **real Python
integration code** inside Core against synthetic fixtures, not a mocked
Core import API.

Tests and boundaries:

- P13: 1,232 synthetic Core StateMachine entities flow through local UDS
  with a private capability to the Rust read-only HTTP API; after the
  sidecar exits, the next read must return HTTP 503.
- P6d: genuine upstream Watchman 0.8.4 pinned sources and requirements
  inside the real Core config-entry lifecycle.
- P6k: genuine Blueprint Studio 2.5.2 release verifies Core HTTP
  authentication/authorization boundaries between admin and non-admin
  synthetic users.
- P6l: genuine CUP Component 1.5.0 **published ZIP** (not the dev
  placeholder manifest) updates real Core sensor objects, using a
  local synthetic target.

External upstream assets are SHA-256 or exact Git commit pinned.
Docker runs with restricted privileges, no radio or USB access, ephemeral
state, read-only filesystem and internal/disabled networks during Core
execution. No production HAOS backups or personal credentials exist
in this public repository.

**Scope**: These are isolated runtime acceptance tests of three selected
HACS integrations, not 29/29 compatibility, not nine real Apps,
not live device-driver validation and not a household migration.

All 15 mandatory production gates remain blocked, with no production
signoff or cutover authority.

- P6c genuine isolated Home Assistant Core 2026.10.2 config-entry lifecycle:\n  fixture config flow, create, setup, unload and remove, with independent\n  report verification. This is a single synthetic integration, not 29/29.\n