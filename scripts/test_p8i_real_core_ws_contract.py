#!/usr/bin/env python3
"""Real QEMU WS probe is static, app-internal, bounded and non-authorizing."""
from pathlib import Path
import tomllib
import unittest
ROOT=Path(__file__).resolve().parent.parent
class CoreLiveContract(unittest.TestCase):
    def test_fixed_target_and_denied_public_input(self):
        package=tomllib.loads((ROOT/"crates/supervisor-apps/Cargo.toml").read_text())
        self.assertNotIn("core-user-live-lab",package["features"].get("default",[]))
        impl=(ROOT/"crates/supervisor-apps/src/core_user_live_lab.rs").read_text()
        self.assertIn('"ws://supervisor/core/websocket"',impl)
        self.assertIn('"config/auth/list"',impl)
        self.assertIn('env::var("SUPERVISOR_TOKEN")',impl)
        self.assertIn('env::var("HARS_P8H_QEMU_ONLY")',impl)
        self.assertNotIn('pub fn run_protocol',impl)
        self.assertNotIn('call_service',impl.split("#[cfg(test)]")[0].replace("No call_service",""))
    def test_disposable_guest_and_actual_qemu_protocol_gate(self):
        manager=(ROOT/"packaging/supervisor-app/hars_rust_probe/config.yaml").read_text()
        self.assertIn("homeassistant_api: false",manager)
        config=(ROOT/"packaging/supervisor-app/hars_core_ws_probe/config.yaml").read_text()
        self.assertIn("homeassistant_api: true",config)
        self.assertIn("hassio_api: false",config)
        self.assertIn("hassio_role: default",config)
        self.assertIn("protected: true",config)
        self.assertIn("docker_api: false",config)
        docker=(ROOT/"packaging/supervisor-app/hars_core_ws_probe/Dockerfile").read_text()
        self.assertIn("guest_core_users_probe /hars-core-users",docker)
        workflow=(ROOT/".github/workflows/p8c2-haos-qemu.yml").read_text()
        self.assertIn("packaging/supervisor-app/hars_core_ws_probe/config.yaml",workflow)
        self.assertIn("guest_core_users_probe",workflow)
        qemu=(ROOT/"scripts/p8c2_real_haos_supervisor_pytest.py").read_text()
        self.assertIn("docker exec app_local_hars_core_ws_probe /hars-core-users --probe",qemu)
        self.assertIn("P8I_REAL_CORE_WS_AUTHENTICATED_READ_ONLY_PROBE_PASS",qemu)
        self.assertIn("P8I_REAL_CORE_WS_USERS_READ",qemu)
        # An upstream permission denial is a valid NEGATIVE test only:
        # it must never count as successful proof of role resolution.
        self.assertIn('any("P8I_REAL_CORE_WS_USERS_READ" in item for item in proof)',qemu)
        self.assertIn('"P8I_REAL_CORE_WS_LIST_DENIED" not in',qemu)
        self.assertIn('"P8I_REAL_CORE_WS_LIST_DENIED"',qemu)
        self.assertIn("P8I_REAL_CORE_WS_PERMISSION_DENIED",qemu)

if __name__=="__main__":unittest.main()
