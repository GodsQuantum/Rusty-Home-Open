# Persistent official Supervisor job polling (experimental read-only module)

The `durable_job_watch` component can now resume a previously registered observation using only authenticated official Supervisor GETs, without relying on caller-constructed JSON job or App inventory snapshots.

- `poll_app_supervisor`: genuine `AppSupervisorClient` via Supervisor's fixed internal service.
- `poll_local_supervisor`: strictly loopback-only test adapter; live HTTP with a fake Supervisor token in isolated fixtures.
- Query `/jobs/{id}` (or `/v2/jobs/{id}`) first. Query `/addons` (or `/v2/apps`) only after a successful job response. Both independently validate IDs, schema and App state.
- No GET for unregistered or invalid identifiers; immutably preserve terminal outcomes. Reject clock rollback before a network request.
- Failed, unauthenticated, malformed, unavailable or mismatched official reads return a distinct error and **do not update** persistent observations. A successful HTTP response or 100% progress alone is not success.
- Verified App postconditions and completed jobs are persisted using the existing redb two-phase commit.

The loopback HTTP integration tests exercise restart, nonterminal jobs, V1/V2 paths, nonmatching installed state, success, malicious/mismatched UUIDs, unauthorized inventory reads and missing-job/clock preflight.

**Security boundary:** The watch's actor label is still untrusted. These methods do not prove that a Supervisor job originated from an audited App mutation, do not implement Core authorization, and do not expose an end-user HTTP route. Before shipping a real App mutation feature, persist a trusted one-time Core-authorized submission-to-job association, implement expiry/revocation and ambiguity recovery, and prove backup and rollback. No production system or household HAOS was changed. 0/15 production gates certified.
