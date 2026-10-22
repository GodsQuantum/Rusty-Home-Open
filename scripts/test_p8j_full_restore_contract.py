#!/usr/bin/env python3
"""P8j.i genuine full HAOS restore must be scoped and verifiable."""
from pathlib import Path
import unittest

ROOT=Path(__file__).resolve().parent.parent
class FullRestoreContract(unittest.TestCase):
    def test_mutate_before_full_restore_and_verify_core_plus_bytes(self):
        text=(ROOT/"scripts/p8c2_real_haos_supervisor_pytest.py").read_text()
        idx=text.index("def test_hars_full_restore_archive_qemu(")
        code=text[idx:text.index("def test_hars_protected_full_backup_restore_qemu(",idx)]
        for required in (
            "ha backups restore ", "backup.get(\"type\") == \"full\"",
            "rusty_home_full_restore_marker","P8JI_SHARE_BYTES_ORIGINAL_20261021",
            "ha supervisor info", "ha core info",
            "current.get(\"version\") == core_version",
            "P8JI_REAL_FULL_HAOS_RESTORE_CONTENT_AND_CORE_PASS",
        ):
            self.assertIn(required,code)
        self.assertIn('"ha backups restore " + slug + " --no-progress --raw-json"',code)
        self.assertNotIn('" --app ',code)
        self.assertNotIn('" --folders ',code)
        self.assertNotIn("qm ",code)
        self.assertLess(
            text.index("P8JI_SHARE_BYTES_MUTATED"),
            text.index("def test_hars_full_restore_archive_qemu("),
        )
    def test_dedicated_disposable_kvm_and_twelve_real_tests(self):
        wf=(ROOT/".github/workflows/p8c2-haos-qemu.yml").read_text()
        self.assertIn("-snapshot -accel kvm",wf)
        self.assertIn("::test_hars_full_restore_archive_qemu",wf)
        j=(ROOT/"scripts/check_p8d_junit.py").read_text()
        self.assertIn('"test_hars_full_restore_archive_qemu"',j)
        self.assertIn("16/16 expected cases",j)
        self.assertIn('("skipped", "failure", "error")',j)
if __name__=="__main__":
    unittest.main()
