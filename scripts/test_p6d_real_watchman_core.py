#!/usr/bin/env python3
"""Real upstream Watchman 0.8.4 in official isolated Home Assistant Core."""
import asyncio
import json
import os
from homeassistant.const import __version__ as ha_core_version
from pathlib import Path
from importlib.metadata import version

from homeassistant import loader
from homeassistant.core import HomeAssistant
from homeassistant.config_entries import ConfigEntries, ConfigEntryState
from homeassistant.helpers import device_registry as dr, entity_registry as er
from homeassistant.setup import async_setup_component

DOMAIN = "watchman"

async def main():
    assert version("prettytable") == "3.12.0", "not original manifest dependency"
    expected_core=os.environ.get("HARS_EXPECTED_CORE_VERSION","2026.10.2")
    assert expected_core in {"2026.10.2","2026.10.4"}
    assert ha_core_version == expected_core, "unexpected Core OCI version"
    routes = Path("/proc/net/route").read_text().splitlines()[1:]
    assert not routes and [x.split(":",1)[0].strip() for x in
        Path("/proc/self/net/dev").read_text().splitlines()[2:] if ":" in x] == ["lo"]
    hass = HomeAssistant("/config")
    loader.async_setup(hass)
    hass.config_entries = ConfigEntries(hass, {DOMAIN: {}})
    try:
        await asyncio.wait_for(hass.async_start(), 45)
        dr.async_setup(hass)
        await dr.async_load(hass)
        await er.async_load(hass)
        # Actual Core built-in entity platforms are required by real Watchman.
        for domain in ("sensor", "text", "button"):
            assert await asyncio.wait_for(
                async_setup_component(hass, domain, {}), timeout=45
            ), "built-in entity platform not available: " + domain
        result = await asyncio.wait_for(
            hass.config_entries.flow.async_init(DOMAIN, context={"source":"user"}),
            45,
        )
        assert result["type"] == "create_entry", "real upstream Watchman config_flow refused"
        entries = hass.config_entries.async_entries(DOMAIN)
        assert len(entries) == 1, "Watchman config entry not created"
        entry = entries[0]
        await hass.async_block_till_done()
        if entry.state is not ConfigEntryState.LOADED:
            assert await asyncio.wait_for(
                hass.config_entries.async_setup(entry.entry_id), 75
            ), "Watchman setup failed"
        assert entry.state is ConfigEntryState.LOADED
        assert hass.services.has_service(DOMAIN, "report"), (
            "Real Watchman report action not registered"
        )
        # Execute the genuine upstream service, without writing a report file
        # or invoking user notifications/other integrations.
        report = await asyncio.wait_for(
            hass.services.async_call(
                DOMAIN, "report",
                {"create_file": False, "send_notification": False, "parse_config": False},
                blocking=True, return_response=True,
            ), timeout=75
        )
        assert isinstance(report, dict), "Watchman report returned no structured data"
        assert isinstance(report.get("missing_entities"), list)
        assert isinstance(report.get("missing_actions"), list)
        assert not Path("/config/watchman_report.txt").exists(), (
            "Report file was created despite create_file=False"
        )
        assert await asyncio.wait_for(
            hass.config_entries.async_unload(entry.entry_id), 75
        ), "Watchman unload failed"
        assert entry.state is ConfigEntryState.NOT_LOADED
        assert not hass.services.has_service(DOMAIN,"report"), (
            "Watchman service leaked after unload"
        )
        assert await asyncio.wait_for(
            hass.config_entries.async_setup(entry.entry_id), 75
        ), "Watchman reload failed"
        assert entry.state is ConfigEntryState.LOADED
        assert hass.services.has_service(DOMAIN,"report")
        assert await asyncio.wait_for(
            hass.config_entries.async_unload(entry.entry_id), 75
        )
        await asyncio.wait_for(
            hass.config_entries.async_remove(entry.entry_id),75
        )
        assert not hass.config_entries.async_entries(DOMAIN)
        assert not hass.services.has_service(DOMAIN,"report")
        proof = {
            "real_upstream":"dummylabs/thewatchman",
            "upstream_commit":"2eab21b7aeb4d4e20c71c3e2b3e449c42c4a3b35",
            "integration_version":"0.8.4",
            "dependency_prettytable":"3.12.0",
            "genuine_core_config_flow":True,
            "genuine_core_setup":True,
            "genuine_core_report_action":True,
            "genuine_core_report_executed_without_file":True,
            "genuine_core_unload_and_reload":True,
            "genuine_core_remove":True,
            "real_user_config_or_devices_tested":False,
            "production_ready":False,
        }
        # Keep original 2026.10.2 proof shape byte-compatible with its
        # independently verified strict P6d contract.
        if expected_core == "2026.10.4":
            proof["core_version"] = ha_core_version
        Path("/config/watchman-proof.json").write_text(
            json.dumps(proof,sort_keys=True),encoding="utf-8"
        )
    finally:
        await hass.async_stop()

if __name__=="__main__":
    import traceback
    Path("/config/watchman-started").write_text("launched")
    try:
        asyncio.run(main())
    except BaseException:
        Path("/config/watchman-failure.txt").write_text(traceback.format_exc())
        raise
