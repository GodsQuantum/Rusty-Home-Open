#!/usr/bin/env python3
"""Tests for strict and bounded Supervisor /store/reload recovery logic."""
import importlib.util
from pathlib import Path
import unittest

P=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location("p8j_store",P/"p8j_supervisor_store_recovery.py")
mod=importlib.util.module_from_spec(spec)
spec.loader.exec_module(mod)
SLUG="local_hars_rust_probe"
def reply(version="0.1.0", slug=SLUG):
    return {"result":"ok","data":{"slug":slug,"version_latest":version}}

class TestStoreReadiness(unittest.TestCase):
    def test_already_available_never_reloads(self):
        calls=[]
        def fake(command,timeout):
            calls.append(command)
            return reply()
        outcome=mod.ensure_supervisor_local_app_version(fake,SLUG,"0.1.0",sleeper=lambda n:None)
        self.assertEqual(outcome["version_latest"],"0.1.0")
        self.assertEqual(len(calls),1)
        self.assertNotIn("ha store reload",calls[0])
    def test_timeout_after_server_applied_reload_is_not_repeated(self):
        calls=[]
        def fake(cmd,timeout):
            calls.append(cmd)
            if "store reload" in cmd:
                raise TimeoutError("response lost after successful POST")
            return {"result":"error"} if calls.count(cmd)==1 else reply()
        found=mod.ensure_supervisor_local_app_version(fake,SLUG,"0.1.0",sleeper=lambda n:None)
        self.assertEqual(found["slug"],SLUG)
        self.assertEqual(sum("store reload" in c for c in calls),1)
    def test_eventual_version_update_requires_observed_exact_version(self):
        calls=[]
        def fake(cmd,timeout):
            calls.append(cmd)
            if "reload" in cmd:
                return {"result":"ok"}
            if sum("reload" in c for c in calls)==0:
                return reply("1.0.0")
            return reply("1.0.1")
        got=mod.ensure_supervisor_local_app_version(fake,SLUG,"1.0.1",sleeper=lambda n:None)
        self.assertEqual(got["version_latest"],"1.0.1")
        self.assertEqual(sum("reload" in c for c in calls),1)
    def test_unreachable_supervisor_fails_closed_with_bounded_calls(self):
        calls=[]
        def fake(cmd,timeout):
            calls.append(cmd)
            raise TimeoutError("unavailable")
        with self.assertRaisesRegex(RuntimeError,"failed expected version"):
            mod.ensure_supervisor_local_app_version(fake,SLUG,"0.1.0",sleeper=lambda n:None)
        self.assertEqual(sum("store reload" in c for c in calls),2)
        self.assertLessEqual(len(calls),10)
    def test_wrong_slug_version_and_error_reply_not_accepted(self):
        def fake(cmd,timeout):
            if "reload" in cmd:return {"result":"error","message":"unauthorized"}
            return reply("0.1.0",slug="local_other")
        with self.assertRaises(RuntimeError):
            mod.ensure_supervisor_local_app_version(fake,SLUG,"0.1.0",sleeper=lambda n:None)
    def test_all_five_real_qemu_store_sites_require_exact_version(self):
        script=(P/"p8c2_real_haos_supervisor_pytest.py").read_text()
        self.assertEqual(script.count("_ensure_store(shell_json,"),5)
        for wanted in ('_ensure_store(shell_json,app,"0.1.0"',
                       '_ensure_store(shell_json,core_app,"0.1.0"',
                       '_ensure_store(shell_json,version_app,"1.0.0"',
                       '_ensure_store(shell_json,version_app,"1.0.1"'):
            self.assertIn(wanted,script)
        self.assertIn('_ensure_store(shell_json, app, "0.1.0-alpha.1"',script)
        self.assertNotIn('"ha store reload --no-progress --raw-json"',script)
    def test_real_haos_ci_autoruns_on_relevant_pr_without_write_token(self):
        workflow=(P.parent/".github/workflows/p8c2-haos-qemu.yml").read_text()
        self.assertIn("  pull_request:",workflow)
        self.assertIn('      - "scripts/p8j_supervisor_store_recovery.py"',workflow)
        self.assertIn('      - ".github/workflows/p8c2-haos-qemu.yml"',workflow)
        self.assertIn("  contents: read",workflow)
        self.assertNotIn("pull_request_target",workflow.split("permissions:")[0].replace(
            "# GitHub token is read-only; never use pull_request_target for test execution.",""))
        self.assertIn("-snapshot -accel kvm",workflow)
    def test_prerelease_versions_only_allow_bounded_safe_semver(self):
        target="local_rusty_home"
        def fake(cmd,timeout):
            return reply("0.1.0-alpha.1",target)
        found=mod.ensure_supervisor_local_app_version(
            fake,target,"0.1.0-alpha.1",sleeper=lambda n:None)
        self.assertEqual(found["version_latest"],"0.1.0-alpha.1")
        def forbidden(*args,**kwargs):
            raise AssertionError("no shell invocation expected")
        for unsafe in ["0.1.0-alpha/../../bad", "0.1.0-foo;ls",
                       "0.1.0-"+"a"*70, "0.1.0-alpha\nanything",
                       "0.1.0-alpha_unsafe"]:
            with self.assertRaises(ValueError):
                mod.ensure_supervisor_local_app_version(
                    forbidden,target,unsafe,sleeper=lambda n:None)

    def test_invalid_identifiers_cannot_be_used_to_build_shell_commands(self):
        def should_not_call(*args,**kwargs):
            raise AssertionError("no shell invocation expected")
        for slug,version in [
            ("core_ssh","0.1.0"),("local_thing; reboot","0.1.0"),
            ("local_thing","$(id)"),("local_thing","1.2.3; echo secret")
        ]:
            with self.assertRaises(ValueError):
                mod.ensure_supervisor_local_app_version(should_not_call,slug,version)
if __name__=="__main__":
    unittest.main()
