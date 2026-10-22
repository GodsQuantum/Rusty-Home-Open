#!/usr/bin/env python3
"""P6i strict upstream license and real Core lifecycle guard."""
import importlib.util,copy,unittest
from pathlib import Path
R=Path(__file__).resolve().parent.parent
s=importlib.util.spec_from_file_location("p6i",R/"scripts/check_p6i_adaptive_core_report.py")
m=importlib.util.module_from_spec(s);s.loader.exec_module(m)
class Adaptive(unittest.TestCase):
 def test_expected(self):m.verify(copy.deepcopy(m.REQUIRED))
 def test_each_boolean(self):
  for key,value in m.REQUIRED.items():
   if type(value) is bool:
    x=copy.deepcopy(m.REQUIRED);x[key]=not value
    with self.subTest(key=key),self.assertRaises(ValueError):m.verify(x)
 def test_version_attestation(self):
  for key,value in (("integration_version","1.31.0"),("upstream_commit","0"*40),("dependency_ulid_transform","2.2.8")):
   x=copy.deepcopy(m.REQUIRED);x[key]=value
   with self.subTest(key=key),self.assertRaises(ValueError):m.verify(x)
 def test_missing_extra(self):
  x=copy.deepcopy(m.REQUIRED);x["all_29_certified"]=True
  with self.assertRaises(ValueError):m.verify(x)
  del x["all_29_certified"];del x["real_core_switch_created"]
  with self.assertRaises(ValueError):m.verify(x)
 def test_official_core_true_unload(self):
  code=(R/"scripts/test_p6f_real_adaptive_core.py").read_text()
  for v in ("STATE_UNAVAILABLE",'state.attributes.get("restored") is True',"state.state != STATE_UNAVAILABLE",'assert not hass.states.async_entity_ids("switch")'):
   self.assertIn(v,code)
 def test_isolated_pinned_upstream(self):
  sh=(R/"scripts/run_p6f_real_adaptive_core.sh").read_text()
  for v in ("7f7fec4e34d331fa5fd84b95514ddb1baf2a401e","c71d239df91726fc519c6eb72d318ec65820627232b2f796219e87dcf35d0ab4","--network none","--read-only","--cap-drop ALL","scripts/check_p6i_adaptive_core_report.py"):
   self.assertIn(v,sh)
 def test_independent_official_core_ci(self):
  ci=(R/".github/workflows/ci.yml").read_text()
  self.assertIn("python3 scripts/test_p6i_adaptive_core_contract.py",ci)
  self.assertIn("bash scripts/run_p6f_real_adaptive_core.sh",ci)
if __name__=="__main__":unittest.main()
