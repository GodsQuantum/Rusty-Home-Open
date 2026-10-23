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
