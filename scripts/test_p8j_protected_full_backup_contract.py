#!/usr/bin/env python3
"""Adversarial static safety contract for actual protected full HAOS backup in KVM."""
from pathlib import Path
import unittest

ROOT=Path(__file__).resolve().parent.parent

class ProtectedFullBackup(unittest.TestCase):
    def test_real_secure_full_restore_not_partial_or_placeholder(self):
        source=(ROOT/"scripts/p8c2_real_haos_supervisor_pytest.py").read_text()
        a=source.index("def test_hars_protected_full_backup_restore_qemu(")
        code=source[a:source.index(
            "def test_hars_protected_full_restore_wrong_password_rejected_qemu(",a
        )]
        for required in (
            '"ha backups info "',
            'item.get("type") == "full"',
            'item.get("protected") is True',
            "P8JJ_PROTECTED_ORIGINAL_BYTES_20261021",
            "P8JJ_PROTECTED_MUTATED_BYTES",
            "sha256sum /mnt/data/supervisor/backup/",
            "ha backups restore ",
            "fake_password",
            '--password "',
            "ha supervisor info",
            "ha core info",
            'item["homeassistant"]',
            "P8JJ_REAL_PROTECTED_FULL_RESTORE_EXACT_BYTES_AND_CORE_PASS",
        ):
            self.assertIn(required,code)
        self.assertNotIn('" --app ',code)
        self.assertNotIn('" --folders ',code)
        self.assertNotIn("shell.run_check(\"cat /home",code)
    def test_genuine_kvm_strict_fifteen_execution_cases(self):
        wf=(ROOT/".github/workflows/p8c2-haos-qemu.yml").read_text()
        self.assertIn("-snapshot -accel kvm",wf)
        self.assertIn("::test_hars_protected_full_backup_restore_qemu",wf)
        junit=(ROOT/"scripts/check_p8d_junit.py").read_text()
        self.assertIn('"test_hars_protected_full_backup_restore_qemu"',junit)
        self.assertIn("16/16 expected cases",junit)
        self.assertIn('("skipped", "failure", "error")',junit)
    def test_fixture_scope_and_encrypted_metadata_only_no_real_user_password(self):
        source=(ROOT/"scripts/p8c2_real_haos_supervisor_pytest.py").read_text()
        begin=source.index("def test_hars_protected_full_backup_restore_qemu(")
        sub=source[begin:source.index(
            "def test_hars_protected_full_restore_wrong_password_rejected_qemu(",begin
        )]
        self.assertIn("RustyQemuSyntheticOnly_2026_Protected",sub)
        self.assertIn("Test password is a public, fixed synthetic fixture",sub)
        self.assertNotIn("SUPERVISOR_TOKEN",sub)
        self.assertNotIn("synthetic VM",sub)

if __name__=="__main__":
    unittest.main()
