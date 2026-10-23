# Typed Supervisor write receipts (disposable-HAOS preview only)

The existing feature-gated authenticated Supervisor lifecycle transport returned
the entire arbitrary upstream data JSON from the POST response, including
potential passwords, App options, device paths and authentication credentials.
That is incompatible with a future product control plane.

This change replaces it with a finite Rust enum:

- Accepted: validated App slug and action, completion_verified=false.
  An HTTP 200/result=ok proves submission only. Separately trusted Supervisor
  job ID and independently verified installed state are required for completion.
- BackupCreated: only a syntactically verified Supervisor backup slug.
  Archive integrity and a successful restore remain unproven.
- Update: completion_verified can be true only after the authenticated
  installed-version and update-flag postconditions have succeeded.

Unknown fields are dropped rather than serialized or logged. Third-party App
payloads are never returned. Invalid backup slugs fail closed. Three unit tests
cover lifecycle actions, secret stripping, path/header injection, malformed
backup IDs, and no false success flags.

This does not add App mutations to the default release. A production
implementation still requires trusted Core identity and revocation checks,
durable RBAC, CSRF/replay-resistant user consent, audit, correct
operation/job correlation and rollback/restore tests.

No household HAOS deployment or real App was modified.
