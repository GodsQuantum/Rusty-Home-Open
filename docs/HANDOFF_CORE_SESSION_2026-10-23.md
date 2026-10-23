# Rusty Home — Core session product increment handoff

Repository: GodsQuantum/Rusty-Home-Open (public). Worktree isolated in
LXC700 DEV, branch `feat/core-verified-session-readonly-20261023`.
The historic private repository must remain untouched.

Implemented: native `GET /api/session/core`, official fixed-target Core
WebSocket read, verified Ingress TCP peer, fresh Core active/owner/admin
checks, fail-closed 403/503, no personal IDs in HTTP, dashboard Core
status and adversarial tests. The old `/api/session` remains informational
Supervisor group evidence. No POST, no App mutation, no production access.

Local verification: initial RED E0425/E0433, then Rust workspace all-target
all-features PASS, Clippy -D warnings PASS, rustfmt PASS, JS syntax PASS,
public redaction PASS, existing Supervisor group contract PASS.
Local Playwright screenshot not executed: Playwright package/Chrome absent
in this isolated worktree; GitHub Actions supplies both. Independent CI
and HAOS KVM proof must be checked on the exact final PR head.

Concurrent work: PR #56 official Supervisor job discovery pending HAOS KVM;
PR #34 old handoff. Do not discard or overwrite their branches/worktrees.

Next code: trusted persistent per-App operation receipts with one-time
consent, explicit RBAC, idempotency, official job reconciliation and crash
recovery. Only then lifecycle writes in disposable HAOS CI. All 15
production gates remain noncertified; no VM100 or live household access.
