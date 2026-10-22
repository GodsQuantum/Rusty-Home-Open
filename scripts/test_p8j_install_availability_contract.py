#!/usr/bin/env python3
"""P8j.a real Supervisor availability preflight must precede Rust install POST."""
from pathlib import Path
import unittest
ROOT=Path(__file__).resolve().parent.parent

class InstallAvailabilityGuard(unittest.TestCase):
    def test_preflight_is_inside_mutation_transport_before_install_post(self):
        rust=(ROOT/"crates/supervisor-apps/src/official_transport.rs").read_text()
        start=rust.index("pub fn preview_execute(")
        scope=rust[start:]
        must='if request.action() == crate::managed_apps::Action::Install'
        self.assertIn(must,scope)
        position=scope.index(must)
        self.assertIn("self.require_install_available(request.target())?",scope[position:])
        self.assertLess(position,scope.index("stream.write_all(body)"))
        self.assertIn('/store/addons/{app}/availability',rust)
        self.assertIn('GET {path} HTTP/1.1',rust)
    def test_no_available_override_or_skip_flag_in_guest(self):
        guest=(ROOT/"crates/supervisor-apps/examples/guest_supervisor_managed_preview.rs").read_text()
        self.assertNotIn("--skip-availability",guest)
        self.assertIn("grant_recorded_session(",guest)
        self.assertIn("client.preview_execute(permit)",guest)

if __name__=="__main__": unittest.main()
