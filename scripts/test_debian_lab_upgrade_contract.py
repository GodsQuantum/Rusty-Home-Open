#!/usr/bin/env python3
"""P12a installable Debian lab evidence, not a production packaging approval."""
from pathlib import Path
import unittest
ROOT=Path(__file__).resolve().parent.parent

class RootlessDebianLab(unittest.TestCase):
    def test_builder_is_rootless_and_no_service_autostart(self):
        script=(ROOT/"scripts/build_debian_lab_package.sh").read_text()
        for required in ("$(id -u)", "cargo build --release --locked -p ha-rust-bridge",
                         "--root-owner-group", "SOURCE_DATE_EPOCH",
                         "rusty-home-lab_", "0.1.0-1|0.1.1-1"):
            self.assertIn(required,script)
        for forbidden in ("systemctl", "sudo ", "setcap", "postinst", "preinst"):
            self.assertNotIn(forbidden,script)
        self.assertIn("LAB ONLY",script)
    def test_upgrade_is_real_dpkg_in_isolated_offline_container(self):
        script=(ROOT/"scripts/test_debian_lab_upgrade.sh").read_text()
        for required in ("docker run --rm", "--network none", "--cap-drop ALL",
                         "--security-opt no-new-privileges", "dpkg -i",
                         "dpkg --purge rusty-home-lab", "0.1.1-1", "0.1.0-1",
                         "--fixture /fixture.jsonl", "P12A_REAL_DPKG_INSTALL_UPGRADE_DOWNGRADE_PURGE_PASS"):
            self.assertIn(required,script)
        self.assertIn("P12A_REPRODUCIBLE_DEBIAN_PACKAGE_PASS",script)
        self.assertIn("ubuntu@sha256:68ee22004a9109b44973313b02ff7b1de250f1de0f5ecb628d73f579aaff33ac",script)
        self.assertIn("grep -q",script)
        self.assertIn("read_only",script)
        self.assertIn('rusty-home-lab-upgrade-$RANDOM',script)
        self.assertNotIn("pct ",script)
        self.assertNotIn("qm ",script)
    def test_release_gate_does_not_grant_production(self):
        import json
        gates=json.loads((ROOT/"config/release-gates.json").read_text())["required_gates"]
        packaging=next(x for x in gates if x["id"]=="packaging_updates")
        self.assertNotEqual(packaging["status"],"certified")
        doc=(ROOT/"docs/P12A_DEBIAN_LAB_INSTALL_UPGRADE_2026-10-22.md").read_text()
        self.assertIn("NOT_PRODUCTION_READY",doc)
        self.assertIn("pas de mises à jour automatiques",doc)
    def test_independent_ci_runs_rootless_real_dpkg(self):
        wf=(ROOT/".github/workflows/p12a-debian-package-lab.yml").read_text()
        self.assertIn("  pull_request:",wf)
        self.assertIn("  contents: read",wf)
        self.assertIn("runs-on: ubuntu-24.04",wf)
        self.assertIn("bash scripts/test_debian_lab_upgrade.sh",wf)
        self.assertIn("python3 scripts/test_debian_lab_upgrade_contract.py",wf)
        self.assertNotIn("pull_request_target",wf)
if __name__=="__main__":unittest.main()
