# P8ab: native opaque full-backup escrow and independent disaster recovery

**Release status: NOT_PRODUCTION_READY, 0/15 production gates.**

## New capability vs historical work

- Earlier P8p proves HAOS->fresh HAOS directly (6/6); P8aa proves real HAOS full archive->native Rust **entity registry only**. Do not rerun/relabel either as P8ab.
- New Rust module backup_escrow holds the ENTIRE HAOS archive, including optional encrypted components, without decrypting it. Content SHA256 verified on ingestion and export. It uses exclusively owner-private 0700 directories, nofollow file descriptors, one hard link, private 0600 sources, bounded streaming (1GiB CI cap), atomic no-overwrite tempfile commits, fsync of data and private parent directory, and fail-closed tamper detection. Identical archives are idempotent.
- New safe CLI verbs escrow-ingest and escrow-export are limited to the existing --lab-readonly mode. They never run Home Assistant services, automations, radios, Core, apps or passwords; source archive material is sensitive even if it remains encrypted.

## Evidence: not yet certified until CI success

- Synthetic integration test: scripts/test_p8ab_backup_escrow.py covers durable roundtrip and corruption, symlink, permission, stale identity and no-overwrite denial.
- Genuine two-VM test: .github/workflows/p8ab-native-escrow-rollback.yml uses official HAOS 18.3 QEMU image at pinned immutable SHA. The first disposable HAOS guest requests a REAL Supervisor backup, then real Rust escrow ingests SHA256 bytes, persists a private content-addressed image, exports byte-identical output and swaps the relay source to that exported file. A SECOND clean independent HAOS guest downloads **Rust-exported data**, restores using official Supervisor and checks original data marker, core version and distinct boot id. This is new evidence of a Rust-mediated disaster recovery flow rather than a repetition of the old direct two-guest transfer.
- This is a byte-preserving rollback guarantee at the archive level, NOT proof of executing imported Core or Apps on Rusty Home. No household data or real radios in tests.

## Remaining blockers

True HAOS->Rust full runtime, integration support, automation execution, app/container lifecycle and migration of live credentials are not implemented. Encrypted bytes are safely *retained*, but Rust never decrypts/restores protected archives itself; the official HAOS target does restoration. Independent long-running durability, multi-host storage replication and field data recoverability require later qualification.
