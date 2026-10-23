#!/usr/bin/env python3
"""P8j.m real Supervisor partial App-data restore must be byte-exact and scoped."""
from pathlib import Path
import unittest

ROOT=Path(__file__).resolve().parent.parent
class RealFileEditorPartialRestore(unittest.TestCase):
    def test_real_app_data_only_official_supervisor(self):
        data=(ROOT/"scripts/p8c2_real_haos_supervisor_pytest.py").read_text()
        start=data.index("def test_hars_real_official_file_editor_partial_restore_bytes_qemu(")
        case=data[start:]
        for mark in (
            '"core_configurator"', "ha apps install ",
            "/mnt/data/supervisor/apps/data/",
            "/mnt/data/supervisor/addons/data/",
            "len(available_roots) == 1", "sha256sum ",
            "ha backups new --name", "--app ", "ha backups info ",
            'meta.get("type") == "partial"',
            "ha backups restore ", "ha apps uninstall ",
            "ha backups remove ",
            "P8JM_REAL_OFFICIAL_FILE_EDITOR_DATA_BYTES_RESTORED_PASS"
        ):
            self.assertIn(mark,case)
        self.assertNotIn("/mnt/sample lab host",case)
        self.assertNotIn("192.168.",case)
        self.assertNotIn("SUPERVISOR_TOKEN",case)
    def test_original_is_actually_corrupted_before_restore(self):
        s=(ROOT/"scripts/p8c2_real_haos_supervisor_pytest.py").read_text()
        case=s[s.index("def test_hars_real_official_file_editor_partial_restore_bytes_qemu("):]
        for mark in (
            "P8JM_REAL_FILE_EDITOR_ORIGINAL_20261022",
            "P8JM_REAL_FILE_EDITOR_MUTATED_20261022",
            '"ha backups restore " + backup_slug',
            'recovered_hash[0].split()[0] == original_hash[0].split()[0]',
            "state.get(\"version\") == original_version",
            'core.get("result") == "ok"',
            'supervisor.get("result") == "ok"',
        ):
            self.assertIn(mark,case)
        self.assertLess(case.index("P8JM_REAL_FILE_EDITOR_MUTATED"),case.index('"ha backups restore "'))
        self.assertLess(case.index("finally:"),case.index('"ha apps uninstall "'))
    def test_official_haos_sixteen_exact_fail_closed(self):
        flow=(ROOT/".github/workflows/p8c2-haos-qemu.yml").read_text()
        self.assertIn("-snapshot -accel kvm",flow)
        self.assertIn("::test_hars_real_official_file_editor_partial_restore_bytes_qemu",flow)
        guard=(ROOT/"scripts/check_p8d_junit.py").read_text()
        self.assertIn('"test_hars_real_official_file_editor_partial_restore_bytes_qemu"',guard)
        self.assertIn("17/17 expected cases",guard)
        self.assertIn('("skipped", "failure", "error")',guard)
    def test_only_synthetic_data_not_global_certification(self):
        doc=(ROOT/"docs/P8JM_REAL_FILE_EDITOR_DATA_RESTORE_2026-10-22.md").read_text()
        for mark in ("NOT_PRODUCTION_READY","synthetic VM","huit autres Apps",
                     "données fictives","16/16"):
            self.assertIn(mark,doc)
if __name__ == "__main__":
    unittest.main()
