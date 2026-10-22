#!/usr/bin/env python3
"""P14: benchmark MUST measure genuine Core and genuine pinned Moli, never mock a speedup."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

ROOT=Path(__file__).resolve().parent.parent

def load_checker():
    path=ROOT/"scripts/check_p14_hot_read_benchmark.py"
    spec=importlib.util.spec_from_file_location("p14_checker",path)
    mod=importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod

class P14Proof(unittest.TestCase):
    def test_real_dual_process_and_pinned_core(self):
        workflow=(ROOT/".github/workflows/ci.yml").read_text()
        chunk=workflow[workflow.index("  p14-core-native-hot-read-microbench:"):].split("\n  rust-lab:",1)[0]
        for marker in ("runs-on: ubuntu-latest","cargo build --locked --release -p hars-native-hub --example bench_hot_state",
                       "ghcr.io/home-assistant/home-assistant@sha256:130241f28d01fa80dfa3b44f7d89781c188d8642ecab9024f079f19f9f82baa0",
                       "scripts/p14_core_hot_read.py","--network none","--read-only",
                       "--cap-drop ALL","check_p14_hot_read_benchmark.py",
                       "actions/upload-artifact@v4","if: always()", "docker ps -a"):
            self.assertIn(marker,chunk)
        self.assertNotIn("192.168.",chunk)
        self.assertNotIn("synthetic VM",chunk)

    def test_genuine_core_state_and_real_native_driver_no_fake_ratio(self):
        core=(ROOT/"scripts/p14_core_hot_read.py").read_text()
        rust=(ROOT/"crates/native-hub/examples/bench_hot_state.rs").read_text()
        self.assertIn("homeassistant.core.HomeAssistant(",core)
        self.assertIn("hass.states.async_set(",core)
        self.assertIn("hass.states.get(",core)
        self.assertIn("moli_runtime",rust)
        self.assertIn("spawn_driver(",rust)
        self.assertIn(".state(point)",rust)
        for s in (core,rust):
            self.assertIn("1232",s)
            self.assertIn("30",s)
            self.assertIn("round_ns",s)
        self.assertNotIn("faster_than_home_assistant",rust)
        self.assertNotIn("speedup_claim",core)

    def test_checker_rejects_forged_or_incomplete_samples(self):
        module=load_checker()
        def example(backend):
            return {"schema_version":1, "backend":backend,
                    "scope":"hot_in_memory_state_read",
                    "entities":1232, "warmup_rounds":5,
                    "round_ns":[80000+i*3 for i in range(30)],
                    "rss_kb":1024,"pss_kb":700,"durable":False}
        with tempfile.TemporaryDirectory() as root:
            a,b=Path(root)/"core.json",Path(root)/"rust.json"
            a.write_text(json.dumps(example("official_core_python")))
            b.write_text(json.dumps(example("moli_native_rust")))
            ok=module.compare(json.loads(a.read_text()),json.loads(b.read_text()))
            self.assertEqual(ok["rounds"],30)
            self.assertEqual(ok["entities"],1232)
            broken=example("moli_native_rust")
            broken["round_ns"]=[-1]*30
            with self.assertRaises((ValueError,AssertionError)):
                module.compare(example("official_core_python"),broken)
            broken=example("moli_native_rust")
            broken["round_ns"]=[1]*29
            with self.assertRaises((ValueError,AssertionError)):
                module.compare(example("official_core_python"),broken)
            broken=example("moli_native_rust")
            broken["scope"]="full_homeassistant_stack"
            with self.assertRaises((ValueError,AssertionError)):
                module.compare(example("official_core_python"),broken)
            with self.assertRaises((ValueError,AssertionError)):
                module.compare(example("official_core_python"),example("official_core_python"))

    def test_no_production_certification_claim(self):
        doc=(ROOT/"docs/P14_MICROBENCH_HA_CORE_VS_MOLI_2026-10-22.md").read_text()
        for word in ("NOT_PRODUCTION_READY","15/15","30", "1232",
                     "sans réseau", "pas une preuve"):
            self.assertIn(word,doc)
if __name__=="__main__":
    unittest.main()
