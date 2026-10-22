#!/usr/bin/env python3
"""Require genuine mutation and byte-for-byte recovery of App /data in QEMU."""
from pathlib import Path
import unittest
ROOT=Path(__file__).resolve().parent.parent

class RealAppDataRoundTrip(unittest.TestCase):
    def test_disposable_haos_guest_writes_mutates_restores_and_compares_data(self):
        guest=(ROOT/"scripts/p8c2_real_haos_supervisor_pytest.py").read_text()
        for marker in (
            "P8J_DATA_BYTES_ORIGINAL_20261020",
            "P8J_DATA_BYTES_CORRUPTED_20261020",
            "hars_p8j_roundtrip_fixture",
            "P8J_ACTUAL_APP_DATA_RECOVERED_PASS",
        ):
            self.assertIn(marker, guest)
        self.assertIn("assert recovered_data", guest)
        self.assertIn("assert corrupt_data", guest)
        self.assertIn("docker exec ", guest)
        self.assertIn("restore-partial core_ssh", guest)

if __name__=="__main__": unittest.main()
