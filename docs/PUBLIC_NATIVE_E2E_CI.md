# Independent native Rust process-level CI

These six additional tests exercise real separately launched Rust
executables with fake fixture data on an isolated Linux host:

1. Bearer-authenticated loopback native daemon state and durable restart.
2. Real SIGKILL during a redb command: dedupe and no unsafe auto-replay.
3. Restart recovery: explicitly quarantine ambiguous effects.
4. SIGSTOP/CONT epoch fencing of a stale synthetic writer.
5. Offline HA-style MQTT sensor discovery with invalid input rejection.
6. Modern and legacy HA automation YAML parser dry runs with denial
   of templates, outside paths and mutating commands.

The GitHub Actions workflow always runs them on standard ubuntu-24.04,
without USB, Zigbee/Thread/Matter fabrics or household accounts.

These are **narrow laboratory claims**. They do not certify real devices,
App lifecycle parity, production authentication or migration. Every
release gate remains blocked pending independent end-to-end evidence.
