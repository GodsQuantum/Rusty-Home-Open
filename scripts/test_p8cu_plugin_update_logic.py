#!/usr/bin/env python3
"""No VM: test exact version/postcondition and one-shot update discipline."""
import importlib.util
import sys,types
# The helper must be testable without installing pytest in LXC700.
def identity(*args, **kwargs):
    return lambda f:f
sys.modules.setdefault("pytest",types.SimpleNamespace(
    mark=types.SimpleNamespace(dependency=identity,timeout=identity)))
from pathlib import Path
p=Path(__file__).with_name("p8cu_official_file_editor_partial_qemu_pytest.py")
spec=importlib.util.spec_from_file_location("p8cu_guest_fixture",p)
mod=importlib.util.module_from_spec(spec)
spec.loader.exec_module(mod)
assert mod._plugin_is_current({"result":"ok","data":{"version":"1.0","version_latest":"1.0"}},"cli")
for invalid in ({}, {"result":"error"}, {"result":"ok","data":{"version":"1.0"}},
                {"result":"ok","data":{"version":"1.0","version_latest":""}}):
    try: mod._plugin_is_current(invalid,"cli")
    except AssertionError: pass
    else: raise AssertionError("Unknown plugin version accepted")
updates=[]
versions={"dns":"1","audio":"1","multicast":"1","cli":"1"}
def fake(command, timeout=90):
    name=command.split()[1]
    if " info " in command:
        return {"result":"ok","data":{"version":versions[name],"version_latest":"2"}}
    assert " update " in command
    updates.append(name)
    versions[name]="2"
    if name=="cli": raise RuntimeError("CLI disconnected after update")
    return {"result":"ok"}
mod._ensure_disposable_plugins_current(fake,lambda seconds:None)
assert updates==["dns","audio","multicast","cli"],updates
mod._ensure_disposable_plugins_current(fake,lambda seconds:None)
assert len(updates)==4,"Repeated an already applied update"
print("P8CU_OFFICIAL_PLUGIN_VERSION_POSTCONDITION_AND_ONE_SHOT_UPDATE_PASS")
