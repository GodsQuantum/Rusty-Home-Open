# Universal Supervisor Store App documentation (read-only)

Rusty Home now discovers DOCS.md from any official Supervisor Store App
repository, including community providers not previously known to this product.
It calls the documented GET /store/addons/{slug}/documentation path or
the explicitly configured experimental V2 equivalent, never arbitrary URLs.

Before loading documentation the Rust server authenticates the official Store
and verifies that exactly one catalog entry owns the requested slug.
The native transport reuses its bounded 64 KiB plain-text parser, rejects
invalid UTF-8 or unsafe control characters, and never interprets Markdown,
HTML, scripts, embedded links or executable tags. The user sees inert text
only when pressing the per-App documentation button in the Store.

There is no new App write operation or install permission. The compatibility
work only provides actual dynamic documentation discovery in the marketplace.
It is not proof of full App lifecycle authorization, migration or readiness.
Official API: https://contributors.home-assistant.io/apps/supervisor-api/store/
