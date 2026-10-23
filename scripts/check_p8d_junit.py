#!/usr/bin/env python3
"""Fail-closed verifier for upstream Home Assistant OS QEMU JUnit lifecycle proof.

Never consider a GitHub Actions SUCCESS with skipped pytest dependencies to
be real Supervisor App lifecycle certification.
"""
from __future__ import annotations
import sys
from pathlib import Path
from xml.etree import ElementTree as ET

EXPECTED = frozenset({
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
})

def verify_junit(path: Path) -> str:
    if not path.is_file() or path.stat().st_size > 10 * 1024 * 1024:
        raise ValueError("JUnit file missing or too large")
    root = ET.parse(path).getroot()
    cases = root.findall(".//testcase")
    names = [c.attrib.get("name", "") for c in cases]
    if len(cases) != len(EXPECTED) or set(names) != EXPECTED:
        raise ValueError("JUnit expected exactly seventeen unique Home Assistant lifecycle tests")
    for case in cases:
        if any(case.find(tag) is not None for tag in ("skipped", "failure", "error")):
            raise ValueError("JUnit contains skipped, failed or errored tests")
    return "PASS JUnit gate: 17/17 expected cases executed; 0 skipped, 0 failures, 0 errors"

if __name__ == "__main__":
    try:
        if len(sys.argv) != 2:
            raise ValueError("Expected one JUnit XML path")
        print(verify_junit(Path(sys.argv[1])))
    except (ValueError, OSError, ET.ParseError) as e:
        print(f"FAIL JUnit gate: {e}", file=sys.stderr)
        sys.exit(2)
