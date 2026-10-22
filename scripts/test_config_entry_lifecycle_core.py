#!/usr/bin/env python3
"""Genuine pinned Home Assistant Core config-flow+entry lifecycle proof.

Only synthetic files under the disposable /config. No network or user devices.
Executed INSIDE the unprivileged, network-none official Home Assistant container.
"""
import asyncio
import json
from pathlib import Path

from homeassistant import loader
from homeassistant.config_entries import ConfigEntries, ConfigEntryState
from homeassistant.core import HomeAssistant
from homeassistant.setup import async_setup_component
from homeassistant.helpers import device_registry as dr, entity_registry as er

DOMAIN = "hars_lifecycle_probe"
ENTITY = "sensor.hars_lifecycle_probe"

async def main():
    iface = [r.split(":", 1)[0].strip() for r in Path("/proc/self/net/dev").read_text().splitlines()[2:] if ":" in r]
    assert iface == ["lo"], "non-loopback interface detected"
    assert not Path("/proc/net/route").read_text().splitlines()[1:], "unexpected external route"

    hass = HomeAssistant("/config")
    loader.async_setup(hass)
    hass.config_entries = ConfigEntries(hass, {DOMAIN: {}})
    try:
        await asyncio.wait_for(hass.async_start(), timeout=45)
        # The minimal Python-Core boot harness deliberately does not start
        # the complete HA supervisor; config-entry deletion nevertheless uses
        # BOTH genuine device and entity registries for cleanup.
        dr.async_setup(hass)
        await asyncio.wait_for(dr.async_load(hass), timeout=30)
        await asyncio.wait_for(er.async_load(hass), timeout=30)
        assert await asyncio.wait_for(
            async_setup_component(hass, DOMAIN, {DOMAIN: {}}), timeout=45
        ), "official Core failed synthetic integration initial setup"
        assert hass.states.get(ENTITY) is None, "fixture created state before entry"
        flow = await asyncio.wait_for(
            hass.config_entries.flow.async_init(DOMAIN, context={"source": "user"}),
            timeout=45,
        )
        assert flow["type"] == "form", "real HA config_flow not presented"
        assert flow["step_id"] == "user"
        completed = await asyncio.wait_for(
            hass.config_entries.flow.async_configure(
                flow["flow_id"], {"label": "fictional_test"}
            ),
            timeout=45,
        )
        assert completed["type"] == "create_entry", "config_flow did not create entry"
        entry = hass.config_entries.async_entries(DOMAIN)
        assert len(entry) == 1, "unexpected number of Core entries"
        entry = entry[0]
        await hass.async_block_till_done()
        if entry.state is not ConfigEntryState.LOADED:
            assert await asyncio.wait_for(
                hass.config_entries.async_setup(entry.entry_id), timeout=45
            ), "official Core manager failed to load new entry"
        assert entry.state is ConfigEntryState.LOADED, "Core config entry not loaded"
        state = hass.states.get(ENTITY)
        assert state is not None and state.state == "4242"
        assert state.attributes.get("synthetic") is True
        assert hass.data[DOMAIN]["setup_count"] >= 1
        duplicate = await asyncio.wait_for(
            hass.config_entries.flow.async_init(
                DOMAIN, context={"source": "user"}, data={"label": "fictional_test"}
            ), timeout=45
        )
        assert duplicate["type"] == "abort" and duplicate["reason"] == "already_configured", (
            "Core did not reject a duplicate synthetic config entry"
        )
        assert len(hass.config_entries.async_entries(DOMAIN)) == 1

        assert await asyncio.wait_for(
            hass.config_entries.async_unload(entry.entry_id), timeout=45
        ), "Core config-entry unload failed"
        assert entry.state is ConfigEntryState.NOT_LOADED
        assert hass.states.get(ENTITY) is None, "stale state remained after unload"
        assert hass.data[DOMAIN]["unload_count"] >= 1

        assert await asyncio.wait_for(
            hass.config_entries.async_setup(entry.entry_id), timeout=45
        ), "Core config-entry reload setup failed"
        assert entry.state is ConfigEntryState.LOADED
        assert hass.states.get(ENTITY).state == "4242"
        assert hass.data[DOMAIN]["setup_count"] >= 2

        await asyncio.wait_for(
            hass.config_entries.async_remove(entry.entry_id), timeout=45
        )
        assert not hass.config_entries.async_entries(DOMAIN), "Core entry not removed"
        assert hass.states.get(ENTITY) is None, "stale entity after removing entry"
        assert hass.data[DOMAIN]["remove_count"] == 1, "remove hook not invoked"

        proof = {
            "real_core_config_flow": True,
            "real_core_config_entry_created": True,
            "real_core_entry_loaded": True,
            "real_core_unload_state_removed": True,
            "real_core_reload_state_restored": True,
            "real_core_remove_hook": True,
            "offline_loopback_only": True,
            "synthetic_integration_count": 1,
            "actual_29_hacs_integrations_certified": 0,
            "duplicate_config_entry_denied": True,
            "production_ready": False,
        }
        Path("/config/p6c-proof.json").write_text(
            json.dumps(proof, sort_keys=True), encoding="utf-8"
        )
        print(json.dumps(proof, sort_keys=True))
    finally:
        await hass.async_stop()

if __name__ == "__main__":
    import traceback
    Path("/config/p6c_launch").write_text("launched", encoding="utf-8")
    try:
        asyncio.run(main())
    except BaseException:
        Path("/config/p6c_debug.log").write_text(
            traceback.format_exc(), encoding="utf-8"
        )
        raise
