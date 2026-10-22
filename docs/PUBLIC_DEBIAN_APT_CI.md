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
