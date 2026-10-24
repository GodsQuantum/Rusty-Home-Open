# Durable read-only Supervisor job watches (Rust)

The native `hars-supervisor-apps::durable_job_watch` module persists bounded,
provider-independent job observations to a local redb database. A restart can
resume tracking a job without losing its App slug, operation, prior version,
caller label or last observed outcome. This is a **product component**, not
production certification or authorization to execute App lifecycle operations.

The ledger validates every App slug and Supervisor job ID using the existing
shared reconciler. A registered job ID cannot be silently rebound to a different
App, operation, prior version or actor label. Updates use two-phase ACID commits,
reject clock regressions, preserve terminal results across outages and never
claim a postcondition unless the existing read-only reconciler observed it.
Capacity is limited to 1,024 watches; the parent must be owner-only 0700 and
the database a regular owner-only 0600 file, with symlinks/hard links rejected.

**Security boundary:** The caller-supplied actor label is **not authenticated**;
no HTTP handler currently exposes these receipts, and a job UUID is not proof
that the job actually targeted the claimed App. `postcondition_observed` is
only a state observation, never proof of authorized execution or a trusted
job-to-App correlation. No Supervisor write is enabled. There is no user-controlled
App allowlist and no changes to any real HAOS installation.

**Next product code:** Wire this ledger to an authenticated Core-admin Ingress
watch API, replace caller-provided labels with independently verified Core
subjects, obtain and persist a trusted job-to-App binding from the *actual*
Supervisor operation submission, add per-App RBAC and one-time consent, then
allow install/start/stop/update with recovery and verified backup/rollback.
Until those steps are complete, do not call these records execution receipts.
