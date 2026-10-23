# Universal Supervisor App release notes (read-only product increment)

Branch: `feat/supervisor-app-logs-readonly-20261023` (name is historical; the feature is **changelog**, not private logs).
Status: code implemented in DEV; independent GitHub CI required before merge.

## Product behavior

- For **any currently installed App slug** reported by the official authenticated
  Supervisor inventory, `GET /api/apps/{slug}/changelog` fetches its release
  notes from the official Supervisor endpoint.
- V1: `GET /addons/{slug}/changelog`; V2: `GET /v2/store/apps/{slug}/changelog`
  (the latter requires the Supervisor V2 API feature flag).
- The Rust backend validates the slug and current installed inventory, then
  issues a bounded authenticated read from the fixed internal Supervisor
  service. It rejects ambiguous sources, uninstalled Apps, unauthorized
  responses, redirects, missing length, non-plain-text content, invalid UTF-8,
  control characters and bodies larger than 64 KiB.
- The dashboard loads notes **only on user click**, with `textContent` rather
  than HTML or Markdown rendering. Third-party markup cannot execute scripts.
- No fixed App names, no lifecycle mutations, no secret fields, no logs or
  credential exposure. This is a read-only aid for evaluating updates.

## Evidence and limitations

TDD initially failed with E0432/E0599 before implementation. Three loopback
HTTP transport tests and one control-plane route test passed after correction.
The Rust workspace, Clippy and dashboard smoke checks must pass before PR.
The browser regression uses only synthetic release notes; it is not a real
Supervisor authentication or full HAOS end-to-end proof.

The transport currently requires an explicit Content-Length and rejects
chunked text responses. Such responses remain unavailable rather than
risking an unbounded stream. No deployment is authorized. No production
release gate changes state.

Next: trusted Core-verified per-request RBAC and durable user/App/action/job
receipts; then guarded install/start/stop/update, cancellation/retry and
restart-safe reconciliation in disposable HAOS KVM.
