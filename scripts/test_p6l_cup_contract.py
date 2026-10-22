#!/usr/bin/env python3
import copy,unittest
from pathlib import Path
from check_p6l_cup_report import REQUIRED,verify
ROOT=Path(__file__).resolve().parent.parent
class Test(unittest.TestCase):
 def test_good(self):verify(copy.deepcopy(REQUIRED))
 def test_bad(self):
  for k,v in REQUIRED.items():
   t=copy.deepcopy(REQUIRED);t[k]=not v if type(v) is bool else "BAD"
   with self.subTest(k=k),self.assertRaises(ValueError):verify(t)
 def test_no_extra(self):
  t=copy.deepcopy(REQUIRED);t["all_29"]=True
  with self.assertRaises(ValueError):verify(t)
 def test_no_boolean_number_confusion(self):
  t=copy.deepcopy(REQUIRED);t["genuine_core"]=1
  with self.assertRaises(ValueError):verify(t)
 def test_release_source_placeholder_not_substituted(self):
  source=(ROOT/"scripts/run_p6l_real_cup_core.sh").read_text()
  for marker in ('"version"]=="0.0.0"', '"version"]=="v1.5.0"',
    "--internal --driver bridge","sha256sum -c","stat.S_ISLNK",
    "ghcr.io/home-assistant/home-assistant@sha256:"):
   self.assertIn(marker,source)
 def test_actual_core_ci(self):
  ci=(ROOT/".github/workflows/ci.yml").read_text()
  self.assertIn("python3 scripts/test_p6l_cup_contract.py",ci)
  self.assertIn("bash scripts/run_p6l_real_cup_core.sh",ci)
if __name__=="__main__":unittest.main()
