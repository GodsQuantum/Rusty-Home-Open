# Generic Supervisor App catalog — functional implementation

Rusty Home now supports **read-only listing, searching and pagination** over
the catalog returned by the official Home Assistant Supervisor Store API,
including unknown or future Apps and any third-party repository. **There is
no fixed list of nine Apps**, and those sample Apps remain test fixtures only.

## Rust API

- `LocalSupervisorClient::fetch_store(ApiVersion::V1)` obtains the official
  `/store` response from a locally authenticated fixture/client.
- `AppSupervisorClient::fetch_store(...)` uses the official Supervisor
  internal service in the dedicated App runtime.
- `Catalog::filter_apps(Filter { ... })` filters a fetched snapshot
  on repository slug, Unicode-aware case-insensitive search, installed
  state, and pending updates. Results are slug-ordered and paginated.
- Pages contain total, entries and next_offset. Their size is bounded at
  200 items to protect the frontend; this does **not** limit the number
  of Apps that can be discovered.
- Existing Supervisor API V1 `addons` and V2 `apps` result structures
  are accepted by the same parser. Duplicate/malformed identifiers fail.

## Offline command-line demonstration

Pass a **synthetic or authorized** Supervisor response on standard input.
The command never contacts a household directly:

```shell
cat synthetic-supervisor-store.json |
  cargo run --locked -p hars-supervisor-apps --example parse_official_store -- \
    --repository third_party_repository --installed true --updates-only \
    --limit 50 --offset 0
```

The returned JSON contains `total`, `apps`, `next_offset` and
`activation_authorized: false`. Without flags, the tool retains its
previous behavior of returning the full bounded read-only catalog.
Unexpected flags, invalid paging and unsafe names are rejected.

Independent product-level regression coverage includes 1,317 arbitrary
Apps across 13 providers via the CLI, 1,373 Apps across 11 providers via
the Rust API, and 1,211 Apps via an authenticated local mock Supervisor
HTTP client. This is **not** an allowlist of supported Apps.

## Remaining work before a finished product

Browsing does not provide complete installation/stop/update/restore
functionality. Those require authenticated operator permissions,
the full Supervisor job lifecycle, App data recovery and error handling,
and universal frontend wiring. Those are **unfinished product features**,
not post-development certification tasks.

See `crates/supervisor-apps/src/catalog_filter.rs` and
`scripts/test_generic_supervisor_catalog_cli.py`.

## Display policy

The experimental web dashboard now shows **all discovered Docker and
Supervisor entries by default**, with source filters. It no longer guesses
App kinds from a fixed list of product/brand names, or hides unknown
containers under a "recognized" filter. The backend reports generic
platform-specific classifications, not per-user known-App categories.

The experimental frontend no longer presents the nine synthetic App
fixtures as a product-wide readiness score. Fixture counts are for test
coverage only; readiness remains distinct from app compatibility.


## Arbitrary App detail retrieval (product increment)

Supervisor 2026.10.3 defines the official read-only endpoints
`GET /addons/{slug}/info` (V1) and
`GET /v2/apps/{slug}/info` (V2). Rusty Home now exposes
`LocalSupervisorClient::fetch_app_detail` and
`AppSupervisorClient::fetch_app_detail` over existing authenticated,
bounded, read-only Supervisor connections. Any valid App slug can be
queried; no provider names, App brands or installation inventories
are compiled into the client.

Returned `AppDetail` contains a typed, bounded set of safe metadata:
slug, name, repository, current/latest version, state, availability,
update status, system-managed state and ingress capability. It never
copies raw options, schemas, API credentials, host addresses or
device paths from Supervisor's confidential response. Invalid slugs,
a mismatched response, repeated JSON keys or 401/403 fail closed.

Verified in actual authenticated loopback TCP tests with unknown App
slugs under both V1 and V2, including an unauthorized request.
Real HAOS production credentials and data were not touched.

**Not yet implemented:** production user RBAC, arbitrary App installation,
start/stop/update actions with job tracking, configuration mutation,
complete recovery or a connected end-user frontend for App detail.

The read-only dashboard now supports live name/host/image search and
source filters without re-fetching Supervisor data, plus 24 cards per
page with previous/next navigation and deterministic name ordering.
Failed refreshes invalidate the previous in-memory catalog. An isolated
Chromium regression test injects 83 unknown Apps and validates safe DOM
text rendering, pagination and filtering.

## Actual installed App runtime inventory

In addition to browsing the Store, Rusty Home reads the **installed Apps**
from the official Supervisor endpoints `GET /addons` (V1) and
`GET /v2/apps` (V2), retaining all third-party providers and Apps even
when their Store repository is no longer listed. The native Rust inventory
normalizes name, slug, repository, state, version and update availability,
without serializing options, network addresses or credentials.

`LocalSupervisorClient::fetch_installed` and
`AppSupervisorClient::fetch_installed` return a typed, read-only
`RunningInventory`. `diff_running` reports install, removal, state,
version and update-availability changes between complete snapshots.
An offline/unresponsive Supervisor returns an **error**, never an invented
empty inventory. Work on generic operation execution and job tracking remains.

## Read-only Supervisor lifecycle job progress

The generic Supervisor clients now inspect any Job UUID through the official
GET /jobs/{uuid} and GET /v2/jobs/{uuid} endpoints. Typed progress tracking
distinguishes queued, active, succeeded and failed operations, including
errors from child jobs. A displayed progress of 100% alone never means success;
Supervisor must report done=true with no recorded errors. Confidential error
messages, host references and raw extra fields are not copied into the UI
model. IDs are validated before any HTTP request is made.

This is a functioning backend component for future install/start/stop/update
workflows. Operator authorization, action submission, job persistence,
reconnection and rollback remain unfinished product features.

## Completion reconciliation for any Supervisor App lifecycle action

The default Rust build now includes `operation_reconcile`, which combines
**two independent official observations**: the correlated Supervisor job
result and a complete post-operation installed-App snapshot. It covers
install, start, stop, restart and update for any App slug or repository,
without name-based classification.

A job is not marked verified merely because its HTTP response was OK,
its progress was 100%, or its state was `done`. The matching job UUID must
have succeeded **and** the required App postcondition must be observed.
For an update the prior version must be known and the new version must
differ, with no update pending. A missing snapshot, stopped service, stale
version or mismatched job UUID returns a pending/unverified result, not
a fabricated success.

The component is read-only reconciliation. User authentication, authorization,
real Supervisor writes and durable recovery are separate unfinished product
functions. The tests cover 903 previously unknown App identifiers and
adversarial mismatched/failed/unavailable observations.

## Live installed-App inventory in the dashboard (product feature)

Inside an authorized Supervisor-managed App container, Rusty Home can
read the actual installed-App inventory from the official Supervisor
API rather than relying on a fixed JSON snapshot.

Use the native control-plane with --supervisor-live --host home
--serve 127.0.0.1:18790 (official V1), or --supervisor-live-v2 for
the explicitly opted-in V2 API. The official SUPERVISOR_TOKEN is
provided by Supervisor to the container, and the service hostname
is fixed to supervisor:80. No caller-supplied URL is accepted.
The dashboard stays loopback-only and read-only; responses are typed
and secret-redacted, with no vendor allowlist.

When the token or endpoint is unavailable, the source is marked
degraded, never represented as a verified empty inventory.
This does not yet submit privileged App lifecycle operations.
Trusted user authentication and management UI remain incomplete.

CI covers an actual loopback HTTP denial with no token, plus
1,403 arbitrary unknown App fixtures in the Rust formatter test.

## Authenticated Home Assistant ingress (read-only)

Rusty Home can be exposed **only through official Supervisor ingress**
using the new --ingress option, with --supervisor-live and an HTTP bind
such as 0.0.0.0:8099 inside the App container. The published App
configuration must enable official ingress with the matching ingress_port.
This does NOT authorize internet exposure or host-level port forwarding.

The server rejects every TCP peer except the official Supervisor ingress
proxy 172.30.32.2. It requires the Supervisor-injected X-Remote-User-Id
and rejects duplicate identity headers, forged local requests, cross-site
Origin/Fetch-Site headers and mutating HTTP methods. No header alone can
grant a role or permission to manage Apps. The authenticated read-only
dashboard uses the same typed, secret-redacting Supervisor backend and
continues to mark source failures as degraded.

This ingress path is tested against forged admin headers on actual loopback
TCP, and against a trusted source-address simulation in Rust unit tests.
It is not yet certified on a deployed official HAOS App, and contains
no production user RBAC, app write routes or backup-and-restore controls.

Official design guidance:
https://developers.home-assistant.io/docs/add-ons/presentation#ingress
https://contributors.home-assistant.io/apps/guides/ingress/
