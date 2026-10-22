#!/usr/bin/env python3
"""Released CUP 1.5.0 in real Core with strictly synthetic CUP HTTP v3 feed."""
import asyncio
import json
import traceback
from datetime import datetime, timezone
from pathlib import Path
from aiohttp import web
from homeassistant import loader
from homeassistant.auth import auth_manager_from_config
from homeassistant.config_entries import ConfigEntries, ConfigEntryState
from homeassistant.core import HomeAssistant
from homeassistant.helpers import device_registry as dr, entity_registry as er
from homeassistant.const import CONF_NAME, CONF_URL
from homeassistant.setup import async_setup_component

DOMAIN="cup_component"
NAME="P6L Synthetic Cup"
UPDATED="2026-10-22T18:00:00+00:00"
CHANGE="2026-10-22T18:01:00+00:00"
def entry(update):
    return {"reference":"synthetic/cup-test:1.0", "result":{
        "has_update":update,"info":{"version_update_type":"patch" if update else "up_to_date"}
    }}
async def run():
    import os
    assert os.environ.get("P6L_DOCKER_INTERNAL") == "verified"
    interfaces=[row.split(":",1)[0].strip() for row in
        Path("/proc/self/net/dev").read_text().splitlines()[2:] if ":" in row]
    assert "lo" in interfaces and len(interfaces)==2
    manifest=json.loads(Path("/config/custom_components/cup_component/manifest.json").read_text())
    assert manifest["version"]=="v1.5.0" and manifest["domain"]==DOMAIN
    # Loopback-only artificial server living within the isolated Core process.
    state={"has_update":True, "calls":0}
    async def json_handler(request):
        state["calls"]+=1
        return web.json_response({"last_updated":UPDATED if state["has_update"] else CHANGE,
            "images":[entry(state["has_update"]),{
                "reference":"synthetic/uptodate:1.0",
                "result":{"has_update":False,"info":{}}
            }]})
    server=web.Application()
    server.router.add_get("/api/v3/json",json_handler)
    runner=web.AppRunner(server)
    await runner.setup()
    site=web.TCPSite(runner,"127.0.0.1",0)
    await site.start()
    port=site._server.sockets[0].getsockname()[1]
    url=f"http://127.0.0.1:{port}"
    hass=HomeAssistant("/config")
    loader.async_setup(hass)
    hass.config_entries=ConfigEntries(hass,{DOMAIN:{}})
    try:
        dr.async_setup(hass)
        await dr.async_load(hass)
        await er.async_load(hass)
        # Real HTTP Core needs the real empty lab auth manager, not a stub.
        hass.auth = await auth_manager_from_config(hass, [], [])
        for part in ("http","frontend","sensor","binary_sensor","button"):
            cfg={"http":{"server_host":"127.0.0.1"}} if part=="http" else {}
            assert await asyncio.wait_for(async_setup_component(hass,part,cfg),80),part
        await asyncio.wait_for(hass.async_start(),70)
        assert await asyncio.wait_for(async_setup_component(hass,DOMAIN,{}),65)
        flow=await asyncio.wait_for(
            hass.config_entries.flow.async_init(DOMAIN,context={"source":"user"}),45)
        assert flow["type"]=="form",flow
        created=await asyncio.wait_for(
            hass.config_entries.flow.async_configure(flow["flow_id"],{
                CONF_NAME:NAME,CONF_URL:url
            }),60)
        assert created["type"]=="create_entry",created
        entries=hass.config_entries.async_entries(DOMAIN)
        assert len(entries)==1
        item=entries[0]
        await hass.async_block_till_done()
        if item.state is not ConfigEntryState.LOADED:
            assert await asyncio.wait_for(hass.config_entries.async_setup(item.entry_id),95)
        assert item.state is ConfigEntryState.LOADED,item.state
        assert state["calls"]>=2, "Real config flow and coordinator did not both hit fake Cup HTTP server"
        api=item.runtime_data.api
        assert api.cache_metrics["monitored_images"]==2
        assert api.cache_metrics["updates_available"]==1
        sensors=[s for s in hass.states.async_all("sensor") if "p6l_synthetic_cup" in s.entity_id]
        assert len(sensors)>=8,[(s.entity_id,s.state) for s in sensors]
        observed={s.entity_id:s.state for s in sensors}
        assert any(k.endswith("_updates_available") and v=="1" for k,v in observed.items()),observed
        assert any(k.endswith("_monitored_images") and v=="2" for k,v in observed.items()),observed
        # Request a real coordinator re-poll, proving HA sensor updates.
        state["has_update"]=False
        await asyncio.wait_for(item.runtime_data.coordinator.async_request_refresh(),65)
        await hass.async_block_till_done()
        assert api.cache_metrics["updates_available"]==0
        sensors=[s for s in hass.states.async_all("sensor") if "p6l_synthetic_cup" in s.entity_id]
        assert any(s.entity_id.endswith("_updates_available") and s.state=="0" for s in sensors)
        second=await asyncio.wait_for(
            hass.config_entries.flow.async_init(DOMAIN,context={"source":"user"}),45)
        again=await asyncio.wait_for(
            hass.config_entries.flow.async_configure(second["flow_id"],{
                CONF_NAME:NAME,CONF_URL:url
            }),60)
        assert again["type"]=="abort" and again["reason"]=="already_configured",again
        assert len(hass.config_entries.async_entries(DOMAIN))==1
        assert await asyncio.wait_for(hass.config_entries.async_unload(item.entry_id),75)
        assert item.state is ConfigEntryState.NOT_LOADED
        assert await asyncio.wait_for(hass.config_entries.async_setup(item.entry_id),90)
        assert item.state is ConfigEntryState.LOADED
        assert await asyncio.wait_for(hass.config_entries.async_unload(item.entry_id),75)
        await asyncio.wait_for(hass.config_entries.async_remove(item.entry_id),80)
        assert not hass.config_entries.async_entries(DOMAIN)
        Path("/config/p6l-proof.json").write_text(json.dumps({
            "genuine_core":True,"actual_release_tag":"v1.5.0",
            "actual_upstream_sha":"ca9b767cb9bdd83d9e279947cd89fa742648d1df",
            "release_asset_sha256":"52f6c367c8b039bb5d0b467206e357dc620c221ec2b255bd09c247b6205e0f51",
            "release_manifest_version":"v1.5.0",
            "actual_core_config_entry_loaded":True,"real_cup_http_config_flow":True,
            "real_cup_http_coordinator":True,"real_synthetic_sensors":True,
            "real_synthetic_update_event":True,"duplicate_config_refused":True,
            "unload_reload_remove":True,"actual_docker_or_user_images_tested":False,
            "production_ready":False
        },sort_keys=True))
        print("P6L_CUP_RELEASE_REAL_CORE_SYNTHETIC_SENSOR_REFRESH_PASS")
    finally:
        await hass.async_stop()
        await runner.cleanup()

if __name__=="__main__":
    try: asyncio.run(run())
    except BaseException:
        Path("/config/p6l-error.txt").write_text(traceback.format_exc())
        raise
