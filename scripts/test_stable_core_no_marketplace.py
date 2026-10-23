#!/usr/bin/env python3
"""Real HA 2026.10.2 image must NOT silently ship future Core Marketplace."""
import importlib.util
from homeassistant.const import __version__
assert __version__=="2026.10.2",__version__
assert importlib.util.find_spec("homeassistant.components.marketplace") is None, (
    "Core Marketplace unexpectedly exists in 2026.10.2; review version fence"
)
print("PASS: official HA Core 2026.10.2 retains legacy HACS / no embedded marketplace")
