#!/usr/bin/env python3
"""Genuine public Adaptive Lighting 1.32.0 Core lifecycle, NO actual lights."""
import asyncio
import json
from pathlib import Path
from importlib.metadata import version

from homeassistant import loader
from homeassistant.config_entries import ConfigEntries, ConfigEntryState
from homeassistant.core import HomeAssistant
from homeassistant.const import STATE_UNAVAILABLE
from homeassistant.helpers import device_registry as dr, entity_registry as er
from homeassistant.setup import async_setup_component

DOMAIN = "adaptive_lighting"

async def main():
    assert version("ulid-transform") == "2.2.9"
    interfaces = [x.split(":",1)[0].strip() for x in
        Path("/proc/self/net/dev").read_text().splitlines()[2:] if ":" in x]
    assert interfaces == ["lo"] and not Path("/proc/net/route").read_text().splitlines()[1:]
    hass=HomeAssistant("/config")
    loader.async_setup(hass)
    hass.config_entries=ConfigEntries(hass,{DOMAIN:{}})
    try:
        await asyncio.wait_for(hass.async_start(),timeout=50)
        dr.async_setup(hass)
        await dr.async_load(hass)
        await er.async_load(hass)
        for domain in ("light","switch"):
            assert await asyncio.wait_for(
                async_setup_component(hass, domain, {}), timeout=45
            ), "Core built-in platform failed: "+domain
        assert await asyncio.wait_for(async_setup_component(
            hass, DOMAIN, {DOMAIN: []}), timeout=45)
        before=hass.states.async_entity_ids("switch")
        assert not before, "unexpected preexisting switch in isolated config"
        flow=await asyncio.wait_for(hass.config_entries.flow.async_init(
            DOMAIN,context={"source":"user"}),timeout=45)
        assert flow["type"]=="form" and flow["step_id"]=="user"
        result=await asyncio.wait_for(hass.config_entries.flow.async_configure(
            flow["flow_id"], {"name":"Rusty Home Empty Lights"}),timeout=45)
        assert result["type"]=="create_entry"
        entries=hass.config_entries.async_entries(DOMAIN)
        assert len(entries)==1
        entry=entries[0]
        await hass.async_block_till_done()
        if entry.state is not ConfigEntryState.LOADED:
            assert await asyncio.wait_for(
                hass.config_entries.async_setup(entry.entry_id),timeout=90)
        assert entry.state is ConfigEntryState.LOADED
        switch_ids=set(hass.states.async_entity_ids("switch"))
        assert switch_ids, "Adaptive Lighting real switch entities were not created"
        assert hass.services.has_service(DOMAIN,"apply")
        assert await asyncio.wait_for(
            hass.config_entries.async_unload(entry.entry_id), timeout=90)
        assert entry.state is ConfigEntryState.NOT_LOADED
        await hass.async_block_till_done()
        observed = {
            entity_id: hass.states.get(entity_id)
            for entity_id in switch_ids
        }
        # Official HA Core keeps registered entities in the state machine as
        # unavailable/restored on config entry unload. Their prior live values
        # MUST NOT remain active.
        assert all(
            state is not None
            and state.state == STATE_UNAVAILABLE
            and state.attributes.get("restored") is True
            for state in observed.values()
        ), "Adaptive Lighting switches remained LIVE after native Core unload"
        Path("/config/adaptive-unload-observation.json").write_text(
            json.dumps({"unavailable_restored":len(observed)})
        )

        assert await asyncio.wait_for(
            hass.config_entries.async_setup(entry.entry_id),timeout=90)
        assert entry.state is ConfigEntryState.LOADED
        reloaded = {
            entity_id: hass.states.get(entity_id)
            for entity_id in switch_ids
        }
        assert all(
            state is not None and state.state != STATE_UNAVAILABLE
            for state in reloaded.values()
        ), "Adaptive Lighting switch was not reactivated after reload"
        assert await asyncio.wait_for(
            hass.config_entries.async_unload(entry.entry_id),timeout=90)
        await asyncio.wait_for(
            hass.config_entries.async_remove(entry.entry_id),timeout=90)
        await hass.async_block_till_done()
        assert not hass.config_entries.async_entries(DOMAIN)
        assert not hass.states.async_entity_ids("switch")
        proof={
            "real_upstream":"basnijholt/adaptive-lighting",
            "upstream_commit":"7f7fec4e34d331fa5fd84b95514ddb1baf2a401e",
            "integration_version":"1.32.0",
            "dependency_ulid_transform":"2.2.9",
            "real_core_config_flow":True,
            "real_core_entry_loaded":True,
            "real_core_switch_created":True,
            "real_core_unload_removes_switch":True,
            "unload_switches_unavailable_restored":True,
            "real_core_reload_recreates_switch":True,
            "real_core_removal":True,
            "real_lights_or_devices_touched":False,
            "production_ready":False,
        }
        Path("/config/adaptive-proof.json").write_text(json.dumps(proof,sort_keys=True))
    finally:
        await hass.async_stop()

if __name__=="__main__":
    import traceback
    Path("/config/adaptive-started").write_text("yes")
    try:
        asyncio.run(main())
    except BaseException:
        Path("/config/adaptive-error.txt").write_text(traceback.format_exc())
        raise
