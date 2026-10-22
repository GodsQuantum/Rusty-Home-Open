#!/usr/bin/env python3
"""Check upstream immutable assets and actual Core lifecycle fail-closed proof."""
import copy
import importlib.util
from pathlib import Path
import unittest

ROOT=Path(__file__).resolve().parent.parent
spec=importlib.util.spec_from_file_location("cb_core",ROOT/"scripts/check_p6g_real_card_builder_report.py")
mod=importlib.util.module_from_spec(spec)
spec.loader.exec_module(mod)

class CardBuilderRealCoreContract(unittest.TestCase):
    def test_exact_synthetic_evidence(self):
        mod.verify(copy.deepcopy(mod.REQUIRED))
    def test_all_positive_claims_required(self):
        for key,expected in mod.REQUIRED.items():
            if expected is True:
                broken=copy.deepcopy(mod.REQUIRED)
                broken[key]=False
                with self.subTest(key=key),self.assertRaises(ValueError):
                    mod.verify(broken)
    def test_negative_prod_flags_must_remain_false(self):
        for key in ("real_user_dashboard_tested","production_ready"):
            broken=copy.deepcopy(mod.REQUIRED)
            broken[key]=True
            with self.subTest(key=key),self.assertRaises(ValueError):
                mod.verify(broken)
    def test_type_confusion_upstream_sha_and_version_rejected(self):
        for key,bad in (
            ("upstream_tag","v2.6.1"),
            ("zip_release_sha256","0"*64),
            ("upstream_commit","0"*40),
            ("core_remove",1),
        ):
            broken=copy.deepcopy(mod.REQUIRED);broken[key]=bad
            with self.subTest(key=key),self.assertRaises(ValueError):
                mod.verify(broken)
    def test_shape_extensions_or_missing_key_rejected(self):
        broken=copy.deepcopy(mod.REQUIRED);broken["all_29_compatible"]=True
        with self.assertRaises(ValueError):mod.verify(broken)
        del broken["all_29_compatible"]
        del broken["upstream_tag"]
        with self.assertRaises(ValueError):mod.verify(broken)
    def test_real_core_is_offline_and_upstream_asset_immutable(self):
        source=(ROOT/"scripts/run_p6g_real_card_builder_core.sh").read_text()
        for marker in (
            "docker network create --internal --driver bridge",
            'docker network inspect',
            "--read-only", "--cap-drop ALL",
            "--security-opt no-new-privileges",
            "sha256sum -c", 'stat.S_ISLNK',
            "ghcr.io/home-assistant/home-assistant@sha256:",
            "scripts/check_p6g_real_card_builder_report.py",
        ):
            self.assertIn(marker,source)
        fixture=(ROOT/"scripts/test_p6g_real_card_builder_core.py").read_text()
        for marker in ("hass.auth = await auth_manager_from_config(hass, [], [])",
                       "hass.config_entries.flow.async_init",
                       "hass.config_entries.async_unload",
                       "hass.config_entries.async_remove",
                       'Path("/config/www/card_builder").is_dir()'):
            self.assertIn(marker,fixture)
    def test_real_core_job_runs_in_github_actions(self):
        workflow=(ROOT/".github/workflows/ci.yml").read_text()
        self.assertIn("bash scripts/run_p6g_real_card_builder_core.sh",workflow)
        self.assertIn("python3 scripts/test_p6g_real_card_builder_contract.py",workflow)
if __name__=="__main__":
    unittest.main()
