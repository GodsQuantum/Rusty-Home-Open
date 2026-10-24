# Official HAOS asynchronous App install: verified release or safe unresolved result

This opt-in test runs only in a pinned genuine Home Assistant OS 18.3 QEMU/KVM guest. It does NOT deploy to a household HAOS device.

## What the test genuinely exercises

- Build and execute a static musl Rust binary inside a protected, real manager-role Supervisor App, using a real guest-provisioned Supervisor token and a private, persistent test audit directory.
- Discover a disposable, locally authored App through the genuine Supervisor Store, issue an authorized and synchronously audited generic background Install request with exactly **one** HTTP POST and persist the App-scoped redb fence before the POST.
- Require a valid job ID returned from that same POST, and authenticate independent official job detail reads, a fallback GET /jobs/info index read for the exact same job, plus the separate installed-App state.

## Two distinguishable outcomes

**Positive verified closure**: If the official job endpoint or exact matching authenticated index independently proves terminal success, and the official installed-App inventory proves the correct App, Rust atomically records Confirmed and releases only the matching App fence. The QEMU harness then prints `P8N_GENUINE_HAOS_ASYNC_INSTALL_AND_DURABLE_UNLOCK_PASS`. Only this marker counts as real job completion and durable-unlock proof.

**Safe unresolved cleanup**: Supervisor jobs with `cleanup=True` can disappear from BOTH official job APIs immediately after finishing; this was observed three times in actual HAOS 18.3 tests (PR #80). For this condition the test explicitly requires a previously observed genuine Running job, two missing authoritative job reads, unchanged Accepted redb record across a database reopen, and rejection of a second independently authorized dispatch to the same App. The QEMU harness additionally checks the actual installed App from Supervisor. It then prints `P8N_GENUINE_HAOS_JOB_EVICTED_SAFE_FENCE_PASS` and `P8N_POSITIVE_TERMINAL_JOB_PROOF_NOT_CERTIFIED`. **This is a safety-negative proof, NOT successful unlock, full lifecycle proof, or production certification.** No missing-job heuristic releases a lock.

All unrelated/missing jobs, failed authenticated reads, malformed responses, changed actor, premature completion, and fake cached watch receipts are rejected. The one POST is never retried automatically. The disposable App is uninstalled and its test assets removed.

The distinction is deliberate: a green CI run with the safe-unresolved marker validates fail-closed behavior but does NOT satisfy the production gate for automatic App lifecycle and recovery. Production release remains **NOT_PRODUCTION_READY, 0/15 gates**.

Official 2026 docs: https://contributors.home-assistant.io/os/supervisor/jobs/ (jobs with cleanup=False remain after completion); https://contributors.home-assistant.io/apps/supervisor-api/supervisor/ (GET /jobs/info, GET /jobs/id).
