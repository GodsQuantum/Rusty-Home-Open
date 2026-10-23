# Marketplace first: HACS compatibility plan for Rusty Home

**Decision date:** 23 October 2026. **Official stable HA today: 2026.10**.

Home Assistant Core merges HACS as the built-in **Marketplace** for
**2026.11**, a *future* stable release at this date. The Marketplace source
is present on upstream dev. Do not incorrectly claim it is in 2026.10.

## Three boundaries — do not duplicate official services

1. **Native Rust core** owns Rusty Home state, event durability, scheduling,
   safe automation execution, and independently certified native device
   drivers. The current synthetic process flock is not real hardware fencing.
2. **Official HA Python Core compatibility sidecar** owns all Python custom
   integrations and in 2026.11+ the actual Marketplace: repository catalog,
   migration of existing HACS metadata, installs/uninstalls, updates,
   download verification, admin sessions and GitHub OAuth. Rust does NOT
   build a parallel HACS registry or install custom integration code.
3. **Official HAOS Supervisor Apps** remain separate from the Marketplace.
   An HAOS App is never a HACS custom integration. Apps use Supervisor
   Store and their own backup, ingress, restore and lifecycle semantics.

## Compatibility matrix

| Home Assistant | Community package manager | Authorized architecture |
| --- | --- | --- |
| 2026.10 stable and earlier | HACS installed separately | Existing HACS process owns installs, update entities, GitHub OAuth; Rust inspect-only |
| 2026.11 dev / future stable | Core integration marketplace | Core migrates existing HACS repositories and serves official catalog; Rust read-only WebSocket adapter |
| All supported HAOS versions | Settings > Apps | Supervisor-only app lifecycle; not exposed as Marketplace |

Official Marketplace supports Integration, Dashboard plugin, Theme and
Template. Legacy AppDaemon and Python scripts become unmanaged; they
must NOT be quietly reinstalled or deleted by Rust. Removed catalog
entries keep files but stop updating; Critical items belong entirely to
Core's own emergency handling. In particular Rust must not intercept,
override or attempt to preempt emergency uninstalls.

## Real source-backed Core WS commands

Upstream Home Assistant Core dev SHA
3bec44d0470b044bb41febd5c7f4046f02bd2fb1,
homeassistant/components/marketplace/websocket:

- marketplace/info (admin only)
- marketplace/repositories/list, optional categories (admin only)
- marketplace/repository/info, exact opaque repository_id (admin only)

Rust code: crates/community-packages/src/marketplace_ws.rs,
unit tests tests/marketplace_ws.rs. The builder only constructs these
commands after version gating. The actual Home Assistant websocket and
server-side session remain independently authenticated by Home Assistant.
Rust performs no transport, token acquisition, installation or delete.

GitHub runner contract: scripts/test_marketplace_core_contract.py downloads
**immutable pinned official Core source**, checks the exact command
registrations/parameters and require_admin decorator. Failing upstream
source comparison FAILS CI; Rust's own unit tests prohibit injection and
authorization-by-response spoofing. This is a **source contract test,
not a runtime WebSocket integration test**.

## Acceptance work remaining

- Authenticate a disposable WebSocket session with genuine Core dev
  2026.11 / official future release, verify all three read commands,
  admin/nonadmin denial, Core restart and token expiry with no real user.
- Validate real HACS-to-Marketplace migration of 29 *actual target*
  integrations, preserving versions and update entities, not merely
  synthetic names. Stop on unsupported Python scripts/AppDaemon.
- Prove migration failure rollback and Core-provided Critical handling
  inside official disposable Core; do not implement a second installer.
- Verify full 9-App Supervisor matrix separately on disposable HAOS;
  real user backups, radio ownership and household cutover remain blocked.

**Release state remains NOT_PRODUCTION_READY: 0/15 gates certified.**

Official references:
- https://contributors.home-assistant.io/marketplace/how-it-works/
- https://contributors.home-assistant.io/marketplace/reference/repository-structure/
- https://contributors.home-assistant.io/marketplace/reference/validation/
- https://www.home-assistant.io/apps

## Genuine official 2026.11-dev OCI runtime registration test

The separate GitHub Actions job runs the actual pre-release Core image
from ghcr.io/home-assistant/home-assistant pinned to digest
sha256:ab9a72edb5cf22bc43047a58a62805bb4d378a6fb85dc5178ee3476eb0523f27
inside a fresh disposable Docker container with no network, no Linux
capabilities, a read-only root filesystem and no production secrets. It
imports the actual Core Marketplace Python modules and invokes the real
WebSocket command registration function with an instrumented registry,
checking that the three required reader handlers and original writer
handlers exist and that registration has no duplicates. This is a **real
pre-release Core runtime import/registration test**, distinct from the
pinned official Python SOURCE decorator contract test. It does not
exercise a fully authenticated admin WebSocket exchange, nor claim
production app/marketplace compatibility.
