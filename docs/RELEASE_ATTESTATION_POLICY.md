# Rusty Home — production release certification attestation

Status: NOT_PRODUCTION_READY, 0 / 15 gates certified.

A local docs/certifications/GATE.json file plus its SHA-256 is not
independent evidence. The original check accepted an arbitrary JSON payload
such as {"certified": true} with its author's own hash. This false
positive has been fixed and regression tested.

## Required production-grade gate evidence

- Every one of the 15 production gates has a mandatory set of acceptance
  scenarios hard-coded in the verifier; no arbitrary 'three tests passed'
  substitute is accepted.
- Strict evidence JSON (duplicate keys rejected), matching gate ID, explicit
  production_end_to_end scope, pinned product commit, tested scenarios,
  at least two distinct independent CI run identifiers, and recovery verified.
- Manifest SHA-256 of the exact evidence bytes.
- Two independent active reviewers must sign the exact same evidence bytes
  with Ed25519; OpenSSL verifies both signatures.
- Trust registry in config/release-attestors.json is intentionally EMPTY.
  No private release keys or locally self-signed approvals are accepted.
- Before keys are added, independent maintainers must review/pin the public
  keys, externally verify CI and test scope, and confirm evidence covers
  the COMPLETE criterion. A signature itself is not proof of factual truth.
- Release approval remains separate: signing proof cannot authorize a
  deployment, home migration, license compliance or hardware takeover.

DO NOT generate release reviewer keys automatically from a CI workflow.
Synthetic ephemeral keys are allowed in negative/positive verifier tests
only, never as project review authority. The actual product is locked.
