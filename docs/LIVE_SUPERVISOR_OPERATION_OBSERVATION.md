# Live Supervisor App operation observation (read-only)

This increment connects the existing generic operation reconciler to **actual
authenticated Supervisor HTTP GET calls**. It is not an App installer, an
authorization service or a production-certified lifecycle manager.

`LocalSupervisorClient::observe_operation` is limited to a loopback fixture.
`AppSupervisorClient::observe_operation` uses only the fixed internal
`supervisor:80` service and its Supervisor-provided token. Both accept any
validated App slug and V1/V2 API version, without vendor allowlists.

For a previously trusted `Watch`, the observer fetches the official job UUID.
Queued/running/failed jobs return their status without fetching inventory.
Only after a successful job does it fetch the **current installed App inventory**
from `/addons` or `/v2/apps` and apply the operation postcondition.
A 401/403, malformed/mismatched job, missing inventory, wrong API version or
network failure is an error, **not** a success or an empty installed inventory.
The observer never POSTs, retries mutations or exposes credentials.

Adversarial integration tests in
`crates/supervisor-apps/tests/live_operation_observation.rs` exercise real
loopback TCP with bearer headers, V1/V2 paths, an unknown future vendor,
pending/failed job, mismatched job UUID, unauthorized inventory and a stale
postcondition. They do not connect to a real household or real HAOS.

## Unfinished product work

The caller still needs a trusted identity and role, a persisted audit-backed
operation receipt binding the job UUID to the App slug, explicit authorization,
durable polling/retry across restarts, lifecycle mutation with preflight and
verified rollback, and a user-facing management UI. A successful unrelated
job must never be associated with an arbitrary App Watch. No production gate
changes until end-to-end independent qualification and signoff.
