# Home Assistant Core-authorized durable App dispatch (disposable lab)

The optional Rust feature `core-durable-dispatch-lab` composes three fail-closed steps: (1) check the actual accepted Supervisor Ingress TCP peer and current administrator status with official Home Assistant Core WebSocket, (2) consume exactly one App/action/parameter-bound consent from the private, durable fixture session ledger and synchronously audit the decision, and (3) persist a two-phase-committed App dispatch fence before any disposable-lab Supervisor POST could be attempted.

The public laboratory function `arm_corroborated_disposable_dispatch` returns an opaque, single-use `ArmedDispatch`, **not** a response indicating that the App operation succeeded. It performs no POST and creates no new production route. The separate QEMU-guarded transport still consumes that capability. Lost responses, a killed process and ambiguous operations remain fenced; no automatic retries or unlocking. The actor is captured in the audited permit rather than accepted as a later HTTP parameter.

Two isolated tests verify denial of revoked/non-admin Core identities, durable one-use consent across restart, duplicate App dispatch prevention, and fail-closed trusted clock overflow. The live Core/TCP verification already has separate authenticated-transport tests. Test fixtures do not replace production identity, end-user CSRF/origin enforcement, external review, backup/rollback or real hardware testing.

All 15 production gates remain uncertified. No family HAOS or peripheral device is used.
