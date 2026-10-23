# License and source provenance

Project-authored Rusty Home code is MIT licensed. Upstream dependencies,
firmware and external Home Assistant components have separate rights.
Some dependencies await independent source-license and notice review.
Do not claim distribution license clearance yet.

## Six crates without a bundled source LICENSE file (2026-10-23)

The exact locked Rust workspace currently resolves **217 packages**:
**209 external** and eight workspace packages. Of the external packages,
203 bundle a file whose name begins LICENSE, LICENCE, COPYING, or NOTICE.
The remaining six are four rumqttc-next family crates 0.34.0 and
r-efi 5.3.0 / 6.0.0.

These six release versions are checked against precise upstream Git
commit trees: the four rumqttc-next crates inherit the root Apache-2.0
LICENSE; r-efi provides triple-license notices in its root AUTHORS file.
For each source, the evidence records exact version, upstream commit,
manifest Git blob, source license Git blob and SHA-256 of the full text,
without redistributing copyrighted source text.

[Immutable upstream evidence and source links](../audits/UPSTREAM_ROOT_LICENSE_PINS_2026-10-23.json).

The independent CI re-fetches all files from the pinned upstream commits.
A missing file, mismatched version, license SHA or tampering fails CI.

**This closes a source-location/provenance research gap, not the legal
clearance gate.** Redistribution and NOTICE attribution, six legacy
SPDX slash expressions, complete artwork/firmware notices, review of
embedded assets, signed SPDX SBOM and independent legal approval still
have no complete acceptance evidence. Hence third_party_licenses remains
lab_partial and Rusty Home remains NOT_PRODUCTION_READY.
