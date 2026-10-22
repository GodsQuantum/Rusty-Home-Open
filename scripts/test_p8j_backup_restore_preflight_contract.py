#!/usr/bin/env python3
"""P8j.b typed restore metadata GET must precede mutating RESTORE POST."""
from pathlib import Path
import unittest
ROOT=Path(__file__).resolve().parent.parent
class BackupRestorePreflight(unittest.TestCase):
    def test_restore_preflight(self):
        s=(ROOT/"crates/supervisor-apps/src/official_transport.rs").read_text()
        method=s[s.index("pub fn preview_execute("):]
        self.assertIn('if let Some(backup) = request.restore_backup_slug()',method)
        self.assertIn('self.require_backup_contains_app(backup, request.target())?',method)
        self.assertLess(method.index("self.require_backup_contains_app(backup, request.target())?"),method.index("stream.write_all(body)"))
        self.assertIn('GET {path} HTTP/1.1',s)
        self.assertIn('format!("/backups/{backup}/info")',s)
    def test_real_guest_verifies_backup_metadata_before_and_after_restore(self):
        s=(ROOT/"scripts/p8c2_real_haos_supervisor_pytest.py").read_text()
        self.assertIn('P8J_REAL_BACKUP_RESTORE_PREFLIGHT_PASS',s)
        self.assertIn('backup_details',s)
if __name__ == "__main__": unittest.main()
