# P19 — Bounded lab HTTP workers and request deadline (22 October 2026)

## Reproducible failure before fix

The read-only control-plane HTTP server, bound exclusively to 127.0.0.1,
handled requests sequentially. Six intentionally incomplete HTTP headers
blocked the entire listener: a valid GET /health timed out after 1.5 seconds,
although Rust was still alive. This is a genuine availability defect in
the localhost-only LAB listener, not evidence of a public remote exploit.

The original code also reset its 2-second read timeout on each read.
A slow byte-dripping client could keep the connection occupied without
an absolute request deadline.

## Implemented mitigation

The existing API remains loopback-only and read-only. An accept thread
dispatches via a sync_channel with exactly 32 pending slots to eight fixed
worker threads. Overloaded connections promptly receive JSON HTTP 503
(with Cache-Control no-store), instead of creating a new worker or blocking
the accept loop. Each worker releases the receiver mutex before executing
the potentially slow handler. An absolute monotonic 2-second deadline
covers the entire HTTP header irrespective of inter-byte timing.

The existing 4096-byte header cap, DNS-rebinding Host/Origin validation,
HTTP 400/431 responses, per-request signed inventory attestation,
Core sidecar 503 fail-closed behavior and no mutating API paths remain.

## Genuine red/green reproducibility

The test script scripts/test_p19_bounded_http_workers.py starts the actual
compiled Rust binary on a random private loopback port. It holds six
partial requests open while checking latency of an independent GET /health.
Before this fix the real test failed TimeoutError. With the fix it passes.
It additionally drip-feeds bytes, creates 70 incomplete connections, checks
immediate 503 overload denial, cleanup/recovery and <=12 Linux threads.
The test child is always stopped and reaped. All data is synthetic.

Independent GitHub rust-lab CI runs the test on each PR/main. Genuine
Rust unit tests, strict Clippy and P16 adversarial parser tests must pass
on the exact commit before accepting this change.

## Explicit limitations

This fixes one laboratory denial-of-service class. It is not completed
production authentication, authenticated host transport, RBAC, hostile WAN
fuzzing, an application penetration test, a 24-hour soak, real HACS/App
parity, or a full Home Assistant substitute. Security_stress_fuzz and
production_ui_auth gates remain blocked; never publish or reverse-proxy
the unauthenticated lab HTTP service. No HAOS synthetic VM, household data,
radios, devices, credentials, production LXC service or NFS mounts changed.
