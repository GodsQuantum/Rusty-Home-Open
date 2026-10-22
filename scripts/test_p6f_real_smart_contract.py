#!/usr/bin/env python3
"""Guard real Smart Announcements Core proof against scope inflation."""
import copy
import importlib.util
from pathlib import Path
import unittest

ROOT=Path(__file__).resolve().parent.parent
spec=importlib.util.spec_from_file_location("smart_core",ROOT/"scripts/check_p6f_real_smart_report.py")
mod=importlib.util.module_from_spec(spec)
spec.loader.exec_module(mod)

class SmartAnnouncementsProof(unittest.TestCase):
    def test_strict_known_proof(self):
        mod.verify(copy.deepcopy(mod.REQUIRED))
    def test_each_positive_real_operation_required(self):
        for key,value in mod.REQUIRED.items():
            if value is True:
                broken=copy.deepcopy(mod.REQUIRED);broken[key]=False
                with self.subTest(key=key),self.assertRaises(ValueError):
                    mod.verify(broken)
    def test_no_real_user_or_sound_proof(self):
        for key in ("synthetic_tts_not_executed","real_user_devices_tested","production_ready"):
            broken=copy.deepcopy(mod.REQUIRED)
            broken[key]=not mod.REQUIRED[key]
            with self.subTest(key=key),self.assertRaises(ValueError):
                mod.verify(broken)
    def test_wrong_upstream_version_sha_not_accepted(self):
        for key,bad in (("version","0.8.4"),("commit","0"*40),("upstream","wrong/repo")):
            broken=copy.deepcopy(mod.REQUIRED);broken[key]=bad
            with self.subTest(key=key),self.assertRaises(ValueError):
                mod.verify(broken)
    def test_missing_or_extra_fields_not_accepted(self):
        broken=copy.deepcopy(mod.REQUIRED);del broken["core_unloaded_and_reloaded"]
        with self.assertRaises(ValueError):mod.verify(broken)
        broken=copy.deepcopy(mod.REQUIRED);broken["all_29_certified"]=True
        with self.assertRaises(ValueError):mod.verify(broken)
    def test_actual_offline_native_core_container_and_real_flow(self):
        run=(ROOT/"scripts/run_p6f_real_smart_core.sh").read_text()
        for marker in (
            "docker network create --internal --driver bridge",
            "docker network inspect", "--read-only", "--cap-drop ALL",
            "--security-opt no-new-privileges",
            "sha256sum -c -",
            "ghcr.io/home-assistant/home-assistant@sha256:",
        ):
            self.assertIn(marker,run)
        code=(ROOT/"scripts/test_p6f_real_smart_core.py").read_text()
        for marker in (
            "auth_manager_from_config(hass,[],[])",
            "hass.config_entries.flow.async_init",
            "hass.config_entries.flow.async_configure",
            "hass.config_entries.async_unload",
            "hass.config_entries.async_remove",
            '"person.fictional"',
            '"media_player.fictional"',
        ):
            self.assertIn(marker,code)
        self.assertNotIn("192.168.",code)
    def test_ci_repeats_real_upstream_case_in_pinned_core(self):
        workflow=(ROOT/".github/workflows/ci.yml").read_text()
        self.assertIn("python3 scripts/test_p6f_real_smart_contract.py",workflow)
        self.assertIn("bash scripts/run_p6f_real_smart_core.sh",workflow)

if __name__=="__main__":
    unittest.main()
