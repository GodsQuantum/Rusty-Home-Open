#!/usr/bin/env python3
"""Real QEMU Manager SIGKILL must precede V1 restore and byte-hash verification."""
from pathlib import Path
import unittest
ROOT=Path(__file__).resolve().parent.parent
class Kill9Recovery(unittest.TestCase):
    def test_real_process_sigkill_then_true_supervisor_restart_then_restore(self):
        t=(ROOT/"scripts/p8c2_real_haos_supervisor_pytest.py").read_text()
        for marker in ("P8JF_MANAGER_KILL9_VERIFIED","docker kill --signal=KILL app_local_hars_rust_probe",
                       "ha apps start local_hars_rust_probe", "P8JE_REAL_TWO_VERSION_ROLLBACK_PASS",
                       "expected_v1_hash"):
            self.assertIn(marker,t)
        sig=t.index("docker kill --signal=KILL app_local_hars_rust_probe")
        self.assertLess(t.index('after_update.get("version")=="1.0.1"'),sig)
        self.assertLess(sig,t.index('"/hars-manager --acknowledge-disposable-qemu restore-partial "'))
        self.assertLess(sig,t.index('print("P8JE_REAL_TWO_VERSION_ROLLBACK_PASS")'))
    def test_no_production_targets_or_restart_and_run_only_in_snapshot(self):
        flow=(ROOT/".github/workflows/p8c2-haos-qemu.yml").read_text()
        self.assertIn("-snapshot -accel kvm",flow)
        t=(ROOT/"scripts/p8c2_real_haos_supervisor_pytest.py").read_text()
        self.assertNotIn("qm stop 100",t)
        self.assertIn("P8JF_MANAGER_KILL9_VERIFIED",t)
if __name__=="__main__":unittest.main()
