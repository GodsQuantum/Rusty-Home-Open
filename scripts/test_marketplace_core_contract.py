#!/usr/bin/env python3
"""Pin-check the future 2026.11 Marketplace READ protocol against Core source.

Uses only public immutable GitHub source, no HAOS instance or auth tokens.
This is a source/ABI contract test; NOT a real Core runtime certification.
"""
import re
import urllib.request
from pathlib import Path

CORE_SHA = "3bec44d0470b044bb41febd5c7f4046f02bd2fb1"
BASE = "https://raw.githubusercontent.com/home-assistant/core/" + CORE_SHA
FILES = {
    "websocket/__init__.py": ("marketplace/info",),
    "websocket/repositories.py": ("marketplace/repositories/list",),
    "websocket/repository.py": ("marketplace/repository/info",),
}
def source(file):
    url = BASE + "/homeassistant/components/marketplace/" + file
    with urllib.request.urlopen(url, timeout=25) as res:
        body = res.read(400_000)
    return body.decode("utf-8")

def main():
    loaded = {path:source(path) for path in FILES}
    registration = loaded["websocket/__init__.py"]
    for method in ("marketplace_info", "marketplace_repositories_list", "marketplace_repository_info"):
        if "async_register_command(hass, " + method + ")" not in registration:
            raise AssertionError("Core registration missing: " + method)
    for file,commands in FILES.items():
        data=loaded[file]
        for command in commands:
            if command not in data:
                raise AssertionError("Core WS method missing: " + command)
            pattern = (r'@websocket_api\.websocket_command\(\s*\{'
                      r'\s*probatio\.Required\("type"\):\s*"'
                      + re.escape(command)
                      + r'"[\s\S]*?\}\s*\)\s*@websocket_api\.require_admin\b')
            if not re.search(pattern, data):
                raise AssertionError("Core admin-only requirement changed: "+command)
    if 'probatio.Optional("categories"): [str]' not in loaded["websocket/repositories.py"]:
        raise AssertionError("Marketplace list categories schema changed")
    if 'probatio.Required("repository_id"): str' not in loaded["websocket/repository.py"]:
        raise AssertionError("Marketplace repository-info parameter changed")
    local=(Path(__file__).resolve().parent.parent/
           "crates/community-packages/src/marketplace_ws.rs").read_text()
    for method in ("marketplace/info", "marketplace/repositories/list",
                   "marketplace/repository/info"):
        if method not in local:
            raise AssertionError("Rust adapter missing command: " +method)
    for denied in ("marketplace/repository/install","marketplace/repository/uninstall",
                   "marketplace/repositories/add","marketplace/warning/accept"):
        if denied in local:
            raise AssertionError("Rust read-only adapter gained mutation: "+denied)
    print("PASS: Core", CORE_SHA, "has 3 official admin-only read WS commands")
    print("PASS: Rust produces only matching read commands; Core mutations not exposed")
    print("SCOPE: pinned dev source contract, not live Home Assistant 2026.11 runtime")
if __name__=="__main__":main()
