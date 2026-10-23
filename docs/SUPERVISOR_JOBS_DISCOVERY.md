# Official Supervisor job discovery — read-only product increment

This increment exposes the **official** Supervisor GET /jobs/info (and
experimental V2 /v2/jobs/info) through a typed Rust transport and
GET /api/jobs. The dashboard can discover recent job identifiers and
select one for the existing GET /api/jobs/<id> progress tracker, without
requiring a user to paste an opaque UUID.

## Access and safety

- Discovery requires a verified TCP connection from the official Supervisor
  Ingress proxy **and** its effective system-admin group; local loopback,
  non-admin and anonymous clients are denied. This is a read-only access
  restriction, **not** production-grade RBAC or permission to submit actions.
- Only one live Supervisor adapter is accepted. Signed snapshots, Docker
  fixtures, ambiguous sources and arbitrary remote URLs cannot substitute
  for the official internal Supervisor service.
- The Supervisor App token stays server-side; the API never returns job names,
  references, stages, error messages, child metadata, extra, conditions or
  private configuration. Only UUID, state, progress and error count are
  exposed. The full upstream list is validated, and the newest 24 jobs are
  shown; more than 512 jobs or 1 MiB fails closed.
- No job identifier proves who initiated the operation. This index is **not**
  a durable operation receipt, authorization, completion proof or App
  ownership proof. Mutation routes remain absent and all release gates
  remain unchanged.

## Tests

- TDD: jobs_discovery.rs initially failed with E0432/E0599 for missing
  parser/transport, then passed after implementation.
- Real loopback TCP GET checks official route and Bearer authentication;
  malicious and duplicate job IDs, duplicate JSON keys, unsafe stages,
  oversized data and private metadata are rejected/redacted.
- Control-plane unit tests deny local, anonymous, non-admin and ambiguous
  sources before any network request.
- Workspace, Clippy, frontend syntax and independent PR CI must be recorded
  with the final SHA; no CI success is implied by this document.

Official reference: https://contributors.home-assistant.io/apps/supervisor-api/supervisor/#jobs

## Next product milestone

Introduce a durable, tamper-evident correlation of verified Core identity,
exact App and action, user consent, official Supervisor submission response
and job ID, with replay prevention and postcondition reconciliation. Keep
all mutation paths disabled until end-to-end RBAC, backup and rollback are
independently verified in disposable HAOS.
