# Roadmap

1. Certify native Rust persistence, security and transport.
2. Validate actual custom integration lifecycle compatibility individually.
3. Prove App packaging, recovery, backup and rollback independently.
4. Qualify duplicate radio controller prevention on actual hardware.
5. Validate full-stack performance, fuzz, packaging and upstream licenses.

All 15 production gates remain blocked.

## P27: Transition HACS -> Core Marketplace (2026.11 planned)

Use the official Marketplace as the single authoritative community package store rather than cloning HACS. [Architecture and source-backed ABI plan](MARKETPLACE_CORE_2026_11_ARCHITECTURE.md). Existing 29-integration runtime gate and OAuth gate remain lab_partial until actual full-life-cycle evidence.

## Universal product functions in development

The generic Supervisor Store App browsing API, including unknown and future repositories, is implemented with search, paging and CLI access. [Functional details](GENERIC_SUPERVISOR_APP_CATALOG.md). Real App mutation/job management and complete user-facing UI remain to be built **before** product certification.

<!-- Product-first development: generic installed App runtime inventory and status reconciliation are in progress. -->

The live Supervisor App operation observer now reads the official job and installed
inventory APIs together, with fail-closed postcondition reconciliation. It is
read-only and does not provide trusted operation receipts, user RBAC, lifecycle
mutations or durable restart recovery. See [scope and limitations](LIVE_SUPERVISOR_OPERATION_OBSERVATION.md).

## Live uninstalled Store App details

The universal Store dashboard now has a native, authenticated, read-only
details path for any catalog-listed App, including uninstalled Apps. See
[scope, limits and next increment](STORE_LIVE_DETAILS.md). This is product
code, not an end-to-end production certification.

## Live Store availability preflight

The native read-only API and dashboard now reconcile the current official
Supervisor Store catalogue with App details for arbitrary vendors. The result
never grants installation permissions. See [scope and test evidence](STORE_AVAILABILITY_PREFLIGHT.md).
Next: trusted user identity/RBAC, durable operation receipts, then guarded
install/start/stop/update with independent disposable-HAOS end-to-end tests.

## Live installed App resource metrics

The universal dashboard now queries authenticated official Supervisor
V1/V2 numeric stats for any installed App, with no provider list and
without exposing raw logs or options. See
[LIVE_SUPERVISOR_APP_STATS.md](LIVE_SUPERVISOR_APP_STATS.md).
Next: trusted Core-verified user RBAC, durable operation receipts,
idempotent lifecycle writes and restart reconciliation.


## Live Supervisor job progress (23 October 2026)

The native Rust read-only `/api/jobs/{uuid}` API and dashboard poll the
official authenticated Supervisor V1/V2 job endpoint for arbitrary operations,
with strict UUID validation, minimal redacted output and no write route.
See [functional scope and limits](LIVE_SUPERVISOR_JOB_PROGRESS.md).
Next: Core-verified Ingress user RBAC, durable job-to-App/user/action receipts,
then guarded mutations and restart-safe reconciliation. Job UUID possession
does not authorize any lifecycle action.
