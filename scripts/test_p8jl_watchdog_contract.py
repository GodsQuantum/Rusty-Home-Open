#!/usr/bin/env python3
"""File Editor official Supervisor watchdog must be disabled, never assumed."""
import importlib.util
from pathlib import Path
import unittest

HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location("p8jl_watchdog",HERE/"p8jl_supervisor_watchdog.py")
mod=importlib.util.module_from_spec(spec)
spec.loader.exec_module(mod)

class DummyShell:
    def __init__(self):self.calls=[]
    def run_check(self,cmd,timeout):
        self.calls.append((cmd,timeout))
        return []

class Guard(unittest.TestCase):
    def test_actual_official_supervisor_api_write_and_get_attestation(self):
        shell=DummyShell()
        queries=[]
        def info(cmd,timeout):
            queries.append(cmd)
            return {"result":"ok","data":{"slug":"core_configurator","watchdog":False,"boot":"manual"}}
        self.assertEqual(mod.prepare_manual_stop(shell,info,"core_configurator"),
                         {"watchdog":False,"boot":"manual"})
        self.assertEqual(len(shell.calls),1)
        cmd,_=shell.calls[0]
        self.assertIn("docker exec hassio_cli sh -c",cmd)
        self.assertIn('Authorization: Bearer $SUPERVISOR_TOKEN',cmd)
        self.assertIn("http://supervisor/addons/core_configurator/options",cmd)
        self.assertIn('watchdog',cmd)
        self.assertIn('manual',cmd)
        self.assertIn(' -o /dev/null',cmd)
        self.assertEqual(len(queries),1)
        self.assertIn("ha apps info core_configurator",queries[0])
        self.assertNotIn("core_ssh",cmd)
        self.assertNotIn("rm ",cmd)
        self.assertNotIn("echo $SUPERVISOR_TOKEN",cmd)

    def test_fail_closed_on_watchdog_enabled_or_wrong_boot(self):
        for state in (
            {"slug":"core_configurator","watchdog":True,"boot":"manual"},
            {"slug":"core_configurator","watchdog":False,"boot":"auto"},
            {"slug":"evil_app","watchdog":False,"boot":"manual"},
            None
        ):
            def info(cmd,timeout):
                return {"result":"ok","data":state}
            with self.subTest(state=state),self.assertRaises(RuntimeError):
                mod.prepare_manual_stop(DummyShell(),info,"core_configurator")

    def test_unexpected_apps_cannot_change_options(self):
        for slug in ("core_ssh","local_hars_rust_probe","core_configurator;reboot"):
            shell=DummyShell()
            with self.assertRaises(ValueError):
                mod.prepare_manual_stop(shell,lambda *args:None,slug)
            self.assertFalse(shell.calls)

    def test_bad_supervisor_response_fails_before_attempting_stop(self):
        for response in ({"result":"error"},None,{},{"result":"ok","data":[]}):
            with self.assertRaises(RuntimeError):
                mod.prepare_manual_stop(DummyShell(),lambda *args, **kwargs:response,"core_configurator")

    def test_real_qemu_order_and_no_test_skips(self):
        suite=(HERE/"p8c2_real_haos_supervisor_pytest.py").read_text()
        portion=suite[suite.index("def test_hars_real_official_file_editor_app_lifecycle_qemu("):]
        self.assertIn("from p8jl_supervisor_watchdog import prepare_manual_stop",portion)
        self.assertLess(portion.index("_manual_stop(shell, shell_json, slug)"),
                        portion.index('shell_json, "stop", slug, shell=shell'))
        self.assertIn('stopped_info.get("state") == "stopped"',portion)
        assert_portion=(HERE/"check_p8d_junit.py").read_text()
        self.assertIn("16/16 expected cases",assert_portion)

if __name__=="__main__":unittest.main()
