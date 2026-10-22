#!/usr/bin/env python3
"""Offline one-connection Python mock server for Rust sidecar IPC tests."""
import json
import os
from pathlib import Path
import socket
import sys

CORE_STATES = len(sys.argv) > 3 and sys.argv[3] == "--core-states"
if len(sys.argv) > 3 and sys.argv[3] in ("--require-homeassistant", "--core-states"):
    import homeassistant.core  # official pinned Core runtime, lab only

def state_fixture():
    if not CORE_STATES:
        return [{"entity_id": f"sensor.mock_{i}", "state": str(i)}
                for i in range(1232)]
    import asyncio
    import tempfile

    async def collect():
        with tempfile.TemporaryDirectory(prefix="hars-core-state-", dir="/tmp") as folder:
            hass = homeassistant.core.HomeAssistant(folder)
            for i in range(1232):
                hass.states.async_set(f"sensor.mock_{i}", str(i), {"synthetic": True})
            values = hass.states.async_all()
            assert len(values) == 1232
            return [{"entity_id":state.entity_id, "state":state.state} for state in values]

    return asyncio.run(collect())

sock = Path(sys.argv[1])
token_path = Path(sys.argv[2])
expected = token_path.read_text()
with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as server:
    server.bind(str(sock))
    os.chmod(sock, 0o600)
    server.listen(1)
    server.settimeout(6)
    conn, _ = server.accept()
    with conn:
        conn.settimeout(4)
        frame=bytearray()
        while b"\n" not in frame:
            part=conn.recv(512)
            if not part:
                raise ConnectionError("truncated IPC message")
            frame.extend(part)
            if len(frame)>512:
                raise ValueError("oversized IPC frame")
        req=json.loads(frame.split(b"\n",1)[0])
        if (req.get("protocol") == "hars-sidecar-v1"
                and req.get("op") == "snapshot"
                and req.get("token") == expected):
            result = {
                "protocol": "hars-sidecar-v1", "read_only": True, "generation": 1,
                "entities": state_fixture()
            }
        else:
            result = {"protocol": "hars-sidecar-v1", "read_only": False,
                      "generation": 0, "entities": []}
        conn.sendall((json.dumps(result) + "\n").encode())
sock.unlink(missing_ok=True)
