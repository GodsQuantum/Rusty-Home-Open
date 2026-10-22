#!/usr/bin/env python3
"""P6j Epson SNMP offline impossible-printer fail-closed certificate."""
import importlib.util,copy,unittest
from pathlib import Path
ROOT=Path(__file__).resolve().parent.parent
sp=importlib.util.spec_from_file_location("p6j",ROOT/"scripts/check_p6j_epson_report.py")
mod=importlib.util.module_from_spec(sp);sp.loader.exec_module(mod)

class CorePrinterSafety(unittest.TestCase):
 def test_valid_negative_proof(self):mod.verify(copy.deepcopy(mod.REQUIRED))
 def test_true_false_evidence(self):
  for key,v in mod.REQUIRED.items():
   if type(v) is bool:
    changed=copy.deepcopy(mod.REQUIRED);changed[key]=not v
    with self.subTest(key=key),self.assertRaises(ValueError):mod.verify(changed)
 def test_fake_29_real_printers_and_versions(self):
  for key,value in (("upstream_version","0.1.2"),("pysnmp_version","7.1.27"),("upstream_git_sha","0"*40)):
   changed=copy.deepcopy(mod.REQUIRED);changed[key]=value
   with self.subTest(key=key),self.assertRaises(ValueError):mod.verify(changed)
 def test_unknown_or_missing_fields(self):
  changed=copy.deepcopy(mod.REQUIRED);changed["all_29_certified"]=True
  with self.assertRaises(ValueError):mod.verify(changed)
  del changed["all_29_certified"];del changed["fake_live_sensors_absent"]
  with self.assertRaises(ValueError):mod.verify(changed)
 def test_true_ha_configflow_retry_and_removal(self):
  code=(ROOT/"scripts/test_p6j_real_epson_offline_core.py").read_text()
  for part in ('ConfigEntryState.SETUP_RETRY','ConfigEntryState.SETUP_ERROR',
               'hass.config_entries.flow.async_configure',
               'hass.config_entries.async_remove',
               'duplicate_host_rejected','127.0.0.1'):
   self.assertIn(part,code)
  self.assertNotIn("192.168.",code)
 def test_real_source_dependency_and_offline_ci(self):
  shell=(ROOT/"scripts/run_p6j_real_epson_core.sh").read_text()
  for part in ("3a4f2c0120de7bfbf149041d484eefcc0b0cad0c",
               "949d1dfcd9a67213bf414b0741fe08319bd65ecf1fde5c56bad412a320bca24e",
               "57e704a6ba2bbf571d16cd5dc08b89bb3fa0ebeb5f4f26b87fececad3b3de7a6",
               "--network none","--read-only","--cap-drop ALL","scripts/check_p6j_epson_report.py"):
   self.assertIn(part,shell)
  ci=(ROOT/".github/workflows/ci.yml").read_text()
  self.assertIn("python3 scripts/test_p6j_epson_contract.py",ci)
  self.assertIn("bash scripts/run_p6j_real_epson_core.sh",ci)
if __name__=="__main__":unittest.main()
