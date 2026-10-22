#!/usr/bin/env python3
"""P6c Core manager lifecycle proof cannot be faked by static fixture success."""
import copy
import importlib.util
from pathlib import Path
import unittest

ROOT=Path(__file__).resolve().parent.parent
spec=importlib.util.spec_from_file_location("hars_lifecycle",ROOT/"scripts/check_config_entry_lifecycle_report.py")
module=importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)

class LifecycleChecks(unittest.TestCase):
    def test_expected_real_core_evidence(self):
        module.verify(copy.deepcopy(module.REQUIRED))
    def test_every_positive_claim_is_independently_required(self):
        for key,value in module.REQUIRED.items():
            if value is True:
                x=copy.deepcopy(module.REQUIRED)
                x[key]=False
                with self.subTest(key=key),self.assertRaises(ValueError):
                    module.verify(x)
    def test_real_29_status_and_production_never_promoted(self):
        for key,value in (
            ("actual_29_hacs_integrations_certified",29),
            ("actual_29_hacs_integrations_certified",True),
            ("synthetic_integration_count",29),
            ("production_ready",True),
        ):
            x=copy.deepcopy(module.REQUIRED);x[key]=value
            with self.subTest(key=key),self.assertRaises(ValueError):
                module.verify(x)
    def test_reject_extra_or_missing_keys(self):
        x=copy.deepcopy(module.REQUIRED);x["fake_full_parity"]=True
        with self.assertRaises(ValueError):module.verify(x)
        del x["fake_full_parity"]
        del x["real_core_unload_state_removed"]
        with self.assertRaises(ValueError):module.verify(x)
    def test_ci_uses_real_official_core_offline_not_direct_fixture_call(self):
        ci=(ROOT/".github/workflows/ci.yml").read_text()
        self.assertIn("bash scripts/run_config_entry_lifecycle_core.sh",ci)
        run=(ROOT/"scripts/run_config_entry_lifecycle_core.sh").read_text()
        for value in (
            "docker run", "--network none", "--read-only", "--cap-drop ALL",
            "--security-opt no-new-privileges", "--entrypoint python3",
            "ghcr.io/home-assistant/home-assistant@sha256:130241f28d01fa80dfa3b44f7d89781c188d8642ecab9024f079f19f9f82baa0",
            "check_config_entry_lifecycle_report.py",
        ):
            self.assertIn(value,run)
    def test_official_manager_api_and_no_live_access(self):
        content=(ROOT/"scripts/test_config_entry_lifecycle_core.py").read_text()
        for token in (
            "hass.config_entries.flow.async_init", "hass.config_entries.flow.async_configure",
            "hass.config_entries.async_unload", "hass.config_entries.async_setup",
            "hass.config_entries.async_remove", "ConfigEntryState.LOADED",
            "ConfigEntryState.NOT_LOADED", "duplicate_config_entry_denied",
        ):
            self.assertIn(token,content)
        self.assertNotIn("192.168.",content)
        self.assertNotIn("vaultwarden",content.lower())
        self.assertNotIn("192.168.",(ROOT/"scripts/fixtures/ha-config-lifecycle/config_flow.py").read_text())
if __name__=="__main__":
    unittest.main()
