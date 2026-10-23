# Rusty Home deterministic source dependency SBOM (23 October 2026)

This public artifact is an SPDX 2.3 **source dependency graph**, generated
deterministically from locked Cargo metadata. It enumerates **217 packages**
including eight workspace packages, 205 registry crates, four Git sources,
and their 495 dependency/DESCRIBES relationships across all targets.

Artifact: [SPDX 2.3 JSON](../audits/CARGO_SOURCE_SPDX_2026-10-23.json).

Every package has a stable SPDX identifier, purl, name, exact version,
source archive SHA-256 when supplied in Cargo.lock, and the declared
SPDX-compatible licensing expression if available. Invalid or legacy
license expressions and absent workspace license declarations are represented
as **NOASSERTION**, not silently replaced by a more permissive license.

Git checkout paths and host file system paths are absent. Download URLs are
recorded only for registry packages. This artifact is NOT an inventory of
files actually linked into a shipped Rusty Home binary and no conclusion is
made about distribution rights.

An independent CI job regenerates the artifact at the exact Cargo.lock state,
compares it byte-for-byte, and validates it using the official SPDX
spdx-tools 0.8.3 reader. No signed SBOM has been issued, no independent legal
review performed, and no production gate was certified from this output.

The missing source LICENSE files for six external crate archives are separately
pinned to their upstream license texts under the P11 license provenance
audit. Remaining work includes attribution and NOTICE obligations, asset and
firmware licenses, and an externally reviewed signed SBOM for actual release
binaries. Rusty Home remains NOT_PRODUCTION_READY.
