"""Offline unit test for disposable Supervisor plugin convergence and auto-update helper in p8c2."""
import importlib.util
import os
import sys
import types
from pathlib import Path

# Stub pytest for offline test run
def identity(*args, **kwargs):
    return lambda f: f

sys.modules.setdefault("pytest", types.SimpleNamespace(
    mark=types.SimpleNamespace(dependency=identity, timeout=identity)
))

SCRIPTS_DIR = Path(__file__).resolve().parent
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))

# Ensure GITHUB_WORKSPACE is set if unset
os.environ.setdefault("GITHUB_WORKSPACE", str(SCRIPTS_DIR.parent))

p8c2_path = SCRIPTS_DIR / "p8c2_real_haos_supervisor_pytest.py"
spec = importlib.util.spec_from_file_location("p8c2_module", p8c2_path)
p8c2 = importlib.util.module_from_spec(spec)
spec.loader.exec_module(p8c2)

assert hasattr(p8c2, "_plugin_is_current"), "_plugin_is_current missing from p8c2"
assert hasattr(p8c2, "_ensure_disposable_plugins_current"), "_ensure_disposable_plugins_current missing from p8c2"
assert hasattr(p8c2, "_prepare_disposable_supervisor_app_environment"), "_prepare_disposable_supervisor_app_environment missing from p8c2"

plugin_is_current = p8c2._plugin_is_current
ensure_plugins = p8c2._ensure_disposable_plugins_current
prepare_env = p8c2._prepare_disposable_supervisor_app_environment

# 1. Test _plugin_is_current
assert plugin_is_current({"result": "ok", "data": {"version": "1.0", "version_latest": "1.0"}}, "dns")
assert not plugin_is_current({"result": "ok", "data": {"version": "1.0", "version_latest": "2.0"}}, "dns")
for invalid in ({}, {"result": "error"}, {"result": "ok", "data": {"version": "1.0"}},
                {"result": "ok", "data": {"version": "1.0", "version_latest": ""}}):
    try:
        plugin_is_current(invalid, "cli")
    except AssertionError:
        pass
    else:
        raise AssertionError("Unknown or invalid plugin version accepted")

# 2. Test _ensure_disposable_plugins_current: Path A - already current (no-update path)
calls_no_up = []
def fake_shell_already_current(cmd, timeout=90):
    calls_no_up.append(cmd)
    return {"result": "ok", "data": {"version": "1.0.0", "version_latest": "1.0.0"}}

ensure_plugins(fake_shell_already_current, lambda s: None)
assert all("info" in c for c in calls_no_up), f"Unexpected commands: {calls_no_up}"
assert not any("update" in c for c in calls_no_up), "Update attempted when plugins already current"

# 3. Test _ensure_disposable_plugins_current: Path B - update required
calls_up = []
plugin_versions = {
    "dns": {"version": "1.0.0", "version_latest": "1.0.1"},
    "audio": {"version": "2.0.0", "version_latest": "2.0.0"},
    "multicast": {"version": "3.0.0", "version_latest": "3.0.0"},
    "cli": {"version": "4.0.0", "version_latest": "4.0.0"},
}
def fake_shell_update_required(cmd, timeout=90):
    calls_up.append(cmd)
    parts = cmd.split()
    plugin = parts[1]
    if "info" in cmd:
        return {"result": "ok", "data": dict(plugin_versions[plugin])}
    elif "update" in cmd:
        assert plugin == "dns"
        plugin_versions["dns"]["version"] = plugin_versions["dns"]["version_latest"]
        return {"result": "ok"}
    raise AssertionError(f"Unexpected command: {cmd}")

ensure_plugins(fake_shell_update_required, lambda s: None)
assert any("update" in c for c in calls_up), "Update not called when version outdated"
assert plugin_versions["dns"]["version"] == "1.0.1"

# 4. Test _prepare_disposable_supervisor_app_environment: auto-update already True
prep_calls_true = []
def fake_shell_prep_true(cmd, timeout=90):
    prep_calls_true.append(cmd)
    if "supervisor info" in cmd:
        return {"result": "ok", "data": {"auto_update": True}}
    if "info" in cmd:
        return {"result": "ok", "data": {"version": "1.0", "version_latest": "1.0"}}
    raise AssertionError(f"Unexpected command: {cmd}")

prepare_env(fake_shell_prep_true, lambda s: None)
assert any("supervisor info" in c for c in prep_calls_true)
assert not any("supervisor options" in c for c in prep_calls_true)
assert not any("supervisor reload" in c for c in prep_calls_true)

# 5. Test _prepare_disposable_supervisor_app_environment: auto-update False -> True
prep_calls_false = []
sup_state = {"auto_update": False}
def fake_shell_prep_false(cmd, timeout=90):
    prep_calls_false.append(cmd)
    if "supervisor info" in cmd:
        return {"result": "ok", "data": dict(sup_state)}
    if "supervisor options --auto-update" in cmd:
        return {"result": "ok"}
    if "supervisor reload" in cmd:
        sup_state["auto_update"] = True
        return {"result": "ok"}
    if "info" in cmd:
        return {"result": "ok", "data": {"version": "1.0", "version_latest": "1.0"}}
    raise AssertionError(f"Unexpected command: {cmd}")

prepare_env(fake_shell_prep_false, lambda s: None)
assert any("supervisor options --auto-update" in c for c in prep_calls_false)
assert any("supervisor reload" in c for c in prep_calls_false)

print("PASS: test_p8c2_plugin_environment offline mock tests passed")
