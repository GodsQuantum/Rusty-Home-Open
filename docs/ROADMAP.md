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
