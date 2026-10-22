#!/usr/bin/env python3
"""Fail-closed genuine upstream Watchman v0.8.4 Core lab contract."""
import copy
import importlib.util
from pathlib import Path
import unittest

ROOT=Path(__file__).resolve().parent.parent
spec=importlib.util.spec_from_file_location("watchman_proof",ROOT/"scripts/check_p6d_real_watchman_report.py")
mod=importlib.util.module_from_spec(spec)
spec.loader.exec_module(mod)

class P6dWatchman(unittest.TestCase):
    def test_exact_real_upstream_proof(self):
        mod.verify(copy.deepcopy(mod.EXPECTED))
    def test_truthful_nonproduction_flags(self):
        for key in ("real_user_config_or_devices_tested","production_ready"):
            bad=copy.deepcopy(mod.EXPECTED);bad[key]=True
            with self.subTest(key=key),self.assertRaises(ValueError):
                mod.verify(bad)
    def test_each_real_operation_is_mandatory(self):
        for k,v in mod.EXPECTED.items():
            if v is True:
                bad=copy.deepcopy(mod.EXPECTED);bad[k]=False
                with self.subTest(key=k),self.assertRaises(ValueError):
                    mod.verify(bad)
    def test_pins_are_immutable_and_types_exact(self):
        for k,v in (("upstream_commit","0"*40),("integration_version","0.8.5"),
                    ("dependency_prettytable","3.18.0"),("production_ready",0)):
            bad=copy.deepcopy(mod.EXPECTED);bad[k]=v
            with self.subTest(key=k),self.assertRaises(ValueError):
                mod.verify(bad)
    def test_extra_and_missing_keys_fail_closed(self):
        bad=copy.deepcopy(mod.EXPECTED);bad["real_29_integrations_certified"]=True
        with self.assertRaises(ValueError):mod.verify(bad)
        del bad["real_29_integrations_certified"]
        del bad["genuine_core_remove"]
        with self.assertRaises(ValueError):mod.verify(bad)
    def test_source_wheel_licence_and_sandbox_guards(self):
        sh=(ROOT/"scripts/run_p6d_real_watchman_core.sh").read_text()
        for fragment in ("git clone --quiet --depth 1 --branch v0.8.4",
                         "2eab21b7aeb4d4e20c71c3e2b3e449c42c4a3b35",
                         "8c81a95dbbcaccd99769c7fd5c2bee891ff8b7fdca45f70d046adcb22120d6ff",
                         "77ca0ad1c435b6e363d7e8623d7cc4fcf2cf15513bf77a1c1b2e814930ac57cc",
                         "sha256sum -c -","--network none","--read-only",
                         "--cap-drop ALL","--security-opt no-new-privileges",
                         "--entrypoint python3","check_p6d_real_watchman_report.py"):
            self.assertIn(fragment,sh)
        self.assertNotIn("docker.sock",sh)
        self.assertNotIn("192.168.",sh)
        self.assertIn("bash scripts/run_p6d_real_watchman_core.sh",
                      (ROOT/".github/workflows/ci.yml").read_text())
        source=(ROOT/"scripts/test_p6d_real_watchman_core.py").read_text()
        for required in ("hass.services.async_call(","blocking=True, return_response=True",
                         '"create_file": False', "missing_entities", "missing_actions",
                         'not Path("/config/watchman_report.txt").exists()'):
            self.assertIn(required,source)
        for val in ("hass.config_entries.flow.async_init","hass.config_entries.async_setup",
                    "hass.config_entries.async_unload","hass.config_entries.async_remove",
                    "hass.services.has_service","ConfigEntryState.NOT_LOADED"):
            self.assertIn(val,source)
if __name__=="__main__":
    unittest.main()
