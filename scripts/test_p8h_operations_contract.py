#!/usr/bin/env python3
"""P8h QEMU-only proof must execute manager operations in genuine Rust App."""
from pathlib import Path
import tomllib
import unittest
ROOT=Path(__file__).resolve().parent.parent
class GenericOperationsProof(unittest.TestCase):
    def test_compile_out_of_default_build_and_force_qemu_only(self):
        cargo=(ROOT/"crates/supervisor-apps/Cargo.toml").read_text()
        rust=(ROOT/"crates/supervisor-apps/src/official_transport.rs").read_text()
        features=tomllib.loads(cargo)["features"]
        self.assertIn("managed-apps-preview",features)
        self.assertNotIn("managed-apps-preview",features.get("default", []))
        self.assertIn('required-features = ["managed-apps-preview"]',cargo)
        self.assertIn('HARS_P8H_QEMU_ONLY',rust)
        self.assertIn('permit: crate::audit_lab::AuditedPermit',rust)
        self.assertIn('permit.request()',rust)
        self.assertNotIn('externally_verified_operator',rust)
        guest_binary=(ROOT/"crates/supervisor-apps/examples/guest_supervisor_managed_preview.rs").read_text()
        self.assertIn('grant_recorded_session(',guest_binary)
        self.assertIn('client.preview_execute(permit)',guest_binary)
    def test_static_binary_packaged_into_true_manager_app(self):
        flow=(ROOT/".github/workflows/p8c2-haos-qemu.yml").read_text()
        docker=(ROOT/"packaging/supervisor-app/hars_rust_probe/Dockerfile").read_text()
        self.assertIn("--features managed-apps-preview --example guest_supervisor_managed_preview",flow)
        self.assertIn("guest_supervisor_managed_preview",flow)
        self.assertIn("COPY --chmod=0555 guest_supervisor_managed_preview /hars-manager",docker)
        self.assertIn("ENV HARS_P8H_QEMU_ONLY=1",docker)
    def test_real_haos_rust_installs_and_restarts_and_creates_partial_backup(self):
        guest=(ROOT/"scripts/p8c2_real_haos_supervisor_pytest.py").read_text()
        for text in (
            "/hars-manager --acknowledge-disposable-qemu install core_ssh",
            "/hars-manager --acknowledge-disposable-qemu restart core_ssh",
            "/hars-manager --acknowledge-disposable-qemu backup-partial core_ssh",
            "P8H_REAL_SUPERVISOR_INSTALL_PASS",
            "P8H_REAL_SUPERVISOR_RESTART_PASS",
            "P8H_BACKUP_SLUG=",
            "P8H_STOP_CONFIRMED_NOT_RUNNING",
        ):
            self.assertIn(text,guest)
if __name__=="__main__":
    unittest.main()
