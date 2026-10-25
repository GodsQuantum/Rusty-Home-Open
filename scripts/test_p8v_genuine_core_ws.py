#!/usr/bin/env python3
"""Genuine official Home Assistant Core WebSocket -> Rust lab, disposable only.

Runs INSIDE pinned Core image on Docker --internal bridge, no external egress.
Uses genuine config entries, entity registry, HA auth, HTTP and WS components.
Never reads household data or writes any physical device/service.
"""
import asyncio
import json
import os
from pathlib import Path
import traceback

from homeassistant import loader
from homeassistant.auth import auth_manager_from_config
from homeassistant.auth.const import GROUP_ID_ADMIN
from homeassistant.config_entries import ConfigEntries
from homeassistant.core import HomeAssistant
from homeassistant.helpers import device_registry as dr, entity_registry as er
from homeassistant.setup import async_setup_component

DOMAIN = "hars_lifecycle_probe"
ROOT=Path("/config")

async def main():
    assert os.environ.get("P8V_NETWORK_NONE")=="verified"
    ifaces=[r.split(":",1)[0].strip() for r in Path("/proc/self/net/dev").read_text().splitlines()[2:] if ":" in r]
    assert ifaces==["lo"]
    assert not Path("/proc/net/route").read_text().splitlines()[1:]
    assert ROOT.joinpath("custom_components",DOMAIN,"manifest.json").is_file()
    hass=HomeAssistant("/config")
    loader.async_setup(hass)
    hass.config_entries=ConfigEntries(hass,{DOMAIN:{}})
    try:
        dr.async_setup(hass)
        await asyncio.wait_for(dr.async_load(hass),45)
        await asyncio.wait_for(er.async_load(hass),45)
        hass.auth=await auth_manager_from_config(hass,[],[])
        for name in ("http","websocket_api"):
            cfg={"http":{"server_host":"0.0.0.0","server_port":8123}} if name=="http" else {}
            assert await asyncio.wait_for(async_setup_component(hass,name,cfg),60),name
        await asyncio.wait_for(hass.async_start(),60)
        assert await asyncio.wait_for(async_setup_component(hass,DOMAIN,{DOMAIN:{}}),60)
        flow=await hass.config_entries.flow.async_init(DOMAIN,context={"source":"user"})
        assert flow["type"]=="form"
        created=await hass.config_entries.flow.async_configure(
            flow["flow_id"],{"label":"p8v_fictional"})
        assert created["type"]=="create_entry"
        entries=hass.config_entries.async_entries(DOMAIN)
        assert len(entries)==1
        entry=entries[0]
        await hass.async_block_till_done()
        registry=er.async_get(hass)
        entity=registry.async_get_or_create("sensor",DOMAIN,
            unique_id="p8v_synthetic_serial",config_entry=entry,
            suggested_object_id="official_ws_probe")
        assert entity.config_entry_id==entry.entry_id
        eid=entity.entity_id
        hass.states.async_set(eid,"off",{"synthetic":True,"seq":0})
        owner=await hass.auth.async_create_user("p8v-fictional-owner",group_ids=[GROUP_ID_ADMIN])
        refresh=await hass.auth.async_create_refresh_token(owner,
            client_id="http://localhost/p8v-offline-only")
        access=hass.auth.async_create_access_token(refresh)
        assert isinstance(access,str) and len(access)>16
        # Genuine Core Registry Storage schedules a save; do not forge format.
        await asyncio.sleep(12)
        saved=json.loads(ROOT.joinpath(".storage/core.entity_registry").read_bytes())
        assert saved["key"]=="core.entity_registry"
        assert any(row.get("unique_id")=="p8v_synthetic_serial"
                   and row.get("config_entry_id")==entry.entry_id
                   for row in saved["data"]["entities"])
        for name,value in (("p8v-token",access),("p8v-entity-id",eid)):
            file=ROOT/name
            file.write_text(value,encoding="utf-8")
            file.chmod(0o600)
        (ROOT/"p8v-ready").write_text("ready",encoding="utf-8")
        print("P8V_REAL_CORE_HTTP_WEBSOCKET_REGISTRY_AND_AUTH_READY",flush=True)
        for i in range(1,110):
            if (ROOT/"p8v-stop").exists(): break
            # A repeated on-state with increasing attributes ensures genuine
            # state_changed events regardless of when Rust subscribes.
            hass.states.async_set(eid,"on",{"synthetic":True,"seq":i})
            await asyncio.sleep(0.2)
        print("P8V_REAL_CORE_WS_FIXTURE_EXIT",flush=True)
    finally:
        await hass.async_stop()

if __name__=="__main__":
    try:
        asyncio.run(main())
    except BaseException:
        (ROOT/"p8v-error.txt").write_text(traceback.format_exc(),encoding="utf-8")
        raise
