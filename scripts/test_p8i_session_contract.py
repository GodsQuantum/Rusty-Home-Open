#!/usr/bin/env python3
"""P8i.d.1 QEMU fixture must use scoped, expiring sessions with one-use approval."""
from pathlib import Path
import unittest
ROOT=Path(__file__).resolve().parent.parent

class SessionLabContract(unittest.TestCase):
    def test_guest_session_gate_before_supervisor_mutation(self):
        guest=(ROOT/"crates/supervisor-apps/examples/guest_supervisor_managed_preview.rs").read_text()
        transport=(ROOT/"crates/supervisor-apps/src/official_transport.rs").read_text()
        self.assertIn("LabSessionRegistry::default()",guest)
        self.assertIn(".issue_fixture(",guest)
        self.assertIn(".issue_confirmation(",guest)
        self.assertIn(".grant_recorded_session(",guest)
        self.assertIn("client.preview_execute(permit)",guest)
        self.assertLess(guest.index(".grant_recorded_session("),guest.index("client.preview_execute(permit)"))
        self.assertIn("HARS_P8H_QEMU_ONLY",transport)
        self.assertIn("P8I_QEMU_FIXTURE_SESSION_GATE_PASS",guest)
    def test_strict_real_qemu_fixture_assertion_and_no_default_feature(self):
        qemu=(ROOT/"scripts/p8c2_real_haos_supervisor_pytest.py").read_text()
        self.assertIn("P8I_QEMU_FIXTURE_SESSION_GATE_PASS",qemu)
        pkg=(ROOT/"crates/supervisor-apps/Cargo.toml").read_text()
        self.assertIn('getrandom = { version = "0.4", optional = true }',pkg)
        self.assertIn('sha2 = { version = "0.10", optional = true }',pkg)
        rust=(ROOT/"crates/supervisor-apps/src/session_lab.rs").read_text()
        self.assertIn("Sha256::digest",rust)
        self.assertIn("getrandom::fill",rust)
        self.assertIn("self.approvals.remove",rust)
        self.assertIn("self.sessions.remove",rust)
if __name__=="__main__": unittest.main()
