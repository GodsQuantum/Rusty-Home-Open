#!/usr/bin/env python3
"""Adversarial schema/CI contract. Not a substitute for real Core job."""
import copy
import unittest
from pathlib import Path
from check_p6k_blueprint_report import REQUIRED, verify
ROOT=Path(__file__).resolve().parent.parent
class Contract(unittest.TestCase):
    def test_exact_valid(self): verify(copy.deepcopy(REQUIRED))
    def test_all_positive_must_be_real(self):
        for field,expect in REQUIRED.items():
            if expect is True:
                data=copy.deepcopy(REQUIRED);data[field]=False
                with self.subTest(field=field),self.assertRaises(ValueError):verify(data)
    def test_negative_scope_cannot_be_upgraded(self):
        for field in ("real_user_data_tested","production_ready"):
            data=copy.deepcopy(REQUIRED);data[field]=True
            with self.assertRaises(ValueError):verify(data)
    def test_sha_version_and_type_cannot_be_faked(self):
        for field,bad in [("upstream_sha","0"*40),("upstream_tag","v2.5.1"),
            ("actual_core_http_admin_write_200",1),("paramiko_present",1)]:
            data=copy.deepcopy(REQUIRED);data[field]=bad
            with self.subTest(field=field),self.assertRaises(ValueError):verify(data)
    def test_unknown_evidence_denied(self):
        d=copy.deepcopy(REQUIRED);d["all_29_certified"]=True
        with self.assertRaises(ValueError):verify(d)
    def test_genuine_release_is_offline_and_in_ci(self):
        runner=(ROOT/"scripts/run_p6k_real_blueprint_core.sh").read_text()
        for needle in ("--internal --driver bridge","sha256sum -c",
            "--network", "--read-only", "--cap-drop ALL", "no-new-privileges",
            "ghcr.io/home-assistant/home-assistant@sha256:",
            "943aed0be67f3a910b0f70a3864288d5e17e55ce"):
            self.assertIn(needle,runner)
        harness=(ROOT/"scripts/test_p6k_blueprint_real_core.py").read_text()
        for needle in ('hass.config_entries.flow.async_init',
            'ClientSession(', 'GROUP_ID_USER',
            'hass.auth.async_create_refresh_token','status == 403'):
            self.assertIn(needle,harness)
        ci=(ROOT/".github/workflows/ci.yml").read_text()
        self.assertIn("bash scripts/run_p6k_real_blueprint_core.sh",ci)
        self.assertIn("python3 scripts/test_p6k_blueprint_contract.py",ci)
if __name__=="__main__":unittest.main()
