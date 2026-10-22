#!/usr/bin/env python3
"""Authentic Smart Announcements 0.8.3 UI flow in pinned HA Core offline."""
import asyncio
import json
import traceback
from pathlib import Path

from homeassistant import loader
from homeassistant.auth import auth_manager_from_config
from homeassistant.core import HomeAssistant
from homeassistant.config_entries import ConfigEntries, ConfigEntryState
from homeassistant.helpers import device_registry as dr, entity_registry as er, area_registry as ar
from homeassistant.setup import async_setup_component

DOMAIN = "smart_announcements"

async def main():
    # Docker ephemeral --internal bridge permits mDNS socket binding but no
    # external route or published ports. Runner verifies Internal=true.
    assert __import__("os").environ.get("P6F_INTERNAL_BRIDGE") == "verified"
    ifaces=[row.split(":",1)[0].strip() for row in Path("/proc/self/net/dev").read_text().splitlines()[2:] if ":" in row]
    assert "lo" in ifaces and len(ifaces)==2
    hass=HomeAssistant("/config")
    loader.async_setup(hass)
    hass.config_entries=ConfigEntries(hass,{DOMAIN:{}})
    try:
        await asyncio.wait_for(hass.async_start(),timeout=45)
        dr.async_setup(hass)
        await dr.async_load(hass)
        await er.async_load(hass)
        hass.auth=await auth_manager_from_config(hass,[],[])
        await ar.async_load(hass)
        # Initialise official WebSocket/API and HTTP prerequisites because
        # media_player registers real native routes. The entire test container
        # has Docker network=none and no published ports.
        for name in ("http","websocket_api","switch","media_player","tts"):
            assert await asyncio.wait_for(
                async_setup_component(hass,name,{}),45
            ), "Core native platform unavailable: "+name

        flow=await asyncio.wait_for(hass.config_entries.flow.async_init(DOMAIN,context={"source":"user"}),45)
        assert flow["type"]=="form" and flow["step_id"]=="user"
        assert flow["errors"].get("base")=="no_persons",flow

        hass.states.async_set("person.fictional", "home", {"friendly_name":"Synthetic Person"})
        flow=await hass.config_entries.flow.async_configure(flow["flow_id"],{})
        assert flow["type"]=="form" and flow["errors"].get("base")=="no_areas",flow
        area=ar.async_get(hass).async_create("Synthetic Room")
        hass.states.async_set("media_player.fictional", "idle",{"friendly_name":"Synthetic Player"})
        flow=await hass.config_entries.flow.async_configure(flow["flow_id"],{})
        assert flow["step_id"]=="people_select",flow
        flow=await hass.config_entries.flow.async_configure(
            flow["flow_id"],{"selected_people":["person.fictional"]}
        )
        assert flow["step_id"]=="person_config",flow
        flow=await hass.config_entries.flow.async_configure(
            flow["flow_id"],{"language":"english","enhance_with_ai":False,"translate_announcement":False}
        )
        assert flow["step_id"]=="person_voice",flow
        flow=await hass.config_entries.flow.async_configure(flow["flow_id"],{})
        assert flow["step_id"]=="room_tracking",flow
        flow=await hass.config_entries.flow.async_configure(
            flow["flow_id"],{"room_tracking":False,"presence_verification":False}
        )
        assert flow["step_id"]=="rooms_select",flow
        flow=await hass.config_entries.flow.async_configure(flow["flow_id"],{"selected_rooms":[area.id]})
        assert flow["step_id"]=="room_config",flow
        flow=await hass.config_entries.flow.async_configure(
            flow["flow_id"],{"media_player":"media_player.fictional"}
        )
        assert flow["step_id"]=="pre_announce",flow
        flow=await hass.config_entries.flow.async_configure(
            flow["flow_id"],{"pre_announce_enabled":False}
        )
        assert flow["type"]=="create_entry",flow
        entries=hass.config_entries.async_entries(DOMAIN)
        assert len(entries)==1
        entry=entries[0]
        await asyncio.wait_for(hass.async_block_till_done(),60)
        if entry.state is not ConfigEntryState.LOADED:
            assert await asyncio.wait_for(hass.config_entries.async_setup(entry.entry_id),75)
        assert entry.state is ConfigEntryState.LOADED,entry.state
        assert hass.services.has_service(DOMAIN,"announce")
        assert len(entry.data["people"])==1 and len(entry.data["rooms"])==1
        dup=await hass.config_entries.flow.async_init(DOMAIN,context={"source":"user"})
        assert dup["type"]=="abort" and dup["reason"]=="already_configured",dup
        assert await hass.config_entries.async_unload(entry.entry_id)
        assert entry.state is ConfigEntryState.NOT_LOADED
        assert not hass.services.has_service(DOMAIN,"announce")
        assert await hass.config_entries.async_setup(entry.entry_id)
        assert entry.state is ConfigEntryState.LOADED
        assert hass.services.has_service(DOMAIN,"announce")
        assert await hass.config_entries.async_unload(entry.entry_id)
        await hass.config_entries.async_remove(entry.entry_id)
        assert not hass.config_entries.async_entries(DOMAIN)
        assert not hass.services.has_service(DOMAIN,"announce")
        proof={
           "upstream":"mike-nott/smart-announcements",
           "version":"0.8.3",
           "commit":"e5a843be497e5591233896035a4a20fd60e939bb",
           "core_real_config_flow":True,
           "required_inputs_validated":True,
           "fake_person_and_area":True,
           "core_loaded":True,
           "core_unloaded_and_reloaded":True,
           "core_removed":True,
           "duplicate_rejected":True,
           "synthetic_tts_not_executed":True,
           "real_user_devices_tested":False,
           "production_ready":False,
        }
        Path("/config/smart-proof.json").write_text(json.dumps(proof,sort_keys=True),encoding="utf-8")
        print("P6F_REAL_SMART_ANNOUNCEMENTS_CORE_LIFECYCLE_PASS")
    finally:
        await hass.async_stop()

if __name__=="__main__":
    try:
        asyncio.run(main())
    except BaseException:
        Path("/config/smart-traceback.txt").write_text(traceback.format_exc(),encoding="utf-8")
        raise
