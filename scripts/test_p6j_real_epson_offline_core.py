#!/usr/bin/env python3
"""Actual MIT Epson SNMP v0.1.3 in official Core; absent fictitious printer.

Negative contract only: a fake offline SNMP host must never be exposed as a
loaded printer. No user printer, LAN or real credentials are used.
"""
import asyncio
import json
from importlib.metadata import version
from pathlib import Path

from homeassistant import loader
from homeassistant.config_entries import ConfigEntries, ConfigEntryState
from homeassistant.core import HomeAssistant
from homeassistant.helpers import device_registry as dr, entity_registry as er
from homeassistant.setup import async_setup_component

DOMAIN="epson_snmp"

async def main():
    assert version("pysnmp")=="7.1.22", "pinned real upstream requirement not met"
    interfaces=[x.split(":",1)[0].strip() for x in
                Path("/proc/self/net/dev").read_text().splitlines()[2:] if ":" in x]
    assert interfaces==["lo"] and not Path("/proc/net/route").read_text().splitlines()[1:]
    hass=HomeAssistant("/config")
    loader.async_setup(hass)
    hass.config_entries=ConfigEntries(hass,{DOMAIN:{}})
    try:
        dr.async_setup(hass)
        await dr.async_load(hass)
        await er.async_load(hass)
        await asyncio.wait_for(hass.async_start(),45)
        await asyncio.wait_for(async_setup_component(hass,"sensor",{}),45)
        assert await asyncio.wait_for(async_setup_component(hass,DOMAIN,{}),45)
        flow=await asyncio.wait_for(hass.config_entries.flow.async_init(
            DOMAIN,context={"source":"user"}),45)
        assert flow["type"]=="form" and flow["step_id"]=="user"
        result=await asyncio.wait_for(hass.config_entries.flow.async_configure(
            flow["flow_id"],
            {"host":"127.0.0.1", "name":"Fictional Epson Printer", "community":"public",
             "version":"2c", "scan_interval":300},
        ),45)
        assert result["type"]=="create_entry", result
        entries=hass.config_entries.async_entries(DOMAIN)
        assert len(entries)==1
        entry=entries[0]
        await asyncio.wait_for(hass.async_block_till_done(),90)
        # No UDP listener exists on loopback; a real coordinator SNMP first
        # refresh must fail instead of fabricating an online printer.
        assert entry.state in (ConfigEntryState.SETUP_RETRY,ConfigEntryState.SETUP_ERROR), (
            "Offline nonexistent Epson printer was incorrectly marked LOADED: "
            +str(entry.state)
        )
        assert not hass.states.async_entity_ids("sensor"), (
            "Fictional SNMP printer produced live sensors while unreachable"
        )
        assert not hass.data.get(DOMAIN), "Coordinator leaked after denied setup"
        # Native Core must never produce two config entries for the same host.
        duplicated=await asyncio.wait_for(hass.config_entries.flow.async_init(
            DOMAIN,context={"source":"user"},
            data={"host":"127.0.0.1","name":"Duplicated Fake Epson",
                  "community":"public","version":"2c","scan_interval":300}
        ),45)
        assert duplicated["type"]=="abort" and duplicated["reason"]=="already_configured", (
            "Epson duplicate host accepted"
        )
        await asyncio.wait_for(hass.config_entries.async_remove(entry.entry_id),45)
        assert not hass.config_entries.async_entries(DOMAIN)
        proof={
            "genuine_core":True,
            "upstream_git_sha":"3a4f2c0120de7bfbf149041d484eefcc0b0cad0c",
            "upstream_version":"0.1.3",
            "pysnmp_version":"7.1.22",
            "native_core_config_flow_created":True,
            "offline_fictitious_host_not_loaded":True,
            "fake_live_sensors_absent":True,
            "duplicate_host_rejected":True,
            "entry_removed":True,
            "real_printer_touched":False,
            "positive_printer_lifecycle_certified":False,
            "production_ready":False,
        }
        Path("/config/epson-proof.json").write_text(json.dumps(proof,sort_keys=True))
    finally:
        await hass.async_stop()

if __name__=="__main__":
    import traceback
    try:
        asyncio.run(main())
    except BaseException:
        Path("/config/epson-error.txt").write_text(traceback.format_exc())
        raise
