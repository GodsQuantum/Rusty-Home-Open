# Safe read-only HAOS backup archive inspection

The Home Assistant backup preflight **does not restore, extract, decrypt or authorize a migration**. It checks only the outer tar metadata, component headers and declared App coverage. Current official Supervisor uses securetar for password-protected backups; we must NOT reimplement that cryptography or claim validation of actual inner App data.

This hardening closes two security gaps before using the preflight on untrusted, user-provided archives:

1. The input is opened once with OS-level `O_NOFOLLOW | O_NONBLOCK | O_CLOEXEC`; `fstat()` validates that actual open file descriptor is a regular bounded file, not a symlink, FIFO or a path swapped between separate checks. The exact descriptor is used to read tar metadata. No live HAOS file or real backup is used in these tests.
2. `backup.json` is parsed with a duplicate-key rejecting object pairs hook, including *nested objects*. Contradictory `protected`, `type`, `addons` or nested App `slug` keys and NaN/Infinity values fail closed instead of silently taking the last value.

CI includes synthetic tests for nested duplicate JSON keys, malformed metadata, symbolic links, FIFOs and existing traversal/malicious tar member cases. It preserves Supervisor's safe leading `./` archive-name convention and leaves protected-backup component encryption to the official upstream restore path.

This is a **metadata safety check**, not a proven backup, cryptographic validation of encrypted data, real restoration, user migration, or one of the 15 certified production gates. The production-release checker remains NOT_PRODUCTION_READY.
