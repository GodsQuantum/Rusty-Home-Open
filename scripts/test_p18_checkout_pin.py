#!/usr/bin/env python3
"""Immutable upstream checkout action and minimal workflow token exposure."""
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SHA = "3d3c42e5aac5ba805825da76410c181273ba90b1"
EXPECTED = {
    "ci.yml": 5,
    "core-ipc-full-lab.yml": 1,
    "fuzz.yml": 1,
    "p12a-debian-package-lab.yml": 1,
    "p12b-signed-apt-lab.yml": 1,
    "p8c2-haos-qemu.yml": 1,
}
found = {}
for f in (ROOT / ".github/workflows").glob("*.yml"):
    lines = f.read_text().splitlines()
    found[f.name] = 0
    for i, line in enumerate(lines):
        if "uses: actions/checkout@" not in line:
            continue
        match = re.fullmatch(r"(\s*)- uses: actions/checkout@([0-9a-f]{40}) # v7 immutable pin", line)
        assert match and match.group(2) == SHA, (f.name, i + 1)
        indent = match.group(1)
        assert lines[i + 1] == indent + "  with:", (f.name, i + 2)
        assert lines[i + 2] == indent + "    persist-credentials: false", (f.name, i + 3)
        found[f.name] += 1
found = {k: v for k, v in found.items() if v}
assert found == EXPECTED, (found, EXPECTED)
assert sum(found.values()) == 10
print("PASS P18: 10 immutable Checkout v7 usages, read-only checkout credentials disabled")
