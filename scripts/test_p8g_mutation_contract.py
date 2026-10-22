#!/usr/bin/env python3
"""P8g lab-only mutating REST transport is compiled out by default."""
from pathlib import Path
import unittest
ROOT=Path(__file__).resolve().parent.parent
PKG=ROOT/"crates/supervisor-apps"
DOCKER=ROOT/"packaging/supervisor-app/hars_rust_probe/Dockerfile"
WORKFLOW=ROOT/".github/workflows/p8c2-haos-qemu.yml"
GUEST=ROOT/"scripts/p8c2_real_haos_supervisor_pytest.py"

def assert_guest_stop_start_contract(guest: str):
    """Pin the *ordered* real HAOS proof, including physical Docker stop."""
    required_in_order = (
        "docker exec app_local_hars_rust_probe /hars-lifecycle stop",
        'assert any("P8G_REAL_RUST_MANAGER_STOP_PASS"',
        'stop_state in ("stopped", "error")',
        "live_containers = shell.run_check(",
        "docker ps --format",
        "grep -E '(^addon_|^app_)core_ssh$' || true",
        "assert not live_containers",
        "P8H_STOP_CONFIRMED_NOT_RUNNING",
        "docker exec app_local_hars_rust_probe /hars-lifecycle start",
        'assert any("P8G_REAL_RUST_MANAGER_START_PASS"',
        'get("state")=="started"',
    )
    cursor = 0
    for marker in required_in_order:
        found = guest.find(marker, cursor)
        assert found >= cursor, f"Missing or out-of-order HAOS safety proof: {marker}"
        cursor = found + len(marker)

class LabFixtureMutationContract(unittest.TestCase):
    def test_default_build_does_not_enable_mutation(self):
        config=(PKG/"Cargo.toml").read_text()
        assert "lab-app-actions = []" in config
        assert 'required-features = ["lab-app-actions"]' in config
        impl=(PKG/"src/official_transport.rs").read_text()
        assert '#[cfg(feature = "lab-app-actions")]' in impl
        assert '/addons/core_ssh/stop' in impl and '/addons/core_ssh/start' in impl
        assert 'HARS_P8G_QEMU_ONLY' in impl
    def test_guest_only_build_and_docker_env_restricts_access(self):
        workflow=WORKFLOW.read_text()
        docker=DOCKER.read_text()
        assert '--features lab-app-actions --example guest_supervisor_lab_lifecycle' in workflow
        assert 'guest_supervisor_lab_lifecycle' in docker
        assert "ENV HARS_P8G_QEMU_ONLY=1" in docker
        assert 'COPY --chmod=0555 guest_supervisor_lab_lifecycle' in docker
    def test_haos_genuine_manager_app_must_stop_then_start_fixture(self):
        guest=GUEST.read_text()
        assert "P8G_REAL_RUST_MANAGER_STOP_PASS" in guest
        assert "P8G_REAL_RUST_MANAGER_START_PASS" in guest
        assert 'docker exec app_local_hars_rust_probe /hars-lifecycle stop' in guest
        assert 'docker exec app_local_hars_rust_probe /hars-lifecycle start' in guest
        assert 'core_ssh' in guest
        assert_guest_stop_start_contract(guest)
    def test_contract_fails_if_docker_stopped_proof_is_removed(self):
        guest = GUEST.read_text()
        self.assertIn("assert not live_containers", guest)
        with self.assertRaisesRegex(AssertionError, "assert not live_containers"):
            assert_guest_stop_start_contract(
                guest.replace("assert not live_containers", "pass # missing proof", 1)
            )
if __name__=="__main__":unittest.main()
