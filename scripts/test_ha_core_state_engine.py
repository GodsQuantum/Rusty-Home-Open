"""Exercise the official Home Assistant Core in-memory state registry only.
No integrations, database, network, radio, credentials or automation started.
"""
import asyncio
import tempfile
from homeassistant.core import HomeAssistant

async def main():
    with tempfile.TemporaryDirectory(prefix="hars-core-state-", dir="/tmp") as folder:
        hass = HomeAssistant(folder)
        for idx in range(1232):
            hass.states.async_set(f"sensor.hars_fixture_{idx}", str(idx),
                                  {"synthetic": True, "origin": "lab"})
        entities = hass.states.async_all()
        assert len(entities) == 1232, len(entities)
        assert hass.states.get("sensor.hars_fixture_1231").state == "1231"
        assert all(x.attributes.get("synthetic") is True for x in entities)
        print("PASS: authentic HA Core state registry contains 1232 synthetic entities")
asyncio.run(main())
