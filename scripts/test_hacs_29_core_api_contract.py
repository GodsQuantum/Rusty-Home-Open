#!/usr/bin/env python3
"""Fail-closed 29-integration official Core report tests."""
import copy
import importlib.util
from pathlib import Path
import unittest
ROOT=Path(__file__).resolve().parent.parent
spec=importlib.util.spec_from_file_location("core_report",ROOT/"scripts/check_hacs_29_core_api_report.py")
mod=importlib.util.module_from_spec(spec)
spec.loader.exec_module(mod)

class PinnedOfficialCore(unittest.TestCase):
    def test_expected_static_report(self):
        mod.validate(copy.deepcopy(mod.REQUIRED))
    def test_every_integer_is_strict(self):
        for key in ("integrations_inventory","distinct_ha_import_paths",
                    "paths_resolvable","paths_unresolved",
                    "integrations_with_unresolved_import_paths"):
            broken=copy.deepcopy(mod.REQUIRED)
            broken[key]=True if broken[key]==0 else broken[key]+1
            with self.subTest(key=key),self.assertRaises(ValueError):
                mod.validate(broken)
    def test_false_functionality_flags_never_promoted(self):
        for key in ("all_hacs_compatible","config_flow_tested_for_29","real_devices_touched"):
            broken=copy.deepcopy(mod.REQUIRED);broken[key]=True
            with self.subTest(key=key),self.assertRaises(ValueError):
                mod.validate(broken)
    def test_unresolved_paths_or_wrong_core_fail(self):
        for key,value in (("core_version","2026.11.0"),("unresolved_paths",["homeassistant.nonexistent"]),
                          ("scope","all_hacs_compatible")):
            broken=copy.deepcopy(mod.REQUIRED);broken[key]=value
            with self.subTest(key=key),self.assertRaises(ValueError):
                mod.validate(broken)
    def test_unknown_fields_and_missing_fields_fail(self):
        broken=copy.deepcopy(mod.REQUIRED);broken["runtime_functional"]=True
        with self.assertRaises(ValueError):mod.validate(broken)
        broken=copy.deepcopy(mod.REQUIRED);del broken["core_version"]
        with self.assertRaises(ValueError):mod.validate(broken)
    def test_real_ci_job_and_sandbox_guards(self):
        ci=(ROOT/".github/workflows/ci.yml").read_text()
        self.assertIn("bash scripts/run_hacs_core_api_audit.sh",ci)
        self.assertIn("python3 scripts/check_hacs_29_core_api_report.py",ci)
        script=(ROOT/"scripts/run_hacs_core_api_audit.sh").read_text()
        self.assertIn("GITHUB_ACTIONS",script)
        for required in ("--network none","--read-only","--cap-drop ALL","--user 1000:1000",
                         "sha256:130241f28d01fa80dfa3b44f7d89781c188d8642ecab9024f079f19f9f82baa0"):
            self.assertIn(required,script)

if __name__=="__main__":unittest.main()
