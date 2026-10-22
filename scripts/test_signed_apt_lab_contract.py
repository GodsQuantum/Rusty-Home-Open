#!/usr/bin/env python3
"""Real offline apt-secure negative test contract for Rusty Home lab."""
from pathlib import Path
import unittest
ROOT=Path(__file__).resolve().parent.parent
class SignedAptLab(unittest.TestCase):
    def test_ephemeral_key_and_signed_metadata(self):
        s=(ROOT/"scripts/test_signed_apt_lab.sh").read_text()
        for x in ("mktemp -d", "trap 'rm -rf --", "ed25519 sign 1d",
                  "dpkg-scanpackages --multiversion", "apt-ftparchive",
                  "--clearsign", "--detach-sign", "--network none",
                  "--cap-drop ALL","no-new-privileges","signed-by="):
            self.assertIn(x,s)
        self.assertNotIn("apt-key add",s)
        self.assertNotIn("trusted=yes",s)
    def test_real_version_upgrade_and_negative_tamper(self):
        s=(ROOT/"scripts/test_signed_apt_lab.sh").read_text()
        for x in ("rusty-home-lab=0.1.0-1", "rusty-home-lab=0.1.1-1",
                  "P12B_SIGNED_APT_INSTALL_AND_UPGRADE_PASS",
                  "UNSIGNED_TAMPERING",
                  "P12B_TAMPERED_APT_SIGNATURE_REJECTED_PASS",
                  "P12B_SIGNED_APT_OFFLINE_END_TO_END_PASS"):
            self.assertIn(x,s)
    def test_ci_isolation_and_release_blocked(self):
        wf=(ROOT/".github/workflows/p12b-signed-apt-lab.yml").read_text()
        self.assertIn("  pull_request:",wf)
        self.assertIn("  contents: read",wf)
        self.assertIn("ubuntu-24.04",wf)
        self.assertIn("bash scripts/test_signed_apt_lab.sh",wf)
        self.assertNotIn("pull_request_target",wf)
        doc=(ROOT/"docs/P12B_SIGNED_APT_LAB_2026-10-22.md").read_text()
        self.assertIn("NOT_PRODUCTION_READY",doc)
        import json
        gates=json.loads((ROOT/"config/release-gates.json").read_text())["required_gates"]
        self.assertNotEqual(next(g["status"] for g in gates if g["id"]=="packaging_updates"),"certified")
if __name__=="__main__":
    unittest.main()
