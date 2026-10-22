#!/usr/bin/env python3
"""P13: independent genuine Home Assistant Core -> Rust HTTP CI guard."""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parent.parent


class OfficialCoreRustE2E(unittest.TestCase):
    def test_ci_job_runs_real_core_rust_integration(self):
        workflow = (ROOT / ".github/workflows/ci.yml").read_text()
        self.assertIn("core-rust-genuine-e2e:", workflow)
        chunk = workflow[workflow.index("  core-rust-genuine-e2e:"):].split("\n  rust-lab:", 1)[0]
        self.assertIn("runs-on: ubuntu-latest", chunk)
        self.assertIn("timeout-minutes:", chunk)
        self.assertIn("bash scripts/test_core_control_plane_e2e.sh", chunk)
        self.assertIn("python3 scripts/test_p13_core_rust_e2e_ci.py", chunk)
        self.assertIn("if: always()", chunk)
        self.assertIn("docker ps -a", chunk)
        self.assertIn("hars-core-ui-", chunk)

    def test_fixture_never_contacts_real_home_or_exposes_live_values(self):
        s = (ROOT / "scripts/test_core_control_plane_e2e.sh").read_text()
        for must in (
            "--network none", "--read-only", "--cap-drop ALL",
            "--security-opt no-new-privileges", "--pids-limit 128",
            "--memory 1g", "--cpus 2", "--core-states",
            "type=bind,src=$ROOT/scripts/python_sidecar_fixture.py",
            "ghcr.io/home-assistant/home-assistant@sha256:",
            "entities_count", "==1232", "contains_entity_values",
            "STATUS", "== 503", "production_ready",
            "docker rm -f", "trap cleanup EXIT",
        ):
            self.assertIn(must, s)
        self.assertNotIn("192.168.", s)
        self.assertNotIn("SUPERVISOR_TOKEN", s)
        self.assertNotIn("synthetic VM", s)

    def test_enforced_release_block_and_honest_scope(self):
        doc = (ROOT / "docs/P13_REAL_CORE_RUST_CI_2026-10-22.md").read_text()
        for required in ("NOT_PRODUCTION_READY", "15/15",
                         "Core 2026.10.1", "503",
                         "pas un benchmark", "1232"):
            self.assertIn(required, doc)

if __name__ == "__main__":
    unittest.main()
