# P8aa — genuine HAOS backup archive to private Rust identity staging

**25 October 2026 — laboratory ONLY, 0/15 certified production gates.**

## Distinct work, no redundant certification

The prior HAOS->HAOS same-guest (17/17) and different-guest (6/6)
restore proofs and Core->Rust/Moli PR #89-#98 remain acquired.
The unresolved gap is consuming a **genuine Supervisor-created backup**
directly into native Rust, not repeating HAOS->HAOS restore.

Rust module: crates/interop-moli/src/ha_backup_import.rs
CLI: ha_registry_stager --lab-readonly PRIVATE_DIR import-unprotected-backup SOURCE_TAR

The importer enforces O_NOFOLLOW, single-hardlink and inode stability, exact
SHA-256 of the opened outer tar (1GiB cap), strict duplicate-key manifest,
required full/unprotected source, no mixed Core payload ambiguities, no tar
symlinks/devices/path traversal/duplicates, 128MiB compressed Core part,
512MiB decompressed cap and gzip CRC/trailer, 24MiB registry cap.
Only .storage/core.entity_registry is parsed into the existing durable redb.
It persists identity fields (potentially sensitive unique IDs) in a private
0700 directory / 0600 DB, so treat that state as **sensitive**. No other
Core config, authentication secret, add-on DB or payload is extracted.
Import never activates automatically. Existing CAS activation/status/rollback
remain explicit and independent.

## Three independent evidence scopes

1. Synthetic tar and identity fixtures in
   scripts/test_p8aa_backup_to_native_stage.py: positive durable import and
   rollback, plus encryption/duplicate/traversal/symlink/gzip hostility cases.
2. Official Home Assistant Core 2026.10.2 generates actual Core registry;
   run_p8q_real_core_to_native_registry.sh wraps its bytes in a SYNTHETIC
   HAOS-format tar, stages and rolls back. This is NOT a real Supervisor tar.
3. NEW disposable official HAOS 18.3 QEMU workflow
   .github/workflows/p8aa-haos-native-restore.yml produces a REAL complete
   Supervisor archive with explicitly fictional two-vendor Core registry
   identities injected into the throwaway guest. It transfers via a
   bounded localhost relay, validates SHA-256, stages in Rust and rolls back.
   **Do not count this proof until GitHub CI is green at the exact SHA.**

Reference:
https://contributors.home-assistant.io/os/supervisor/architecture/
https://contributors.home-assistant.io/apps/supervisor-api/backups/
https://www.home-assistant.io/common-tasks/general/backups/

## Not covered

Encrypted SecureTar needs official Supervisor/Core decryption. Integrations,
automations, secrets, Core runtime, accounts, third-party App databases,
service calls, actual radios, full HAOS->Rust OS -> HAOS disaster recovery,
multi-user permissions, 72-hour performance soak, independent security and
legal reviews remain unresolved. This is an isolated partial restore path;
haos_backup_restore remains lab_partial, rollback_verified remains blocked,
and the product is NOT_PRODUCTION_READY (0/15).
