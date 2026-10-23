#!/usr/bin/env python3
"""Contract for dynamic, read-only Supervisor repository selection."""
from pathlib import Path

ui = Path("web/index.html").read_text()
assert 'id="store-repository"' in ui, "Repository dropdown missing"
assert "data.catalog.repositories" in ui, "Official repository inventory not used"
assert "dropdown.replaceChildren" in ui, "Repository options must be rebuilt from fresh Store"
assert "a.repository===repository" in ui, "Repository selection must filter actual app ownership"
assert 'dropdown.disabled=true' in ui, "Unavailable Store must disable filter"
assert 'select("#store-repository").addEventListener("change"' in ui, "Filter change listener missing"
assert ".innerHTML" not in ui, "Never render untrusted repository names as HTML"
print("DYNAMIC_STORE_REPOSITORY_FILTER_CONTRACT_PASS")
