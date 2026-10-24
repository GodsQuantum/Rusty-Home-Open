#!/usr/bin/env python3
"""Pinned official Home Assistant Core writes actual .storage entity registry.
Offline, non-root, synthetic Core integration. Never reads household data.
"""
import asyncio
import json
from pathlib import Path
from homeassistant import loader
from homeassistant.config_entries import ConfigEntries
from homeassistant.core import HomeAssistant
from homeassistant.helpers import device_registry as dr, entity_registry as er
from homeassistant.setup import async_setup_component

DOMAIN = "hars_lifecycle_probe"

async def main():
    ifaces=[line.split(":",1)[0].strip() for line in Path("/proc/self/net/dev").read_text().splitlines()[2:] if ":" in line]
    assert ifaces==["lo"]
    assert not Path("/proc/net/route").read_text().splitlines()[1:]
    hass=HomeAssistant("/config")
    loader.async_setup(hass)
    hass.config_entries=ConfigEntries(hass,{DOMAIN:{}})
    try:
        await asyncio.wait_for(hass.async_start(),timeout=45)
        dr.async_setup(hass)
        await asyncio.wait_for(dr.async_load(hass),timeout=45)
        await asyncio.wait_for(er.async_load(hass),timeout=45)
        assert await asyncio.wait_for(async_setup_component(hass,DOMAIN,{DOMAIN:{}}),timeout=45)
        flow=await asyncio.wait_for(hass.config_entries.flow.async_init(DOMAIN,context={"source":"user"}),timeout=45)
        assert flow["type"]=="form"
        finish=await asyncio.wait_for(hass.config_entries.flow.async_configure(
            flow["flow_id"],{"label":"fictional_test"}),timeout=45)
        assert finish["type"]=="create_entry"
        entries=hass.config_entries.async_entries(DOMAIN)
        assert len(entries)==1
        entry=entries[0]
        await hass.async_block_till_done()
        registry=er.async_get(hass)
        created_ids=[]
        for tag in ("fictional-serial-a","fictional-serial-b"):
            item=registry.async_get_or_create(
                "sensor",DOMAIN,unique_id=tag,config_entry=entry,
                suggested_object_id="synthetic_"+tag[-1])
            created_ids.append(item.entity_id)
        assert len(set(created_ids))==2
        hass.states.async_set(created_ids[0],"21.5",{"unit_of_measurement":"°C"})
        hass.states.async_set(created_ids[1],"on",{"synthetic":True})
        snapshots=[hass.states.get(e).as_dict() for e in created_ids]
        assert [row["state"] for row in snapshots]==["21.5","on"]
        snapshot_path=Path("/config/core-states-snapshot.json")
        snapshot_path.write_text(json.dumps(snapshots),encoding="utf-8")
        snapshot_path.chmod(0o600)
        await hass.async_block_till_done()
        # Real Core schedules this registry's persistence after 10 seconds.
        # Let the public lifecycle flush itself; do not call a private Store
        # method with an incorrectly guessed signature.
        await asyncio.sleep(12)
        doc=json.loads(Path("/config/.storage/core.entity_registry").read_bytes())
        assert doc["key"]=="core.entity_registry"
        included=[x for x in doc["data"]["entities"]
                  if x.get("unique_id") in ("fictional-serial-a","fictional-serial-b")]
        assert len(included)==2
        assert all(x["config_entry_id"]==entry.entry_id for x in included)
        print("P8Q_REAL_CORE_REGISTRY_CREATED_AND_SAVED_PASS")
    finally:
        await hass.async_stop()

if __name__=="__main__":
    asyncio.run(main())
