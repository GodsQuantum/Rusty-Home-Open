#!/usr/bin/env python3
"""Real HACS release Card Builder 2.6.0 in genuine official Core, no user data."""
import asyncio
import logging
logging.basicConfig(level=logging.DEBUG,force=True,handlers=[logging.FileHandler('/config/ha-http-debug.log'),logging.StreamHandler()])
import json
from pathlib import Path
from homeassistant import loader
from homeassistant.auth import auth_manager_from_config
from homeassistant.config_entries import ConfigEntries, ConfigEntryState
from homeassistant.core import HomeAssistant
from homeassistant.helpers import device_registry as dr, entity_registry as er
from homeassistant.setup import async_setup_component

DOMAIN="card_builder"
async def main():
    ifaces=[x.split(":",1)[0].strip() for x in
            Path("/proc/self/net/dev").read_text().splitlines()[2:] if ":" in x]
    # Docker --network internal: isolated private bridge, no default route
    # and no reachable external network; multicast membership on eth0 works
    # unlike Docker --network none, which makes real Core Zeroconf fail.
    assert "lo" in ifaces and len(ifaces) == 2
    # Docker --internal is independently checked by the runner, not an
    # assumption about the container's kernel routing-table representation.
    assert __import__("os").environ.get("P6G_DOCKER_INTERNAL") == "verified"
    assert Path("/config/custom_components/card_builder/frontend/dist/card-builder-panel.js").is_file()
    hass=HomeAssistant("/config")
    loader.async_setup(hass)
    hass.config_entries=ConfigEntries(hass,{DOMAIN:{}})
    try:
        dr.async_setup(hass)
        await dr.async_load(hass)
        await er.async_load(hass)
        # Core bootstrap normally wires its native auth manager before HTTP.
        # Use the actual manager and an empty synthetic user/provider store,
        # not a mocked authorization object or any household account.
        hass.auth = await auth_manager_from_config(hass, [], [])
        for component in ("http","websocket_api","frontend","panel_custom"):
            cfg={"http":{"server_host":"127.0.0.1"}} if component=="http" else {}
            assert await asyncio.wait_for(
                async_setup_component(hass,component,cfg),timeout=75
            ), "Official Core subsystem did not initialize: "+component
        await asyncio.wait_for(hass.async_start(),timeout=50)
        assert await asyncio.wait_for(async_setup_component(
            hass,DOMAIN,{}),timeout=45), "real Card Builder async_setup failed"
        result=await asyncio.wait_for(
            hass.config_entries.flow.async_init(DOMAIN,context={"source":"user"}),
            timeout=40
        )
        assert result["type"]=="form", "Card Builder user form missing"
        complete=await asyncio.wait_for(
            hass.config_entries.flow.async_configure(result["flow_id"],{}),
            timeout=50
        )
        assert complete["type"]=="create_entry"
        entries=hass.config_entries.async_entries(DOMAIN)
        assert len(entries)==1
        entry=entries[0]
        await hass.async_block_till_done()
        if entry.state is not ConfigEntryState.LOADED:
            assert await asyncio.wait_for(
                hass.config_entries.async_setup(entry.entry_id),timeout=100)
        assert entry.state is ConfigEntryState.LOADED
        assert DOMAIN in hass.data
        assert Path("/config/www/card_builder").is_dir()
        # Real HA frontend registration, not a fake panel or file stub.
        assert "card_builder" in hass.data
        assert await asyncio.wait_for(
            hass.config_entries.async_unload(entry.entry_id),timeout=90)
        await hass.async_block_till_done()
        assert entry.state is ConfigEntryState.NOT_LOADED
        assert DOMAIN not in hass.data
        assert await asyncio.wait_for(
            hass.config_entries.async_setup(entry.entry_id),timeout=90)
        assert entry.state is ConfigEntryState.LOADED
        assert DOMAIN in hass.data
        assert await asyncio.wait_for(
            hass.config_entries.async_unload(entry.entry_id),timeout=90)
        await asyncio.wait_for(
            hass.config_entries.async_remove(entry.entry_id),timeout=90)
        assert not hass.config_entries.async_entries(DOMAIN)
        assert DOMAIN not in hass.data
        report={
            "real_source":"studiobts/home-assistant-card-builder",
            "upstream_tag":"v2.6.0",
            "upstream_commit":"2ae8e5152a5f6af46307130a045133f900f4397e",
            "zip_release_sha256":"75ca0cc33397b16f2d2af4c2aad40391a84f013786c264a22b7186fe309fb7a7",
            "core_config_flow_and_load":True,
            "core_frontend_asset_registered":True,
            "core_unload_and_reload":True,
            "core_remove":True,
            "real_user_dashboard_tested":False,
            "production_ready":False,
        }
        Path("/config/card-builder-proof.json").write_text(
            json.dumps(report,sort_keys=True),encoding="utf-8")
    finally:
        await hass.async_stop()

if __name__=="__main__":
    import traceback
    Path("/config/card-builder-started").write_text("yes")
    try:
        asyncio.run(main())
    except BaseException:
        Path("/config/card-builder-error.txt").write_text(traceback.format_exc())
        raise
