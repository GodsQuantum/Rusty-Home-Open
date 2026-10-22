#!/usr/bin/env python3
"""Lab-only two-version real Supervisor App update -> backup restore contract."""
from pathlib import Path
import unittest
R=Path(__file__).resolve().parent.parent

class RealVersionRollbackContract(unittest.TestCase):
    def test_two_version_fixture_is_disposable_and_protected(self):
        root=R/"packaging/supervisor-app/hars_version_fixture"
        conf=(root/"config.yaml").read_text()
        docker=(root/"Dockerfile").read_text()
        for key in ('version: "1.0.0"', 'slug: hars_version_fixture','protected: true','docker_api: false','full_access: false','host_network: false'):
            self.assertIn(key,conf)
        self.assertIn("FROM scratch",docker)
        self.assertIn('USER 1000:1000',docker)
        self.assertNotIn("RUN ",docker)
    def test_real_qemu_requires_update_then_selected_backup_rollback_and_data_hash(self):
        workflow=(R/".github/workflows/p8c2-haos-qemu.yml").read_text()
        qemu=(R/"scripts/p8c2_real_haos_supervisor_pytest.py").read_text()
        for token in ("hars_version_fixture/config.yaml","hars_version_fixture/Dockerfile","hars_version_fixture/guest_supervisor_live"):
            self.assertIn(token,workflow)
        self.assertIn('hars_version_fixture/"+asset',qemu)
        self.assertIn('for asset in ("config.yaml", "Dockerfile", "guest_supervisor_live")',qemu)
        for marker in ('update-with-backup "+version_app',
                       'restore-partial "',
                       'P8JE_REAL_TWO_VERSION_ROLLBACK_PASS',
                       'P8JE_VERSION_ONE_ORIGINAL_BYTES',
                       'P8JE_VERSION_TWO_CHANGED_BYTES',
                       'expected_v1_hash'):
            self.assertIn(marker,qemu)
        self.assertIn("finally:",qemu)
        self.assertIn("ha apps uninstall ",qemu)
if __name__ == "__main__": unittest.main()
