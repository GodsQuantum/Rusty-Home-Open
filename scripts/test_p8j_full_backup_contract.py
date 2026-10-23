#!/usr/bin/env python3
"""Full-backup QEMU proof must remain real, bounded, and independently gated."""
from pathlib import Path
import unittest

ROOT=Path(__file__).resolve().parent.parent
class FullBackupContract(unittest.TestCase):
    def test_real_backup_is_full_and_hashed_not_synthetic(self):
        code=(ROOT/"scripts/p8c2_real_haos_supervisor_pytest.py").read_text()
        start=code.index("def test_hars_full_backup_archive_qemu(")
        end=code.index("def test_hars_full_restore_archive_qemu(", start)
        full=code[start:end]
        for required in ("ha backups new --name", "ha backups info ",
                         'item.get("type") == "full"',
                         'item.get("homeassistant")',
                         'item.get("size_bytes")',
                         "sha256sum /mnt/data/supervisor/backup/",
                         "P8JH_REAL_FULL_HAOS_BACKUP_ARCHIVE_SHA256_PASS"):
            self.assertIn(required,full)
        self.assertNotIn("core_ssh --app",full)
        self.assertNotIn("ha backups restore ",full)
    def test_disposable_qemu_and_strict_junit_no_skip(self):
        workflow=(ROOT/".github/workflows/p8c2-haos-qemu.yml").read_text()
        self.assertIn("-snapshot -accel kvm",workflow)
        self.assertIn("supervisor_test/test_supervisor.py::test_hars_full_backup_archive_qemu",workflow)
        junit=(ROOT/"scripts/check_p8d_junit.py").read_text()
        self.assertIn('"test_hars_full_backup_archive_qemu"',junit)
        self.assertIn("17/17 expected cases",junit)
        self.assertIn('("skipped", "failure", "error")',junit)
if __name__=="__main__":
    unittest.main()
