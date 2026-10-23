# Rusty Home — independent public certification ledger

**Snapshot: 23 October 2026. Status: NOT_PRODUCTION_READY.**

**0 / 15 mandatory production release gates certified.** Five gates have
partial lab evidence; none is an independently qualified production gate.
The nine sample Apps published here are all fictional synthetic fixtures.

## Verified, narrowly scoped laboratory results

| Area | Real verified result | GitHub Actions evidence |
| --- | --- | --- |
| Rust native | Bearer auth, state restart, real SIGKILL recovery, fencing, bounded HTTP and synthetic radio flock | [Native Rust](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/39868514395) |
| Home Assistant Core | Real Core 2026.10.1, 1,232 synthetic states via private UDS to Rust, stale HTTP 503 | [Core bridge](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/39870909461) |
| Selected HACS | Eight authentic integration lifecycles on real Core, not 29/29 certification | [First three](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/39870909461), [five more](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/39871512116) |
| HAOS/Supervisor | Real HAOS 18.3 in isolated KVM, genuine File Editor, backup/restore, bad-password rejection: 16/16, no skipped tests | [HAOS KVM](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/39873207970) |
| Debian package | Reproducible test .deb install/update/downgrade/purge and cryptographic APT tamper rejection | [Debian and APT](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/39872009683) |
| Security fuzz | Real sanitizer-instrumented libFuzzer Core user parser with synthetic fixtures for 45 seconds | [Fuzz CI](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/39872710558) |

All of these are valid experiments with specific scopes. None accesses a
real household, proves all 29 custom components or all nine Apps, or
authorizes a hardware radio handoff or real migration.

## Product development evidence (NOT production certification)

- **2026-10-23, read-only Supervisor Job progress (product PR pending):**
  native Rust route and French dashboard poll official V1/V2 job endpoints
  by strictly validated UUID; output contains only state, progress, ID and
  error count, never raw stage, error messages or child jobs.
  Four control-plane adversarial tests and one authenticated TCP loopback
  integration test pass locally; full workspace/Clippy and independent CI
  must be recorded against the final PR SHA before merge.
  No user RBAC, trusted operation receipt or actual App mutation is implied.
  [Scope and limits](LIVE_SUPERVISOR_JOB_PROGRESS.md).


- **2026-10-23, live installed-App telemetry (product feature, PR pending):**
  native Rust V1/V2 Supervisor stats client, current installed-inventory
  gating, numeric-only redacted API and dashboard resource panel for
  arbitrary App slugs. Code commit
  1a5a0fe98bec4936b47438a78ead4163b5fd7098.
  TDD E0432/E0599 failure reproduced; four new adversarial loopback tests,
  Rust workspace all-target tests, strict Clippy, rustfmt, JavaScript syntax
  and public fixture guard passed in isolated LXC700 DEV.
  **Independent CI on the final PR SHA remains pending.**
  This does not certify production user RBAC, write permissions, HAOS
  integration, or any release gate.
  [Functional scope](LIVE_SUPERVISOR_APP_STATS.md).

- **2026-10-23, PR #37:** real read-only Supervisor Store App details for
  installed and uninstalled Apps, arbitrary vendor slugs, native Rust
  authenticated HTTP client, current Store membership validation and
  user-facing dashboard. Code commit:
  `8d1654b11fbcd8d19b099bf96a3b4bc43b7e617e`.
  TDD initially failed for missing methods/routes, then all Rust workspace
  all-target tests, strict Clippy and dashboard JavaScript syntax passed in
  isolated DEV. Independent GitHub CI, including disposable HAOS KVM,
  succeeded on refreshed head `fb25ae2eda428a8b71df94a7a136c74cef8e596a`
  ([HAOS run](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/39972631127));
  merged as `10bb01b2e57e81f0c2276c8342125da1baf54d46`.
  These tests do NOT prove App lifecycle write authorization. See
  [Store detail scope](STORE_LIVE_DETAILS.md) and
  [PR #37](https://github.com/GodsQuantum/Rusty-Home-Open/pull/37).

- **2026-10-23, PR #36 merged:** actual Rusty Home product App installed,
  started, checked non-root and cleaned up in disposable official HAOS 18.3
  KVM. [Independent HAOS workflow](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/39966193068)
  succeeded on PR head `82c51013c8ab4942138d621fff6d4d304e6deeff`;
  merge commit `a36cd02a35361af8a272b7b81ea5614990df97dc`.
  This does not establish human UI authentication or migration readiness.
- **2026-10-23, Store availability preflight:** Rust product code
  `afc1b6065771db60367d65357e7f83154f09b416` and installed-inventory
  reconciliation `260fac994745a1bc7e5a1fc5288ace689c3a4506` add live V1/V2
  availability reconciliation, fail-closed typed read-only API and dashboard.
  TDD first failed at E0432, then seven adversarial native tests, two
  control-plane tests, full Rust workspace all-target tests, strict Clippy,
  rustfmt and dashboard JavaScript syntax passed in isolated LXC700 DEV.
  Independent CI on final PR SHA is **pending**, not certified.
  [Preflight scope](STORE_AVAILABILITY_PREFLIGHT.md).

## Release gate matrix — every gate remains noncertified

| Exact gate identifier | Machine state | Production acceptance still needed |
| --- | --- | --- |
| `authenticated_host_transport` | `blocked` | Least-privileged authenticated live transport, expiry, outage recovery and adversarial audit |
| `ha_core_hacs_29_runtime` | `lab_partial` | Complete lifecycle coverage for all 29 targeted custom integrations |
| `hacs_github_oauth` | `lab_partial` | Genuine OAuth login, renewal, revocation, recovery and stale-session tests |
| `apps_9_lifecycle` | `blocked` | Install, start, update, stop and rollback for each of nine actual Apps |
| `supervisor_apps_restore` | `blocked` | Full App persistent data, ingress and service restore under Supervisor |
| `haos_backup_restore` | `lab_partial` | Protected full archive/data recovery end-to-end with user-equivalent data |
| `rollback_verified` | `blocked` | Revert actual migration including states, persistent data and fabric pairing |
| `single_radio_owner` | `blocked` | Exactly one real Zigbee, Thread, Matter and other device owner at each transition |
| `ha_ws_reconnection` | `lab_partial` | Real Home Assistant WebSocket reconnect and long-duration outage/loss tests |
| `production_ui_auth` | `blocked` | Full session, identity, role, token revocation and API authorization testing |
| `native_drivers_services` | `blocked` | Prove real device driver, service and system integration parity |
| `performance_full_stack` | `blocked` | Representative whole-stack scale, fault tolerance, resources and soak |
| `packaging_updates` | `blocked` | Production-signed distributions, installed real upgrades and rollback |
| `third_party_licenses` | `lab_partial` | Complete attribution, notices, source-license trace and signed SBOM |
| `security_stress_fuzz` | `blocked` | Complete attack-surface fuzz, penetration tests and long security soak |

## Effort estimates — not a certification score

- **About 80% of a laboratory alpha:** a subjective engineering estimate.
- **About 30% of a full Home Assistant replacement:** a subjective
  engineering-effort estimate with substantial uncertainty.
- **Precisely 0 / 15 production gates certified:** the objective result.

Hardware ownership, all selected HACS integrations, App data, OAuth and
real reversible migrations could reveal substantial unplanned work.
No date of production completion can be established from isolated labs.

## Recommended independent certification order

1. Review third-party license obligations and published assets.
2. Finish authenticated transport, browser access and 29 HACS lifecycles.
3. Prove nine App data and lifecycle migrations, then backup and rollback.
4. Prove real single-radio ownership under faults, full-stack load and soak.
5. Record independent decisions and artifacts for each of the 15 gates.

The only machine authority for production release gates is
[config/release-gates.json](../config/release-gates.json).
