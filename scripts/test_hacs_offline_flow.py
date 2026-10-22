"""Real HACS 2.0.5 initial config flow inside isolated real HA Core image.
Never initiates GitHub Device OAuth or touches production HAOS."""
from __future__ import annotations
import asyncio
from pathlib import Path
from types import SimpleNamespace
from homeassistant.data_entry_flow import FlowResultType
from custom_components.hacs.config_flow import HacsFlowHandler

async def test():
    flow=HacsFlowHandler()
    flow.hass=SimpleNamespace(data={})
    flow._async_current_entries=lambda: []
    initial=await flow.async_step_user(None)
    assert initial["type"]==FlowResultType.FORM,initial
    assert initial["step_id"]=="user",initial
    schema=initial["data_schema"]
    keys={str(x.schema) for x in schema.schema}
    assert keys=={"acc_logs","acc_addons","acc_untested","acc_disable"},keys
    print("PASS: authentic HACS ConfigFlow initial consent form (4 flags)")
    rejected=await flow.async_step_user({
        "acc_logs":False,"acc_addons":True,
        "acc_untested":True,"acc_disable":True,
    })
    assert rejected["type"]==FlowResultType.FORM,rejected
    assert rejected["errors"].get("base")=="acc",rejected
    print("PASS: HACS consent validation rejects a disabled required acknowledgement")
    base=Path("/config/custom_components/hacs/hacs_frontend")
    assert (base/"__init__.py").is_file()
    count=sum(1 for x in base.rglob("*") if x.is_file())
    assert count>100, count
    print(f"PASS: genuine HACS frontend packaged assets present ({count} files)")
    # Go through HACS' *real* device OAuth flow logic, substituting a local
    # deterministic fake GitHub API. No HTTP and no real user authorization.
    from unittest.mock import AsyncMock, patch
    from custom_components.hacs import config_flow as hacs_flow

    class FakeGitHubDeviceAPI:
        def __init__(self, **_kwargs):
            pass
        async def register(self):
            return SimpleNamespace(data=SimpleNamespace(
                device_code="synthetic-device-code",
                user_code="TEST-0000",
            ))
        async def activation(self, **_kwargs):
            return SimpleNamespace(data=SimpleNamespace(access_token="fake-never-exported"))
    oauth=hacs_flow.HacsFlowHandler()
    oauth.flow_id="mock-hacs-flow"
    oauth.hass=SimpleNamespace(
        data={},
        async_create_task=lambda coro: asyncio.create_task(coro),
        config_entries=SimpleNamespace(flow=SimpleNamespace(async_configure=AsyncMock())),
    )
    oauth._async_current_entries=lambda: []
    with patch.object(hacs_flow,"GitHubDeviceAPI",FakeGitHubDeviceAPI), (
        patch.object(hacs_flow,"async_get_integration",new=AsyncMock(
            return_value=SimpleNamespace(version="2.0.5")
        ))
    ), patch.object(hacs_flow.aiohttp_client,"async_get_clientsession",return_value=None):
        result=await oauth.async_step_user({
            "acc_logs":True,"acc_addons":True,
            "acc_untested":True,"acc_disable":True,
        })
        assert result["step_id"]=="device",result
        assert result["description_placeholders"]["code"]=="TEST-0000",result
        await asyncio.wait_for(oauth.activation_task,timeout=3)
        assert oauth._activation.access_token=="fake-never-exported"
        result=await oauth.async_step_device({})
        assert result.get("step_id")=="device_done" or result.get("next_step_id")=="device_done",{"keys":sorted(result)}
    print("PASS: real HACS device OAuth step with offline fake API, zero GitHub/network")
    print("NOT TESTED: genuine GitHub authorization, configured HACS account or repo installs")


asyncio.run(test())
