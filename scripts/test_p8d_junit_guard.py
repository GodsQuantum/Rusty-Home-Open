#!/usr/bin/env python3
"""True RED/GREEN tests for real-HAOS pytest/JUnit gate (no guest needed)."""
import subprocess
import sys
import tempfile
from pathlib import Path
import unittest
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parent.parent
VALIDATOR = ROOT / "scripts/check_p8d_junit.py"
EXPECTED = (
    "test_start_supervisor",
    "test_check_supervisor",
    "test_supervisor_is_updated",
    "test_hars_real_supervisor_store",
    "test_hars_rust_supervisor_direct",
    "test_hars_manager_role_app",
    "test_hars_real_product_app_install_boot",
    "test_app_install",
    "test_create_backup",
    "test_app_uninstall",
    "test_restore_backup",
    "test_hars_full_backup_archive_qemu",
    "test_hars_full_restore_archive_qemu",
    "test_hars_protected_full_backup_restore_qemu",
    "test_hars_protected_full_restore_wrong_password_rejected_qemu",
    "test_hars_real_official_file_editor_app_lifecycle_qemu",
    "test_hars_real_official_file_editor_partial_restore_bytes_qemu",
)

def run_validation(names=EXPECTED, skip=None, fail=None, error=None):
    xml = ET.Element("testsuite", tests=str(len(names)))
    for name in names:
        case = ET.SubElement(xml, "testcase", name=name, classname="supervisor_test.test_supervisor")
        if name == skip:
            ET.SubElement(case, "skipped", message="dependency failure")
        if name == fail:
            ET.SubElement(case, "failure", message="failed")
        if name == error:
            ET.SubElement(case, "error", message="error")
    with tempfile.TemporaryDirectory(prefix="hars-junit-") as directory:
        output = Path(directory) / "results.xml"
        ET.ElementTree(xml).write(output, encoding="utf-8", xml_declaration=True)
        return subprocess.run([sys.executable, str(VALIDATOR), str(output)],
                              text=True, capture_output=True, timeout=5)

class RealHaosJUnitGuardTests(unittest.TestCase):
    def test_all_seventeen_real_lifecycle_tests_must_pass(self):
        result = run_validation()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("17/17", result.stdout)

    def test_pytest_skipped_dependency_is_not_success(self):
        result = run_validation(skip="test_restore_backup")
        self.assertNotEqual(result.returncode, 0)

    def test_missing_app_restore_is_not_success(self):
        result = run_validation(names=EXPECTED[:-1])
        self.assertNotEqual(result.returncode, 0)

    def test_test_failures_and_errors_are_not_success(self):
        for key in ("fail", "error"):
            result = run_validation(**{key: "test_app_install"})
            self.assertNotEqual(result.returncode, 0)

    def test_duplicate_testcase_cannot_substitute_for_missing_case(self):
        result = run_validation(names=EXPECTED[:-1] + ("test_start_supervisor",))
        self.assertNotEqual(result.returncode, 0)

if __name__ == "__main__":
    unittest.main()
