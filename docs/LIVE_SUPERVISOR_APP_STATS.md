# Live Supervisor App resource metrics (read-only)

Rusty Home provides a per-installed-App "Ressources en direct" action in the dashboard.
The native Rust client makes a fresh authenticated Supervisor GET /addons/<slug>/stats
request (explicit V2: GET /v2/apps/<slug>/stats) using the internally issued
Supervisor token. No Docker socket, raw device files, mutations, or arbitrary
Supervisor URL is accepted from the browser.

GET /api/apps/<slug>/stats is available only with exactly one SupervisorLive
adapter and a slug independently confirmed in the current official installed
inventory. Missing Apps return 404; absent or conflicting sources, failed
authentication, transport errors, malformed responses and unsupported framing
fail closed. Unsafe or encoded slugs return 400.

The response is strictly allowlisted to numeric CPU/memory/network counters,
an exact slug and read_only: true. No raw logs, options, IP addresses, tokens,
schemas, device identifiers, or unknown fields are copied into the dashboard.
The UI uses text nodes, not HTML injection. This is not a historical metrics
database or authorization to start, stop, install or update Apps.

## Verification scope

- Four adversarial Rust integration tests: official V1/V2 loopback GET,
  token forwarding, arbitrary vendor slug, secret-field dropping, malformed
  or duplicate JSON, invalid numbers, unsafe slugs, 401/403/302 handling.
- Rust control-plane route tests: V1/V2, method and slug validation,
  snapshot rejection, ambiguous live-source rejection, UI contract.
- TDD compilation initially failed with E0432/E0599 (missing module/methods).
- cargo test --locked --workspace --all-targets --quiet, strict Clippy,
  rustfmt, public fixture guard and JavaScript syntax passed in LXC700 DEV.
- Independent GitHub Actions on the final PR SHA must be recorded separately.
  Loopback tests do not establish live HAOS permissions or production security.

Official upstream contract:
https://contributors.home-assistant.io/apps/supervisor-api/apps/

## Next product increment

Trusted ingress user identity and Core-verified RBAC for operations; then
durable server-generated operation receipts, idempotent lifecycle writes,
job tracking, restart recovery and rollback. No synthetic claim or App
inventory is sufficient to authorize a write.

All 15 release gates remain noncertified. Never use VM100 or real household
data for unattended development.
