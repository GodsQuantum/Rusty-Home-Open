# Verified completion and durable release of a Supervisor install dispatch

An experimental pre-dispatch redb fence prevents duplicate mutating Supervisor requests after a crash, but historically never cleared an App lock even after a confirmed background installation. This change allows only an independently corroborated install to complete.

## Protocol

1. Reopen the private dispatch record and require the original accepted Install, audited actor and Supervisor-issued background job ID.
2. Register or resume the exact read-only durable job watch, retaining the source App, actor, action and timestamp.
3. ALWAYS issue a fresh authenticated official GET /jobs/{id}, even if a generic read-only watch already carries a cached terminal Confirmed value supplied by a caller. If (and only if) the individual job GET returns an unexpected HTTP status, independently query authenticated official GET /jobs/info: a strictly validated, unique, exact-match original job UUID with a successful terminal status can serve as the job confirmation. An unrelated, truncated-away, missing, malformed, still-running or failed index job NEVER allows release. Only after that validated job completion, query fresh official installed-App inventory and require the exact matching installed App postcondition. Never allow a cached watch alone to authorize unlocking. V2 routes are supported only where the Supervisor enables V2.
4. Require a terminal successful job AND the matching installed-App postcondition. Transition the immutable operation history to confirmed and remove only the exact matching active App lock atomically via a two-phase redb transaction.
5. Reopening and repeating a confirmed release is idempotent, including after a process restart. Late transport results cannot regress a confirmed operation or affect a newer lock.

Unauthorized/malformed/absent remote data, unresolved POST, job still running, failed job, app missing from inventory, actor/job/App mismatch or clock rollback cannot unlock the App. No POST is automatically retried.

This operation is deliberately limited to background INSTALL. Synchronous operations, updates, backup/restores, missing job receipts and unknown outcomes remain unresolved until distinct independent postcondition and rollback certification. Owner-private lab databases are not a complete production identity provider.

The default product still has no Supervisor lifecycle write HTTP route. No household HAOS system was modified, and the production release remains NOT_PRODUCTION_READY, 0/15 gates certified.

Official API: https://contributors.home-assistant.io/apps/supervisor-api/supervisor/ ; https://contributors.home-assistant.io/apps/supervisor-api/store/
