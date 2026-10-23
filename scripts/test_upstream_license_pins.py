#!/usr/bin/env python3
"""Pinned source texts from upstream version-matched Git tags, not fake legal clearance."""
from pathlib import Path
import sys,json,copy
sys.path.insert(0,str(Path(__file__).resolve().parent))
from verify_upstream_license_pins import OUT,validate,PINS
data=json.loads(OUT.read_text())
assert validate(data)==6
assert data["archives_without_license_file"]==6
assert data["pinned_upstream_source_texts_verified"]==6
assert data["legal_compliance_certified"] is False
assert data["signed_sbom_present"] is False
assert data["review_complete"] is False
assert len({x[2] for x in PINS})==2
assert len({x[3] for x in PINS})==3
for bad in (
    {"row":"pinned_commit","value":"0"*40},
    {"row":"license_git_blob_sha1","value":"bad"},
    {"row":"declared_spdx_expression","value":"UNKNOWN"},
    {"row":"source_url","value":"https://example.invalid"},
):
    changed=copy.deepcopy(data)
    changed["rows"][0][bad["row"]]=bad["value"]
    try:validate(changed)
    except AssertionError:pass
    else:raise AssertionError("tampered provenance field accepted: "+bad["row"])
for field in ("legal_compliance_certified","review_complete","signed_sbom_present"):
    changed=copy.deepcopy(data)
    changed[field]=True
    try:validate(changed)
    except AssertionError:pass
    else:raise AssertionError("unearned release clearance accepted: "+field)
changed=copy.deepcopy(data)
changed["cargo_lock_sha256"]="0"*64
try:validate(changed)
except AssertionError:pass
else:raise AssertionError("untrusted Cargo.lock state accepted")
print("UPSTREAM_LICENSE_SIX_SOURCE_PIN_CONTRACT_PASS")
