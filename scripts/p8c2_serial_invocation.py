"""Exact systemd InvocationID decoder for official HAOS QEMU serial console.

labgrid/HAOS consoles can pad one command output with literal tilde cells.
Only precisely 32 hex digits followed by optional tildes are admissible.
Never accept additional bytes or choose a substring from arbitrary text.
"""
import re

_INVOCATION = re.compile(r"([0-9a-fA-F]{32})~*\Z")

def parse_systemd_invocation(raw: str) -> str:
    if not isinstance(raw, str) or len(raw) > 256:
        raise ValueError("invalid Supervisor service invocation")
    match = _INVOCATION.fullmatch(raw.strip())
    if match is None:
        raise ValueError("malformed Supervisor systemd InvocationID")
    return match.group(1).lower()
