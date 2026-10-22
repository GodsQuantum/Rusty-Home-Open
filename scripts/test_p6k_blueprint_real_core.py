#!/usr/bin/env python3
"""Real released Blueprint Studio 2.5.2 in official Core, synthetic users."""
import asyncio
import json
import traceback
from pathlib import Path
from importlib.metadata import version
from aiohttp import ClientSession, ClientTimeout
from homeassistant import loader
from homeassistant.auth import auth_manager_from_config
from homeassistant.auth.const import GROUP_ID_ADMIN, GROUP_ID_USER
from homeassistant.config_entries import ConfigEntries, ConfigEntryState
from homeassistant.core import HomeAssistant
from homeassistant.helpers import device_registry as dr, entity_registry as er
from homeassistant.setup import async_setup_component

DOMAIN = "blueprint_studio"
URL = "http://127.0.0.1:8123/api/blueprint_studio"
MARKER = "p6k_synthetic_note.txt"
CONTENT = "P6K_SYNTHETIC_ONLY_BLUEPRINT_TEST"

async def perform():
    assert tuple(int(v) for v in version("paramiko").split(".")[:2]) >= (3, 0)
    # Runner checks the Docker network's actual Internal=true property.
    # An internal bridge can still expose an isolated local subnet route.
    import os
    assert os.environ.get("P6K_DOCKER_INTERNAL") == "verified"
    interfaces = [x.split(":",1)[0].strip() for x in
        Path("/proc/self/net/dev").read_text().splitlines()[2:] if ":" in x]
    assert "lo" in interfaces and len(interfaces) == 2
    assert Path("/config/custom_components/blueprint_studio/www").is_dir()
    hass = HomeAssistant("/config")
    loader.async_setup(hass)
    hass.config_entries = ConfigEntries(hass, {DOMAIN: {}})
    try:
        dr.async_setup(hass)
        await dr.async_load(hass)
        await er.async_load(hass)
        hass.auth = await auth_manager_from_config(hass, [], [])
        for part in ("http", "websocket_api", "frontend", "panel_custom"):
            cfg = {"http": {"server_host": "127.0.0.1"}} if part == "http" else {}
            assert await asyncio.wait_for(async_setup_component(hass, part, cfg), 75), part
        await asyncio.wait_for(hass.async_start(), 75)
        assert await asyncio.wait_for(async_setup_component(hass, DOMAIN, {}), 90)
        flow = await asyncio.wait_for(
            hass.config_entries.flow.async_init(DOMAIN, context={"source":"user"}), 50)
        assert flow["type"] == "form", flow
        created = await asyncio.wait_for(
            hass.config_entries.flow.async_configure(flow["flow_id"], {}), 60)
        assert created["type"] == "create_entry", created
        entries = hass.config_entries.async_entries(DOMAIN)
        assert len(entries) == 1
        entry = entries[0]
        await hass.async_block_till_done()
        if entry.state is not ConfigEntryState.LOADED:
            assert await asyncio.wait_for(hass.config_entries.async_setup(entry.entry_id), 90)
        assert entry.state is ConfigEntryState.LOADED, entry.state

        # Both accounts created only in a disposable fake Core configuration.
        # The first Core user is the owner. Keep a separate second admin so
        # a genuine live permission downgrade can be tested without touching
        # the protected owner role.
        owner = await hass.auth.async_create_user("p6k-owner", group_ids=[GROUP_ID_ADMIN])
        assert owner.is_owner
        admin = await hass.auth.async_create_user("p6k-admin", group_ids=[GROUP_ID_ADMIN])
        user = await hass.auth.async_create_user("p6k-ordinary", group_ids=[GROUP_ID_USER])
        assert not admin.is_owner
        assert admin.is_admin and not user.is_admin
        async def get_access(who):
            refresh = await hass.auth.async_create_refresh_token(who, client_id="http://localhost/p6k-core-only")
            return hass.auth.async_create_access_token(refresh)
        admin_access, user_access = await get_access(admin), await get_access(user)
        assert admin_access != user_access
        async with ClientSession(timeout=ClientTimeout(total=12)) as client:
            async def request(method, *, token=None, query=None, body=None):
                headers = {"Authorization": "Bearer " + token} if token else {}
                async with client.request(method, URL, params=query, json=body, headers=headers) as response:
                    return response.status, (await response.text())[:1000]
            status, _ = await request("GET", query={"action":"list_files"})
            assert status == 401, ("anonymous", status)
            status, _ = await request("GET", token=user_access, query={"action":"list_files"})
            assert status == 403, ("ordinary-read", status)
            status, _ = await request("POST", token=user_access,
                body={"action":"create_file", "path":MARKER, "content":CONTENT})
            assert status == 403, ("ordinary-write", status)
            assert not Path("/config/"+MARKER).exists()
            status, payload = await request("GET", token=admin_access, query={"action":"get_version"})
            assert status == 200, ("admin version", status, payload)
            status, payload = await request("POST", token=admin_access,
                body={"action":"create_file", "path":MARKER, "content":CONTENT})
            assert status == 200, ("admin create", status, payload)
            assert Path("/config/"+MARKER).read_text() == CONTENT
            status, payload = await request("GET", token=admin_access,
                query={"action":"read_file", "path":MARKER})
            assert status == 200 and CONTENT in payload, ("admin read", status, payload)
            status, _ = await request("GET", token=user_access,
                query={"action":"read_file", "path":MARKER})
            assert status == 403, ("ordinary secret read", status)
            # Reuse the SAME bearer token after Core downgrades its holder:
            # Blueprint must consult live Core group status, not token age.
            await hass.auth.async_update_user(admin, group_ids=[GROUP_ID_USER])
            assert not admin.is_admin
            status, _ = await request("GET", token=admin_access,
                query={"action":"read_file", "path":MARKER})
            assert status == 403, ("demoted admin stale token READ", status)
            status, _ = await request("POST", token=admin_access,
                body={"action":"create_file", "path":"p6k_denied_after_demote.txt", "content":"DENY"})
            assert status == 403, ("demoted admin stale token WRITE", status)
            assert Path("/config/"+MARKER).read_text() == CONTENT
            assert not Path("/config/p6k_denied_after_demote.txt").exists()
        duplicate = await asyncio.wait_for(
            hass.config_entries.flow.async_init(DOMAIN, context={"source":"user"}), 45)
        assert duplicate["type"] == "abort" and duplicate["reason"] == "single_instance_allowed"
        assert await asyncio.wait_for(hass.config_entries.async_unload(entry.entry_id), 90)
        assert entry.state is ConfigEntryState.NOT_LOADED
        assert await asyncio.wait_for(hass.config_entries.async_setup(entry.entry_id), 90)
        assert entry.state is ConfigEntryState.LOADED
        assert await asyncio.wait_for(hass.config_entries.async_unload(entry.entry_id), 90)
        await asyncio.wait_for(hass.config_entries.async_remove(entry.entry_id), 90)
        assert not hass.config_entries.async_entries(DOMAIN)
        Path("/config/blueprint-proof.json").write_text(json.dumps({
            "genuine_core":True, "upstream_source":"ha-china/blueprint-studio",
            "upstream_tag":"v2.5.2", "upstream_sha":"943aed0be67f3a910b0f70a3864288d5e17e55ce",
            "installed_manifest_version":"2.5.2", "paramiko_present":True,
            "actual_core_config_entry_loaded":True, "actual_core_http_anonymous_401":True,
            "actual_core_http_nonadmin_get_403":True,
            "actual_core_http_nonadmin_write_403":True,
            "actual_core_http_admin_read_200":True,
            "actual_core_http_admin_write_200":True,
            "admin_created_and_read_synthetic_file":True,
            "actual_core_demoted_admin_bearer_denied":True,
            "duplicate_config_refused":True,
            "unload_reload_remove":True, "real_user_data_tested":False,
            "production_ready":False,
        }, sort_keys=True))
        print("P6K_BLUEPRINT_REAL_CORE_ADMIN_HTTP_LIFECYCLE_PASS")
    finally:
        await hass.async_stop()

if __name__=="__main__":
    try:
        asyncio.run(perform())
    except BaseException:
        Path("/config/blueprint-error.txt").write_text(traceback.format_exc())
        raise
