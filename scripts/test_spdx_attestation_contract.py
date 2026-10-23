#!/usr/bin/env python3
"""No OIDC for PRs; only a GitHub-signed source lockfile, not an executable."""
from pathlib import Path
root=Path(__file__).resolve().parent.parent
ci=(root/".github/workflows/spdx-source-bom.yml").read_text()
source, attest=ci.split("  attest-main-source-lock:",1)
assert "permissions:\n  contents: read" in source
assert "id-token: write" not in source
assert "attestations: write" not in source
assert "if: github.event_name == 'push' && github.ref == 'refs/heads/main'" in attest
assert "needs: [source-spdx]" in attest
assert "id-token: write" in attest and "attestations: write" in attest
assert "uses: actions/attest@1e69f48acb82d1966a394da916b4c1698aa569d6" in attest
assert "subject-path: Cargo.lock" in attest
assert "sbom-path: audits/CARGO_SOURCE_SPDX_2026-10-23.json" in attest
assert "gh attestation verify Cargo.lock" in attest
assert "--predicate-type \"https://spdx.dev/Document/v2.3\"" in attest
assert "--source-ref refs/heads/main" in attest
assert "--signer-workflow" in attest
assert "python3 scripts/check_release_gate.py --expect-blocked" in attest
assert "push-to-registry: true" not in attest
assert "pull_request_target" not in ci
docs=(root/"docs/SOURCE_SPDX_2026-10-23.md").read_text()
assert "only Cargo.lock" in docs
assert "not a release binary" in docs
print("SPDX_SOURCE_ONLY_GITHUB_OIDC_ATTESTATION_CONTRACT_PASS")
