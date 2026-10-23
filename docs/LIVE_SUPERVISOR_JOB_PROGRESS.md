# Live Supervisor Job progress — product increment (23 October 2026)

## Implemented
- Rust read-only `GET /api/jobs/{32-lowercase-hex-uuid}` route, on the
  existing authenticated Supervisor App client and the fixed official
  Supervisor V1 `/jobs/{id}` or opt-in V2 `/v2/jobs/{id}` API.
- Accepts exactly one live Supervisor adapter; rejects ambiguous versions,
  missing Supervisor tokens, invalid identifiers and malformed responses.
- Strictly redacted JSON: job ID, state, numeric progress and error count.
  No raw stage, error messages, child jobs, credentials or arbitrary URLs.
- French dashboard with manual job UUID entry, read-only status and bounded
  3-second polling while queued/running. It cannot start, stop, install,
  update or cancel any App or job.
- Adversarial Rust route tests, redaction tests, and authenticated real
  TCP loopback tests of the official V1/V2 paths with synthetic responses.

## Limits
This is useful read-only job progress, **not** an operation receipt.
An arbitrary job UUID does not bind a job to a user, App, operation or
trusted request. The current Ingress claim is not a verified admin role.
The local loopback mode is a development lab; the browser job view is not
approved for general production exposure without end-user RBAC.
No genuine HAOS job or App mutation was exercised for this feature.
All 15 release gates remain noncertified.

## Next product code
1. Verify the current Ingress user against genuine Home Assistant Core
   identity/permissions, fail closed on revoked/inactive/ambiguous users.
2. Persist cryptographically bound operation receipts, including user,
   App slug, action, precondition and Supervisor job UUID, with crash-safe
   recovery and a no-duplicate execution policy.
3. Bind job polling and reconciliation to those receipts rather than an
   arbitrary browser-supplied job UUID. Then add guarded lifecycle writes
   and disposable-HAOS end-to-end failure/restart tests.

Evidence: [PR #45](https://github.com/GodsQuantum/Rusty-Home-Open/pull/45), initial code commit `99f524b0ffc1285d9a631f08bef5a45ad9f09cbc`; independent CI on the final PR head remains mandatory.
Never change `config/release-gates.json` based on this lab milestone.
