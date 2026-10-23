# Core-verified read-only Ingress session

This product increment adds `GET /api/session/core` only when Rusty Home
runs behind the official Supervisor Ingress TCP proxy. The backend first
validates the actual accepted TCP peer and a single Ingress user ID, then
reads `config/auth/list` from authenticated Core WebSocket via the fixed
internal Supervisor proxy. Only active, non-system Core users are reported.
The response exposes a boolean admin status, no user IDs, tokens or groups.

The existing `/api/session` remains a **Supervisor group claim**, not an
independent Core authorization. The UI distinguishes the two checks and
fails closed when Core cannot be reached or refuses the list command.

This is a genuine read-only feature, not a write authorization provider:
`actions_authorized=false` always. No App POST, consent, idempotency,
receipt, CSRF-protected mutation, persistent grants or production release
gate is added. Some Supervisor/Core versions may deny `config/auth/list`;
in that case the new route returns 503 and the UI shows unverified status.

Next: independent HAOS KVM end-to-end proof for an active and revoked Core
user, followed by durable, per-App and per-action grants and operation
receipts before implementing Supervisor lifecycle mutations.


The disposable HAOS KVM regression additionally requests this route directly
from the private App network with a forged user header, both before and
after a real Supervisor daemon restart. Both requests must return 403
without ever reaching Core. This is a negative boundary test, **not** a
positive Core role/revocation end-to-end certification.
