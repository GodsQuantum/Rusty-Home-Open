# Live Supervisor Store availability preflight — product increment

**Code:** `afc1b6065771db60367d65357e7f83154f09b416`
**Installed-inventory reconciliation:** `260fac994745a1bc7e5a1fc5288ace689c3a4506`
**Scope:** universal, read-only platform availability for arbitrary Supervisor Apps.
**Production certification:** none. The 15 release gates remain unchanged.

## Native product behavior

- The dashboard offers **Vérifier disponibilité** for every App discovered
  through the official, authenticated Supervisor Store. No hardcoded vendor
  list, App list, home inventory or user-specific adapter.
- `GET /api/store/apps/{slug}/availability` fetches a fresh official Store
  catalog, installed inventory and Store detail through the fixed internal Supervisor service
  with its Supervisor-issued token (V1 or explicitly configured V2).
- The Rust preflight reconciles exact slug and repository membership, checks
  installed/system-managed state against the official installed inventory,
  and requires both Store responses to
  explicitly agree that an uninstalled App is available.
- The response contains only a typed status (`available`, `unavailable`,
  `already_installed`, `system_managed`, `unknown`), safe identifiers,
  read-only metadata and `activation_authorized:false`.
- Missing App returns 404; malformed slug 400; ambiguous/non-live source,
  unavailable API or conflicting details 503. No snapshot fallback,
  credentials, raw options, arbitrary host selection or mutation endpoint.
- This **does not** prove an App can actually install or authorize installation.
  It only reports the live Supervisor platform availability signal.

## Tests and limits

- TDD reproduced compiler error E0432 before implementation.
- Seven native adversarial preflight tests cover conflicting Store metadata,
  untrusted owner, missing or mismatched repository, already-installed,
  system-managed, unknown availability, and redaction.
- Two control-plane tests cover HTTP route grammar, traversal attempts,
  method rejection and the live-source-only boundary.
- The Rust workspace all-target tests, strict Clippy, rustfmt, diff checks and
  dashboard JavaScript syntax passed locally in isolated LXC700 DEV on the
  code tree before documentation was committed.
- Independent GitHub Actions must be checked on the **final PR SHA**.
  Existing HAOS QEMU tests do not yet drive this exact new preflight route.

## Next product code

1. Establish trusted user identity, per-operation RBAC and revocation across
   the real Supervisor Ingress boundary (not browser-supplied roles).
2. Implement durable, idempotent operation receipts linking a requested
   install/start/stop/update to the Supervisor job and postcondition.
3. Only then enable lifecycle mutations, with timeout, restart reconciliation,
   recovery and independent disposable-HAOS end-to-end tests.
4. Add rollback/backup semantics and broader App compatibility without
   vendor allowlists or any access to a real household.
