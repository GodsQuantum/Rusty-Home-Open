#!/usr/bin/env python3
"""P8f fail-closed packaging and real-QEMU runner contract, no production state."""
from pathlib import Path
import re
import unittest
ROOT=Path(__file__).resolve().parent.parent
APP=ROOT/"packaging/supervisor-app/hars_rust_probe"
FLOW=ROOT/".github/workflows/p8c2-haos-qemu.yml"
GUEST=ROOT/"scripts/p8c2_real_haos_supervisor_pytest.py"

class AppPackage(unittest.TestCase):
    def test_real_app_manifest_has_restricted_manager_role(self):
        raw=(APP/"config.yaml").read_text()
        for key,val in {
            "slug":"hars_rust_probe",
            "hassio_role":"manager",
            "hassio_api":"true",
            "protected":"true",
            "arch":'["amd64"]',
            "boot":"manual",
        }.items():
            self.assertRegex(raw,rf"(?m)^{re.escape(key)}:\s*{re.escape(val)}\s*$")
        for name in ("full_access","docker_api","host_network","host_pid","host_ipc","host_dbus","homeassistant_api"):
            self.assertNotRegex(raw,rf"(?m)^{re.escape(name)}:\s*true\s*$")
        for forbidden in ("devices","ports","map","image"):
            self.assertNotRegex(raw,rf"(?m)^{re.escape(forbidden)}:")
    def test_from_scratch_unprivileged_binary_container(self):
        docker=(APP/"Dockerfile").read_text()
        for required in ("FROM scratch",'io.hass.type="app"','io.hass.arch="amd64"',"USER 1000:1000","guest_supervisor_live"):
            self.assertIn(required,docker)
        self.assertNotIn("privileged",docker.lower())
        self.assertIn("ENV HARS_PROBE_STAY_RUNNING=1",docker)
        # Guest curl creates a 0644 build-context file; Docker must set +x.
        self.assertIn("COPY --chmod=0555 guest_supervisor_live /hars-supervisor-probe",docker)
        rust=(ROOT/"crates/supervisor-apps/examples/guest_supervisor_live.rs").read_text()
        self.assertIn("app_service_mode",rust)
        self.assertIn("std::thread::park()",rust)
    def test_unprivileged_qemu_audit_path_is_preowned_and_private(self):
        docker=(APP/"Dockerfile").read_text()
        guest_binary=(ROOT/"crates/supervisor-apps/examples/guest_supervisor_managed_preview.rs").read_text()
        runner=FLOW.read_text()
        guest=GUEST.read_text()
        self.assertIn("USER 1000:1000",docker)
        self.assertIn("COPY --chown=1000:1000 --chmod=0700 audit_layer/ /hars-lab-audit/",docker)
        self.assertIn('Path::new("/hars-lab-audit")',guest_binary)
        self.assertNotIn('Path::new("/data/hars-lab-audit")',guest_binary)
        self.assertIn("audit_layer/.seed",runner)
        self.assertIn("audit_layer/.seed",guest)
        self.assertTrue((APP/"audit_layer/.seed").is_file())

    def test_real_qemu_runs_manager_role_app_not_fake_cli(self):
        workflow=FLOW.read_text()
        guest=GUEST.read_text()
        for required in ("test_hars_manager_role_app","local_hars_rust_probe","hassio_role","P8E_REAL_RUST_SUPERVISOR_AUTH_PASS","10.0.2.2:18673/hars_rust_probe/"):
            self.assertIn(required,guest)
        for required in ("test_hars_manager_role_app","packaging/supervisor-app/hars_rust_probe","check_p8d_junit.py"):
            self.assertIn(required,workflow)
        self.assertNotIn("docker exec hassio_cli /tmp/hars-rust-probe",guest)

if __name__=="__main__":
    unittest.main()
