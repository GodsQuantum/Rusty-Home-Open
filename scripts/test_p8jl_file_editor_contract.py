#!/usr/bin/env python3
"""Real official File editor HAOS App test must be exact, isolated and fail-closed."""
from pathlib import Path
import unittest

ROOT=Path(__file__).resolve().parent.parent

class FileEditorOfficialLifecycle(unittest.TestCase):
    def test_real_official_store_app_not_synthetic_fixture(self):
        code=(ROOT/"scripts/p8c2_real_haos_supervisor_pytest.py").read_text()
        chunk=code[code.index("def test_hars_real_official_file_editor_app_lifecycle_qemu("):]
        for required in (
            '"core_configurator"',
            "ha apps info ", "ha apps install ", "ha apps logs ",
            "from p8jl_app_state_recovery import execute_and_observe",
            "installed_by_test", "state", "version_latest",
            "P8JL_REAL_OFFICIAL_FILE_EDITOR_INSTALL_START_STOP_UNINSTALL_PASS",
        ):
            self.assertIn(required,chunk)
        self.assertNotIn("/mnt/data/supervisor/apps/local/",chunk)
        self.assertNotIn("ha store add ",chunk)
        self.assertNotIn("192.168.",chunk)
        self.assertNotIn("gh auth",chunk)
    def test_finally_uninstalls_even_on_failing_start(self):
        code=(ROOT/"scripts/p8c2_real_haos_supervisor_pytest.py").read_text()
        chunk=code[code.index("def test_hars_real_official_file_editor_app_lifecycle_qemu("):]
        self.assertIn("finally:",chunk)
        self.assertLess(
            chunk.index("finally:"),
            chunk.index('_observed(shell_json, "uninstall", slug'),
        )
        self.assertIn('get("state") == "unknown"',chunk)
        self.assertIn("get(\"state\") == \"stopped\"",chunk)
    def test_real_qemu_kvm_and_strict_junit_fifteen(self):
        script=(ROOT/".github/workflows/p8c2-haos-qemu.yml").read_text()
        self.assertIn("-snapshot -accel kvm",script)
        self.assertIn("::test_hars_real_official_file_editor_app_lifecycle_qemu",script)
        self.assertLess(
            script.index("::test_hars_real_official_file_editor_app_lifecycle_qemu"),
            script.index("::test_hars_rust_supervisor_direct"),
        )
        self.assertIn("memory: 1G/memory: 3G",script)
        self.assertIn('depends=["test_hars_real_supervisor_store"]',
            (ROOT/"scripts/p8c2_real_haos_supervisor_pytest.py").read_text()
        )
        junit=(ROOT/"scripts/check_p8d_junit.py").read_text()
        self.assertIn('"test_hars_real_official_file_editor_app_lifecycle_qemu"',junit)
        self.assertIn("16/16 expected cases",junit)
        self.assertIn('("skipped", "failure", "error")',junit)
    def test_real_user_nine_app_data_not_falsely_claimed(self):
        doc=(ROOT/"docs/P8JL_REAL_OFFICIAL_FILE_EDITOR_APP_2026-10-22.md").read_text()
        for phrase in ("NOT_PRODUCTION_READY","15 barrières","huit autres Apps",
                       "synthetic VM","sans données personnelles"):
            self.assertIn(phrase,doc)
if __name__=="__main__":
    unittest.main()
