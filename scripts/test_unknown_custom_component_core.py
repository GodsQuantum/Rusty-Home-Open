#!/usr/bin/env python3
"""Genuine pinned Home Assistant Core tests a never-before-seen Python component.

Executed ONLY inside one-off Docker --network none on synthetic /config.
No imports or configuration from production HAOS, no GitHub/OAuth/real devices.
"""
import asyncio
import json
from pathlib import Path

from homeassistant.core import HomeAssistant
from homeassistant import loader
from homeassistant.setup import async_setup_component
from homeassistant.config_entries import ConfigEntries

async def main():
    network=Path("/proc/self/net/dev").read_text().splitlines()[2:]
    interfaces=[row.split(":",1)[0].strip() for row in network if ":" in row]
    assert interfaces==["lo"], f"unexpected network devices {interfaces}"
    routes=Path("/proc/net/route").read_text().splitlines()[1:]
    assert not any(r.strip() for r in routes), "external route in HACS compatibility lab"
    hass=HomeAssistant("/config")
    loader.async_setup(hass)
    hass.config_entries=ConfigEntries(hass,{"hars_future_probe":{}})
    result=False
    try:
        await hass.async_start()
        result=await asyncio.wait_for(async_setup_component(
            hass, "hars_future_probe", {"hars_future_probe": {}}), timeout=30)
        assert result, "official Core loader did not initialize unfamiliar custom integration"
        state=hass.states.get("sensor.hars_future_probe")
        assert state is not None and state.state=="4242", "Core state machine missing synthetic state"
        assert state.attributes.get("hars_synthetic") is True
        assert hass.services.has_service("hars_future_probe","hello"), "synthetic service not registered"
        print(json.dumps({
            "genuine_homeassistant_core": True,
            "arbitrary_custom_integration_loaded": True,
            "native_core_state_observed": True,
            "unknown_custom_service_registered": True,
            "isolated_loopback_only": True,
            "python_sidecar_required": True,
            "all_hacs_certified": False,
            "production_ready": False,
        }))
    finally:
        await hass.async_stop()
asyncio.run(main())
