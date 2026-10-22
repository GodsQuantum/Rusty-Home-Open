# HA Rust Supervisor Manager Probe (P8f laboratory only)

This self-contained Home Assistant Supervisor App is **not** a replacement Home Assistant service. It is a minimal disposable certification fixture for actual App installation and scoped `manager` API access.

On start, a static Rust executable reads its Supervisor-provisioned token in memory, calls only `GET http://supervisor/store` and emits sanitized app/repository counts, then exits. Never install on a production HAOS instance. There are no ports, devices, Docker API mounts, network host sharing, or real user credentials. All proofs run only inside the official disposable HAOS QEMU tests.
