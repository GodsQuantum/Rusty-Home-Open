# Continuous Rusty Home development and regression testing

**Public source of truth:** https://github.com/GodsQuantum/Rusty-Home-Open

- [V1 roadmap, exact 15 production gates](https://github.com/GodsQuantum/Rusty-Home-Open/issues/9)
- [Security and authenticated host transport](https://github.com/GodsQuantum/Rusty-Home-Open/issues/10)
- [29 HACS compatibility and OAuth](https://github.com/GodsQuantum/Rusty-Home-Open/issues/11)
- [Nine Apps, backup/restore and rollback](https://github.com/GodsQuantum/Rusty-Home-Open/issues/12)
- [Radio ownership, drivers and whole-stack performance](https://github.com/GodsQuantum/Rusty-Home-Open/issues/13)
- [Packaging, signed updates and licenses](https://github.com/GodsQuantum/Rusty-Home-Open/issues/14)
- [Evidence and gate-by-gate scorecard](CERTIFICATION_LEDGER.md)

## Independent automatic GitHub Actions

The six existing production-free CI workflows run once a week on standard
public-repository GitHub-hosted Ubuntu runners. They are staggered on
Monday in UTC; GitHub may start scheduled jobs later than the cron time.
They also keep their prior PR and push triggers:

| UTC | Workflow | Real laboratory scope |
| --- | --- | --- |
| 02:07 | ci.yml | Rust unit tests, Clippy, native crash/auth/security and synthetic scenarios |
| 02:17 | core-interop-public.yml | Genuine pinned Home Assistant Core and three HACS integrations |
| 02:27 | core-hacs-wave2.yml | Five additional real-Core HACS component lifecycles |
| 02:37 | public-debian-apt.yml | Real disposable Debian and signed APT with negative tampering |
| 02:47 | fuzz.yml | Sanitizer-instrumented native Core-user parser |
| 02:57 | p8c2-haos-qemu.yml | Official disposable HAOS KVM Supervisor backup/restore, 16/16 gate |

Dependabot proposes grouped **minor/patch** Rust and GitHub Actions
updates every Monday; it is limited to at most three open Rust PRs and
two GitHub Actions PRs. Dependabot cannot bypass branch protection or
authorize production deployment. Major updates remain explicit reviews.

## Development iteration contract

A separately configured ChatGPT recurring task is intended to pick up
work twice per day (approximately 08:00 and 20:00 Europe/Paris). Its
execution depends on service availability and authorized tool access;
it is **not** a permanently running agent, a guarantee of completed code
in every iteration or a substitute for repo CI. This repository and
the issue tracker retain all durable context between runs.

Each successful iteration should read open PRs/issues and the release
ledger, implement one real tested milestone, open a limited-scope PR,
wait for independent CI on the exact SHA, and merge only via normal
branch protections. Never bypass checks, delete independent failure
proofs or use real private household inventories.

### Unattended limits

No access to real radios/fabrics, actual HAOS VM100, live backups,
personal credentials, homelab mounts or production services. None of
these weekly jobs may change production \`config/release-gates.json\`.
Actual field migration or hardware tests require fresh explicit user
authorization and independent safety signoff. Current production
readiness is **NOT_PRODUCTION_READY (0/15 gates)**.
