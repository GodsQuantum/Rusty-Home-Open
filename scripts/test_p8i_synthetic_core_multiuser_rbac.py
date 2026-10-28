#!/usr/bin/env python3
"""Contract guard: deterministic synthetic multi-user negative test and auditable rejection."""
from pathlib import Path
import tomllib
import unittest

ROOT = Path(__file__).resolve().parent.parent

class SyntheticCoreMultiuserRbacContract(unittest.TestCase):
    def test_multiuser_rbac_negative_test_guards(self):
        pkg = tomllib.loads((ROOT / "crates/supervisor-apps/Cargo.toml").read_text())
        features = pkg["features"]
        self.assertIn("core-corroborated-preview-lab", features)
        self.assertNotIn("core-corroborated-preview-lab", features.get("default", []))

        source = (ROOT / "crates/supervisor-apps/src/core_corroborated_permit_lab.rs").read_text()
        required_patterns = (
            "fn synthetic_multiuser_rbac_cross_user_and_revocation_denial()",
            "user A must be granted permit on own fixture",
            "user B must not issue confirmation for user A resource",
            "user B must be denied on user A resource",
            "cross-user claim and session token mismatch must be rejected",
            "expired session must be denied",
            "revoked Core user must be denied even with active session",
            "revoked session must be denied",
            'dir.path().join("audit.jsonl").exists()',
            "grant_checked_fixture_status(",
            "issue_checked_fixture_confirmation(",
            "sessions.revoke(",
        )
        for pattern in required_patterns:
            self.assertIn(pattern, source, f"Missing pattern: {pattern}")

        # Ensure no accidental unauthenticated or uncorroborated bypass in the lab
        disallowed = (
            "preview_execute(",
            "TcpListener::bind(",
            "SUPERVISOR_TOKEN",
            "POST /",
        )
        for term in disallowed:
            self.assertNotIn(term, source, f"Disallowed term found: {term}")

    def test_ci_wiring(self):
        ci = (ROOT / ".github/workflows/ci.yml").read_text()
        self.assertIn("python3 scripts/test_core_corroborated_preview_contract.py", ci)
        self.assertIn("python3 scripts/test_p8i_synthetic_core_multiuser_rbac.py", ci)
        self.assertIn("core-corroborated-preview-lab", ci)

if __name__ == "__main__":
    unittest.main()
