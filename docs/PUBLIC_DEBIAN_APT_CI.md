# Rusty Home independent Linux packaging lab — bounded scope

The package CI tests **actual Debian package installation** with a Rust
read-only fixture binary in an isolated, temporary Ubuntu Docker image.
The disposable GitHub runner compiles from pinned Cargo.lock and tests
reproducibility of identical version artifacts before dpkg:
install 0.1.0-1, upgrade to 0.1.1-1, downgrade to 0.1.0-1 and purge.

A separate test generates a temporary OpenPGP signing key and signed
local APT repository, exercises `apt-get update/install/upgrade` using
`signed-by`, and deliberately corrupts the signed Release metadata.
APT must reject the tampered feed with a **signature-specific** error,
not an unrelated file or network failure.

Boundaries:

- These are **synthetic laboratory .deb files**, not a production signed
  distribution or a verified update policy for real installed homes.
- No real host configuration, service unit, privileged hardware driver,
  household credential or data backup is involved.
- CI runs on free public GitHub `ubuntu-24.04` runners.
- Published packages, rollback, signed updates and real upgrades still
  require third-party legal clearance, infrastructure and independent
  production tests. All 15 release gates remain blocked.

## P8ak/P8al native Rust HAOS manager extension (26 October 2026)

The earlier synthetic read-only CLI packaging tests remain valid and are
not a substitute for real HAOS guest host verification. P8ak PR #112
separately certified the actual versioned native Rust HAOS manager
Debian install, upgrade, downgrade and purge on ephemeral Ubuntu. The
service unit had no automatic start/enable, and real systemd+QEMU HAOS
18.3 start/stop was independently green.

P8al reuses this existing signed OpenPGP APT repository builder and
temporary offline verification client. Its signed Packages index now
contains BOTH native manager revisions as well as synthetic CLI revisions.
An offline client must discover the native versions only after successful
signed-by verification; corrupting InRelease must still fail with a genuine
cryptographic signature error. The ephemeral CI OpenPGP key is discarded
and has no production trust authority. The offline client does NOT install
the native manager (its QEMU system dependencies are absent there); the
real dpkg install lifecycle is certified under P8ak. Production release
remains NOT_PRODUCTION_READY and 0/15 fully certified gates.
