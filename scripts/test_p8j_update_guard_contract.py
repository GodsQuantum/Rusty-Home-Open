#!/usr/bin/env python3
"""P8j.d version-changing update requires official info, backup and postcondition."""
from pathlib import Path
import unittest
ROOT=Path(__file__).resolve().parent.parent
class UpdatePreflight(unittest.TestCase):
    def test_mutating_transport_is_guarded_pre_and_post(self):
        text=(ROOT/"crates/supervisor-apps/src/official_transport.rs").read_text()
        idx=text.index("pub fn preview_execute(")
        method=text[idx:]
        for k in (
            "require_update_available(request.target())?",
            "body.get(\"backup\")",
            "fetch_installed_app_info(request.target())?",
            "verify_update_postflight(&updated",
        ):
            self.assertIn(k,method)
        self.assertLess(method.index("require_update_available(request.target())?"),method.index("stream.write_all(body)"))
        self.assertGreater(method.index("verify_update_postflight(&updated"),method.index("stream.write_all(body)"))
        self.assertIn("GET /addons/{app}/info HTTP/1.1",text)
    def test_real_haos_guest_only_probes_no_version_change(self):
        q=(ROOT/"scripts/p8c2_real_haos_supervisor_pytest.py").read_text()
        rust=(ROOT/"crates/supervisor-apps/examples/guest_supervisor_managed_preview.rs").read_text()
        self.assertIn("probe-update core_ssh",q)
        self.assertIn("P8JD_REAL_READ_ONLY_UPDATE_PREFLIGHT_PASS",q)
        self.assertIn("preview_probe_update_eligibility(&app)",rust)
        self.assertIn("P8JD_REAL_SUPERVISOR_UPDATE_DENIED",rust)
        self.assertIn("P8JD_REAL_SUPERVISOR_UPDATE_READY",rust)
if __name__ == "__main__": unittest.main()
