# Rusty Home deterministic source dependency SBOM (23 October 2026)

This public artifact is an SPDX 2.3 **source dependency graph**, generated
deterministically from locked Cargo metadata. It enumerates **226 packages**
including eight workspace packages, 214 registry crates, four Git sources,
and their 524 dependency/DESCRIBES relationships across all targets.

Artifact: [SPDX 2.3 JSON](../audits/CARGO_SOURCE_SPDX_2026-10-23.json).

Every package has a stable SPDX identifier, purl, name, exact version,
source archive SHA-256 when supplied in Cargo.lock, and the declared
SPDX-compatible licensing expression if available. Invalid or legacy
license expressions are represented as **NOASSERTION**, not silently
replaced by a more permissive license. All eight first-party workspace crates
now explicitly declare MIT, including the native ha-rust-bridge package.
Seven declared-license expressions remain unresolved in the strict SPDX source inventory (including the newly added filetime 0.2.29 crate). These are NOASSERTION pending review, not assumed redistributable.

Git checkout paths and host file system paths are absent. Download URLs are
recorded only for registry packages. This artifact is NOT an inventory of
files actually linked into a shipped Rusty Home binary and no conclusion is
made about distribution rights.

An independent CI job regenerates the artifact at the exact Cargo.lock state,
compares it byte-for-byte, and validates it using the official SPDX
spdx-tools 0.8.3 reader. On successful pushes to main only, a separate
GitHub OIDC job signs an SBOM attestation whose subject is **only Cargo.lock**
and whose SBOM predicate is this repository's locked source dependency graph.
The action is pinned to its exact immutable release commit, and the
attestation must pass independent GitHub CLI Sigstore verification bound
to this repository's workflow and main branch. PR test code never receives
OIDC/attestation write permissions. This is not a release binary SBOM or
proof of legal compliance, and no production gate is certified by it.
If GitHub OIDC signing or verification fails, the source attestation is
not treated as issued.

The missing source LICENSE files for six external crate archives are separately
pinned to their upstream license texts under the P11 license provenance
audit. Remaining work includes attribution and NOTICE obligations, asset and
firmware licenses, and an externally reviewed signed SBOM for actual release
binaries. Rusty Home remains NOT_PRODUCTION_READY.

**Inventory refreshed 25 October 2026** after adding the native read-only official-format HAOS backup parser (tar/flate2 dependency closure). Source SPDX metadata is deterministic; signed binary SBOM and legal clearance remain pending.

**Refreshed 25 October 2026:** P8ac adds existing locked nix/sha2 dependencies to the native Hub; graph +2 edges, package count unchanged. Not a binary SBOM or legal clearance.
