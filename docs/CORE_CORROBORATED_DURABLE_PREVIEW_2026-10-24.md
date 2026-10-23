# Core-corroborated durable authorization preview (laboratory only)

This increment composes independently checked prerequisites for a prospective
App write permit rather than accepting self-described administrator status.

The new Rust core-corroborated-preview-lab feature is OFF by default and
imports the existing fixed-target authenticated Core WebSocket,
exact Supervisor Ingress TCP peer inspection, private redb durable fixture
sessions, per-App RBAC, single-use approvals and fsynced hash-chain audit.

An App-specific preview permit is possible only after:

1. Accepted TCP socket belongs to Supervisor Ingress proxy (172.30.32.2)
   and has precisely one valid user-id claim.
2. Supervisor's fixed internal WebSocket Core endpoint authenticates, returns
   correlated current Core config/auth/list response, and user is active,
   human and owner/admin.
3. Active private durable fixture session belongs to exact Core subject and
   is explicitly Administrator, not another user or stale session.
4. Per-App/action policy accepts actor, single-use confirmation matches
   exact operation, and owner-private journal records decision synchronously.

Tests show blocked revoked/demoted/system accounts, wrong subjects,
wrong App and action, replay after reopening DB, expiry, session revocation
and forged forwarded peer headers. Feature is absent from default build.

Remaining blockers: fixture session issuance is not production authentication.
Caller supplies time; release requires trustworthy clock, gateway/CSRF controls,
durable actual user-grant management, trusted audit anchors, job correlation,
backup/rollback verification and independent signoff. No default POST route,
no production App mutations; 0/15 certified.
