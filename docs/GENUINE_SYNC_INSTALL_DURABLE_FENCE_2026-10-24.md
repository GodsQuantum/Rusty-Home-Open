# Genuine HAOS native synchronous installation: independent postflight and durable App fence

This test-only optional Supervisor Apps capability uses the officially documented Store POST /store/addons/{app}/install, default `background: false`. Unlike asynchronous installs, it returns only after the Supervisor has finished the action and requires no ephemeral job ID.

The private two-phase redb dispatch intent must be armed before the mutating POST. The exact scoped, audited permit and request cannot be replaced by browser-supplied actor, App, or Boolean. Following a successful authenticated POST, a second independent authenticated GET /addons must include precisely the installed arbitrary-vendor App, a nonempty version and a normal `started` or `stopped` state. Only then can the same protected Rust client atomically transition the dispatch history to `confirmed` with explicit `synchronous_transport_and_inventory` evidence and remove only its own App lock. The record persists across an actual redb close/reopen; a new separately audited operation can then arm. No synthetic job ID is ever manufactured.

Missing/malformed/different App inventory, an App in `error` state, a remote authentication or transport failure, storage error or uncertain POST response can NEVER release the lock or retry a mutation. Only one existing private code path can call the synchronous closer. The default Rusty Home build stays read-only and does not expose a mutating HTTP route.

A real isolated HAOS 18.3 QEMU test runs the native statically linked Rust binary from a protected manager-role test App and installs another, unprivileged local disposable App through the genuine Supervisor, proves its state by independent official CLI, then uninstalls the fixture. It is not a household installation.

This lab does NOT prove production Core end-user identity + CSRF, full third-party App backup/restore, arbitrary physical radios, multi-day load, signed release or independent security signoff. Release remains NOT_PRODUCTION_READY, 0/15 gates certified.

Official API: https://contributors.home-assistant.io/apps/supervisor-api/store/ . The install endpoint's `background` key defaults to `false`.
