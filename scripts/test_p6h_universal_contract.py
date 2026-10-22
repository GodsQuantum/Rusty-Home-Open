#!/usr/bin/env python3
"""P6h release ZIP provenance and real Home Assistant lifecycle guard tests."""
from pathlib import Path
import copy
import importlib.util
import unittest

ROOT=Path(__file__).resolve().parent.parent
sp=importlib.util.spec_from_file_location("p6h",ROOT/"scripts/check_p6h_universal_report.py")
mod=importlib.util.module_from_spec(sp)
sp.loader.exec_module(mod)

class RealUniversalRelease(unittest.TestCase):
    def test_genuine_expected_report(self):
        mod.verify(copy.deepcopy(mod.REQUIRED))
    def test_every_boolean_state_is_required(self):
        for key,expected in mod.REQUIRED.items():
            if type(expected) is bool:
                x=copy.deepcopy(mod.REQUIRED);x[key]=not expected
                with self.subTest(key=key),self.assertRaises(ValueError):mod.verify(x)
    def test_manifest_and_release_mismatch_never_hidden(self):
        for key,value in (("manifest_version","v1.3.0"),("upstream_tag_manifest_version","v1.9.0"),
                          ("release_zip_sha256","0"*64)):
            x=copy.deepcopy(mod.REQUIRED);x[key]=value
            with self.subTest(key=key),self.assertRaises(ValueError):mod.verify(x)
    def test_no_fake_completeness_or_unknown_properties(self):
        x=copy.deepcopy(mod.REQUIRED);x["all_29_hacs_certified"]=True
        with self.assertRaises(ValueError):mod.verify(x)
        del x["all_29_hacs_certified"];del x["real_composite_entity_created"]
        with self.assertRaises(ValueError):mod.verify(x)
    def test_real_multistep_core_and_unavailable_restored(self):
        code=(ROOT/"scripts/test_p6h_real_universal_core.py").read_text()
        for wanted in ("async_setup_component(hass,component,cfg)",
                       '"next_step_id":"config_save"',
                       "real_composite_entity_created",
                       "STATE_UNAVAILABLE",
                       'unloaded.attributes.get("restored") is True',
                       "reloaded.state != STATE_UNAVAILABLE",
                       'not (set(hass.states.async_entity_ids("media_player")) & generated)'):
            self.assertIn(wanted,code)
    def test_git_release_and_docker_runtime_are_fenced(self):
        runner=(ROOT/"scripts/run_p6h_universal_core.sh").read_text()
        for wanted in ("gh release download v1.9.0",
                       "ec47b0114a5bf447ef1b7fa15cfa80829619f6785ece33ee4b5957ef906f7865",
                       "b07faa8840ce95567748bd798d3d9bbf30c2c433",
                       "4e7f19b5dd0c8d63985a6bf36f04317ea55b4d0e77c464bcc8cd03a6f4711f2e",
                       "stat.S_IFLNK", "len(names)==len(set(names))",
                       "--network none", "--read-only", "--cap-drop ALL",
                       "--security-opt no-new-privileges",
                       "scripts/check_p6h_universal_report.py"):
            self.assertIn(wanted,runner)
    def test_ci_runs_actual_official_core_without_mock(self):
        ci=(ROOT/".github/workflows/ci.yml").read_text()
        self.assertIn("python3 scripts/test_p6h_universal_contract.py",ci)
        self.assertIn("bash scripts/run_p6h_universal_core.sh",ci)
        self.assertIn("GH_TOKEN: ${{ github.token }}",ci)
if __name__=="__main__":unittest.main()
