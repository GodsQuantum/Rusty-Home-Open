"""Fictional config-entry lifecycle integration; no external network or devices."""
from homeassistant.core import HomeAssistant
from homeassistant.config_entries import ConfigEntry

DOMAIN = "hars_lifecycle_probe"
ENTITY = "sensor.hars_lifecycle_probe"

async def async_setup(hass: HomeAssistant, config: dict) -> bool:
    hass.data[DOMAIN] = {"setup_count": 0, "unload_count": 0, "remove_count": 0}
    return True

async def async_setup_entry(hass: HomeAssistant, entry: ConfigEntry) -> bool:
    assert entry.data["label"] == "fictional_test"
    data = hass.data[DOMAIN]
    data["setup_count"] += 1
    hass.states.async_set(ENTITY, "4242", {"synthetic": True})
    return True

async def async_unload_entry(hass: HomeAssistant, entry: ConfigEntry) -> bool:
    hass.data[DOMAIN]["unload_count"] += 1
    hass.states.async_remove(ENTITY)
    return True

async def async_remove_entry(hass: HomeAssistant, entry: ConfigEntry) -> None:
    hass.data[DOMAIN]["remove_count"] += 1
