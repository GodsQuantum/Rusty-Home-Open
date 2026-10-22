#!/usr/bin/env python3
"""Core-user lab response parser must not become an authentication endpoint."""
from pathlib import Path
import tomllib
import unittest
ROOT=Path(__file__).resolve().parent.parent
class CoreUserStatus(unittest.TestCase):
    def test_core_user_data_only_not_authority(self):
        features=tomllib.loads((ROOT/"crates/supervisor-apps/Cargo.toml").read_text())["features"]
        self.assertIn("core-user-snapshot-lab",features)
        self.assertNotIn("core-user-snapshot-lab",features.get("default",[]))
        lib=(ROOT/"crates/supervisor-apps/src/lib.rs").read_text()
        self.assertIn('#[cfg(feature = "core-user-snapshot-lab")]',lib)
        source=(ROOT/"crates/supervisor-apps/src/core_user_snapshot_lab.rs").read_text()
        for term in ("parse_fixture(", "is_active", "system_generated", "system-admin",
                     "Duplicate", "Stale", "Mismatch", "CoreAccountStatus"):
            self.assertIn(term,source)
        for forbidden in ("pub fn grant(", "AuditedPermit", "preview_execute(", "TcpListener::bind(", "SUPERVISOR_TOKEN"):
            self.assertNotIn(forbidden,source)
    def test_snapshot_not_claimed_authenticated_in_docs(self):
        doc=(ROOT/"docs/P8I_CORE_USER_AUTHORIZATION_LAB_2026-10-21.md").read_text()
        self.assertIn("PAS un fournisseur d'identité",doc)
        self.assertIn("15 critères",doc)
        self.assertIn("core/websocket",doc)
if __name__ == "__main__":
    unittest.main()
