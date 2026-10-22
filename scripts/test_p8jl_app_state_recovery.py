#!/usr/bin/env python3
"""Adversarial native File Editor Supervisor lifecycle POST reconciliation."""
import importlib.util
from pathlib import Path
import unittest

R=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location("p8jl",R/"p8jl_app_state_recovery.py")
mod=importlib.util.module_from_spec(spec)
spec.loader.exec_module(mod)

def data(state,slug="core_configurator"):
    return {"result":"ok","data":{"slug":slug,"state":state}}
class ObservedNativeSupervisor(unittest.TestCase):
    def test_success_uses_observed_state_not_post_response_alone(self):
        calls=[]
        def fake(cmd,timeout):
            calls.append(cmd)
            return {"result":"ok"} if "ha apps start " in cmd else data("started")
        actual=mod.execute_and_observe(fake,"start","core_configurator",sleeper=lambda _:None)
        self.assertEqual(actual["state"],"started")
        self.assertEqual(sum("ha apps start " in c for c in calls),1)
    def test_lost_post_reply_then_delayed_started_is_never_reposted(self):
        calls=[]
        def fake(cmd,timeout):
            calls.append(cmd)
            if "ha apps start " in cmd:raise RuntimeError(
                'Post "http://supervisor/addons/core_configurator/start": context deadline exceeded (Client.Timeout exceeded while awaiting headers)'
            )
            return data("stopped" if len(calls)<=3 else "started")
        result=mod.execute_and_observe(fake,"start","core_configurator",sleeper=lambda _:None,attempts=5)
        self.assertEqual(result["state"],"started")
        self.assertEqual(sum("ha apps start " in c for c in calls),1)
    def test_unresponsive_stays_failed_with_bounded_polls(self):
        calls=[]
        def fake(cmd,timeout):
            calls.append(cmd)
            if "ha apps stop " in cmd:raise TimeoutError("context deadline exceeded")
            return data("started")
        with self.assertRaisesRegex(RuntimeError,"never reached"):
            mod.execute_and_observe(fake,"stop","core_configurator",sleeper=lambda _:None,attempts=3)
        self.assertEqual(sum("ha apps stop " in c for c in calls),1)
        self.assertEqual(len(calls),4)
    def test_explicit_rejected_response_cannot_be_overridden(self):
        calls=[]
        def fake(cmd,timeout):
            calls.append(cmd)
            return {"result":"error","message":"denied"}
        with self.assertRaisesRegex(RuntimeError,"rejected"):
            mod.execute_and_observe(fake,"start","core_configurator",sleeper=lambda _:None)
        self.assertEqual(len(calls),1)
    def test_non_timeout_auth_error_is_not_swallowed(self):
        def fake(cmd,timeout):
            raise RuntimeError("403 unauthorized")
        with self.assertRaisesRegex(RuntimeError,"unauthorized"):
            mod.execute_and_observe(fake,"start","core_configurator",sleeper=lambda _:None)
    def test_incorrect_slug_and_state_cannot_fake_success(self):
        for observed in (data("started","local_not_real"),
                         data("error"), {"result":"error"},
                         {"result":"ok","data":None}):
            def fake(cmd,timeout):
                return {"result":"ok"} if "ha apps start " in cmd else observed
            with self.assertRaisesRegex(RuntimeError,"never reached"):
                mod.execute_and_observe(fake,"start","core_configurator",sleeper=lambda _:None,attempts=1)
    def test_uninstall_is_observed_using_documented_unknown_state(self):
        def fake(cmd,timeout):
            return {"result":"ok"} if "ha apps uninstall " in cmd else data("unknown")
        self.assertEqual(mod.execute_and_observe(fake,"uninstall","core_configurator",sleeper=lambda _:None)["state"],"unknown")


    def test_real_docker_container_removed_after_stop_can_attest_supervisor_error(self):
        class Guest:
            def __init__(self):self.calls=[]
            def run_check(self,cmd,timeout):
                self.calls.append(cmd)
                if cmd.startswith("docker ps --format "):
                    return ["addon_core_configurator"]
                if "docker inspect" in cmd:
                    return ["true|/addon_core_configurator"]
                return []  # docker ps -a: no container remains after cleanup
        guest=Guest()
        calls=[]
        error_state=data("error")
        error_state["data"].update({"watchdog":False,"boot":"manual"})
        def api(cmd,timeout):
            calls.append(cmd)
            return {"result":"ok"} if cmd.startswith("ha apps stop ") else error_state
        observed=mod.execute_and_observe(
            api,"stop","core_configurator",
            shell=guest,sleeper=lambda _:None,attempts=3,
        )
        self.assertEqual(observed["state"],"error")
        self.assertIs(observed["_process_stopped_verified"],True)
        self.assertIs(observed["_docker_container_removed_verified"],True)
        self.assertIs(observed["_supervisor_error_state_observed"],True)
        self.assertEqual(len(guest.calls),4)
        self.assertIn("docker ps --format",guest.calls[0])
        self.assertIn("docker inspect",guest.calls[1])
        self.assertTrue(all("docker ps -a" in c for c in guest.calls[2:]))
        self.assertEqual(sum(c.startswith("ha apps stop ") for c in calls),1)
        self.assertEqual(sum(c.startswith("ha apps info ") for c in calls),2)

    def test_error_does_not_count_if_docker_proof_not_exact(self):
        cases=(
            (["false|/addon_core_configurator"], [], False, "pre-not-running"),
            (["true|/other_app"], [], False, "pre-wrong-container"),
            ([], [], False, "pre-missing"),
            (["true|/addon_core_configurator"], ["addon_core_configurator"], False, "post-still-present"),
            (["true|/addon_core_configurator"], [], True, "watchdog-enabled"),
        )
        for before,after,watchdog,label in cases:
            class Guest:
                def run_check(self,cmd,timeout):
                    if cmd.startswith("docker ps --format "):return ["addon_core_configurator"]
                    if "docker inspect" in cmd:return before
                    return after
            reply=data("error")
            reply["data"].update({"watchdog":watchdog,"boot":"manual"})
            def api(cmd,timeout):
                return {"result":"ok"} if cmd.startswith("ha apps stop ") else reply
            with self.subTest(label=label),self.assertRaises((RuntimeError, ValueError)):
                mod.execute_and_observe(api,"stop","core_configurator",
                                        shell=Guest(),sleeper=lambda _:None,attempts=1)

    def test_second_absence_probe_or_supervisor_state_change_cannot_fake_cleanup(self):
        for fail_kind in ("running_second","state_changed","automatic_boot"):
            class Guest:
                def __init__(self):self.absence=0
                def run_check(self,cmd,timeout):
                    if cmd.startswith("docker ps --format "):
                        return ["addon_core_configurator"]
                    if "docker inspect" in cmd:
                        return ["true|/addon_core_configurator"]
                    self.absence+=1
                    return ["addon_core_configurator"] if (fail_kind=="running_second" and self.absence==2) else []
            calls=[]
            first=data("error");first["data"].update({"watchdog":False,"boot":"manual"})
            second=data("error" if fail_kind!="state_changed" else "started")
            second["data"].update({"watchdog":False,"boot":"auto" if fail_kind=="automatic_boot" else "manual"})
            def api(cmd,timeout):
                if cmd.startswith("ha apps stop "):return {"result":"ok"}
                calls.append(cmd)
                return first if len(calls)==1 else second
            with self.subTest(kind=fail_kind),self.assertRaisesRegex(RuntimeError,"never reached"):
                mod.execute_and_observe(api,"stop","core_configurator",
                                        shell=Guest(),sleeper=lambda _:None,attempts=1)

    def test_modern_app_container_name_is_attested(self):
        class Guest:
            def __init__(self): self.calls=[]
            def run_check(self,cmd,timeout):
                self.calls.append(cmd)
                if cmd.startswith("docker ps --format "):
                    return ["hassio_supervisor", "app_core_configurator"]
                if cmd.startswith("docker inspect "):
                    return ["true|/app_core_configurator"]
                if cmd.startswith("docker ps -a --format "):
                    return ["hassio_supervisor"]
                raise AssertionError("Unexpected Docker command: "+cmd)
        guest=Guest()
        error_state=data("error")
        error_state["data"].update({"watchdog":False,"boot":"manual"})
        calls=[]
        def api(cmd,timeout):
            calls.append(cmd)
            return {"result":"ok"} if cmd.startswith("ha apps stop ") else error_state
        actual=mod.execute_and_observe(
            api,"stop","core_configurator",shell=guest,sleeper=lambda _:None,attempts=2
        )
        self.assertIs(actual["_process_stopped_verified"], True)
        self.assertEqual(sum(c.startswith("ha apps stop ") for c in calls),1)
        self.assertTrue(any(c.startswith("docker inspect ") for c in guest.calls))
        self.assertTrue(any(c.startswith("docker ps --format ") for c in guest.calls))

    def test_no_running_or_ambiguous_container_rejects_before_post(self):
        for running in ([], ["other_app"], ["addon_core_configurator","app_core_configurator"]):
            calls=[]
            class Guest:
                def run_check(self,cmd,timeout):
                    if cmd.startswith("docker ps --format "):
                        return running
                    raise AssertionError("Must not inspect or mutate without unique running name")
            def api(cmd,timeout):
                calls.append(cmd)
                raise AssertionError("Must not send stop")
            with self.subTest(running=running),self.assertRaisesRegex(RuntimeError,"No unique running"):
                mod.execute_and_observe(api,"stop","core_configurator",
                                        shell=Guest(),sleeper=lambda _:None,attempts=1)
            self.assertEqual(calls,[])

    def test_alternate_container_must_not_remain_after_stop(self):
        class Guest:
            def run_check(self,cmd,timeout):
                if cmd.startswith("docker ps --format "):return ["app_core_configurator"]
                if cmd.startswith("docker inspect "):return ["true|/app_core_configurator"]
                if cmd.startswith("docker ps -a --format "):return ["addon_core_configurator"]
                raise AssertionError(cmd)
        reply=data("error")
        reply["data"].update({"watchdog":False,"boot":"manual"})
        def api(cmd,timeout):
            return {"result":"ok"} if cmd.startswith("ha apps stop ") else reply
        with self.assertRaisesRegex(RuntimeError,"never reached"):
            mod.execute_and_observe(api,"stop","core_configurator",
                                    shell=Guest(),sleeper=lambda _:None,attempts=1)

    def test_invalid_actions_identifiers_and_unbounded_retries_are_rejected(self):
        def should_not_call(*args,**kwargs):raise AssertionError("must not call shell")
        for action,slug in (("update","core_configurator"),("start","core_ssh"),
                            ("start","core_configurator; rm -rf /")):
            with self.assertRaises(ValueError):
                mod.execute_and_observe(should_not_call,action,slug)
        for bad in (0,81,True,1.2):
            with self.assertRaises(ValueError):
                mod.execute_and_observe(should_not_call,"start","core_configurator",attempts=bad)
    def test_genuine_qemu_harness_wires_read_after_timeout(self):
        code=(R/"p8c2_real_haos_supervisor_pytest.py").read_text()
        chunk=code[code.index("def test_hars_real_official_file_editor_app_lifecycle_qemu("):]
        self.assertIn("from p8jl_app_state_recovery import execute_and_observe",chunk)
        self.assertIn('_observed(shell_json, "start", slug',chunk)
        self.assertIn('stopped_info = _observed(',chunk)
        self.assertIn('shell_json, "stop", slug, shell=shell',chunk)
        self.assertIn('proven_upstream_error_stop',chunk)
        self.assertIn('_observed(shell_json, "uninstall", slug',chunk)
        self.assertNotIn('"ha apps start " + slug + " --no-progress --raw-json"',chunk.split("def test_hars_real_official_file_editor_partial_restore_bytes_qemu")[0])
if __name__=="__main__":
    unittest.main()
