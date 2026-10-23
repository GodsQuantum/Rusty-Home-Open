#!/usr/bin/env python3
"""Test actual 2026.11-dev Core Marketplace registration from its OCI image.

Runs in a disposable network-isolated, read-only official HA Core dev image.
Does NOT start a real home, execute installs or authenticate user sessions.
"""
from unittest.mock import patch
from homeassistant.const import __version__
from homeassistant.components import websocket_api
from homeassistant.components.marketplace.websocket import (
    async_register_websocket_commands,
)
from homeassistant.components.marketplace.const import DOMAIN

assert DOMAIN == "marketplace",DOMAIN
assert __version__.startswith("2026.11."), (
    f"Expected 2026.11 dev image, got {__version__}"
)
registered=[]
with patch.object(websocket_api, "async_register_command",side_effect=lambda hass,handler:registered.append(handler.__name__)):
    async_register_websocket_commands(object())

expected={
 "marketplace_info",
 "marketplace_repositories_list",
 "marketplace_repository_info",
}
assert expected.issubset(registered),(sorted(expected- set(registered)),registered)
mutators={"marketplace_repository_install","marketplace_repository_uninstall","marketplace_repositories_add"}
assert mutators.issubset(registered), (
 "Test accidentally imported a partial/unrepresentative Marketplace implementation"
)
assert len(registered) == len(set(registered)), "Duplicate Core command registration"
print("PASS: official Home Assistant",__version__,"Core Marketplace module imported")
print("PASS: Core itself registered",len(registered),"WebSocket handlers")
print("PASS: three expected read handlers and distinct write handlers exist")
print("SCOPE: official disposable Core image import/registration; no admin auth or production runtime signoff")
