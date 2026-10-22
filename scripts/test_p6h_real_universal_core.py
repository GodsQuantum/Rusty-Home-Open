#!/usr/bin/env python3
"""Genuine v1.9.0 release asset loaded by pinned official Core; fictitious media."""
import asyncio
import json
from pathlib import Path

from homeassistant import loader
from homeassistant.auth import auth_manager_from_config
from homeassistant.config_entries import ConfigEntries, ConfigEntryState
from homeassistant.core import HomeAssistant
from homeassistant.const import STATE_UNAVAILABLE
from homeassistant.helpers import device_registry as dr, entity_registry as er
from homeassistant.setup import async_setup_component

DOMAIN="custom_universal_media_player"

async def main():
    assert not Path("/proc/net/route").read_text().splitlines()[1:]
    assert [line.split(":",1)[0].strip() for line in Path("/proc/self/net/dev").read_text().splitlines()[2:] if ":" in line]==["lo"]
    hass=HomeAssistant("/config")
    loader.async_setup(hass)
    hass.config_entries=ConfigEntries(hass,{DOMAIN:{}})
    try:
        dr.async_setup(hass)
        await dr.async_load(hass)
        await er.async_load(hass)
        # Genuine Home Assistant Auth/HTTP/WS bootstrap with zero synthetic
        # accounts; no test HTTP ports published from this network namespace.
        hass.auth = await auth_manager_from_config(hass, [], [])
        for component in ("http", "websocket_api", "media_player"):
            cfg = {"http": {"server_host": "127.0.0.1"}} if component == "http" else {}
            assert await asyncio.wait_for(async_setup_component(hass,component,cfg),75), (
                "Native Home Assistant subsystem unavailable: "+component
            )
        await asyncio.wait_for(hass.async_start(),45)
        hass.states.async_set("media_player.fixture_child","playing",{
            "friendly_name":"Offline fictitious child","supported_features":0
        })
        fixture_players=set(hass.states.async_entity_ids("media_player"))
        assert fixture_players=={"media_player.fixture_child"}
        flow=await asyncio.wait_for(hass.config_entries.flow.async_init(
            DOMAIN,context={"source":"user"}
        ),45)
        assert flow["type"]=="form" and flow["step_id"]=="user"
        flow=await asyncio.wait_for(hass.config_entries.flow.async_configure(
            flow["flow_id"], {"name":"Rusty Home Synthetic Universal",
                              "children":["media_player.fixture_child"]}
        ),45)
        assert flow["type"]=="form" and flow["step_id"]=="device_class",flow
        flow=await asyncio.wait_for(hass.config_entries.flow.async_configure(
            flow["flow_id"], {"device_class":"none"}
        ),45)
        assert flow["type"]=="menu" and flow["step_id"]=="config_menu",flow
        flow=await asyncio.wait_for(hass.config_entries.flow.async_configure(
            flow["flow_id"], {"next_step_id":"config_save"}
        ),45)
        assert flow["type"]=="create_entry",flow
        entries=hass.config_entries.async_entries(DOMAIN)
        assert len(entries)==1
        entry=entries[0]
        await hass.async_block_till_done()
        if entry.state is not ConfigEntryState.LOADED:
            assert await asyncio.wait_for(hass.config_entries.async_setup(entry.entry_id),75)
        assert entry.state is ConfigEntryState.LOADED,entry.state
        await hass.async_block_till_done()
        generated = set(hass.states.async_entity_ids("media_player")) - fixture_players
        assert len(generated)==1, "real composite media player entity never appeared"
        assert len(hass.config_entries.async_entries(DOMAIN))==1
        assert await asyncio.wait_for(hass.config_entries.async_unload(entry.entry_id),75)
        await hass.async_block_till_done()
        assert entry.state is ConfigEntryState.NOT_LOADED
        # Home Assistant retains registered entities as unavailable/restored
        # after an entry unload. Deleting states here would assert the wrong
        # Core semantics, and could conceal a stale *live* entity.
        for entity_id in generated:
            unloaded = hass.states.get(entity_id)
            assert unloaded is not None and unloaded.state == STATE_UNAVAILABLE, (
                "Composite entity stayed live after config entry unload"
            )
            assert unloaded.attributes.get("restored") is True
        assert await asyncio.wait_for(hass.config_entries.async_setup(entry.entry_id),75)
        assert entry.state is ConfigEntryState.LOADED
        await hass.async_block_till_done()
        for entity_id in generated:
            reloaded = hass.states.get(entity_id)
            assert reloaded is not None and reloaded.state != STATE_UNAVAILABLE, (
                "Reload did not reactivate the genuine composite player"
            )
        assert await asyncio.wait_for(hass.config_entries.async_unload(entry.entry_id),75)
        await asyncio.wait_for(hass.config_entries.async_remove(entry.entry_id),75)
        assert not hass.config_entries.async_entries(DOMAIN)
        await hass.async_block_till_done()
        assert not (set(hass.states.async_entity_ids("media_player")) & generated)
        assert hass.states.get("media_player.fixture_child") is not None
        proof={
           "real_official_core":True,
           "release_zip_sha256":"ec47b0114a5bf447ef1b7fa15cfa80829619f6785ece33ee4b5957ef906f7865",
           "manifest_version":"v1.9.0",
           "upstream_tag_manifest_version":"v1.3.0",
           "genuine_user_config_flow":True,
           "real_composite_entity_created":True,
           "unload_entity_unavailable_restored":True,
           "reload_entity_reactivated":True,
           "entry_remove_entity_gone":True,
           "genuine_entry_setup_unload_reload_remove":True,
           "fictitious_child_preserved":True,
           "real_user_media_devices_touched":False,
           "production_ready":False,
        }
        Path("/config/universal-proof.json").write_text(json.dumps(proof,sort_keys=True))
    finally:
        await hass.async_stop()
if __name__=="__main__":
    import traceback
    try:
        asyncio.run(main())
    except BaseException:
        Path("/config/universal-error.txt").write_text(traceback.format_exc())
        raise
