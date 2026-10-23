#!/usr/bin/env python3
"""No fake SPDX license conclusions, deterministic graph and lock provenance."""
from pathlib import Path
import copy,json,sys
sys.path.insert(0,str(Path(__file__).resolve().parent))
from generate_source_spdx import OUTPUT,validate
d=json.loads(OUTPUT.read_text())
validate(d)
assert len(d["packages"])==217
assert len(d["relationships"])>=400
assert sum(pkg["licenseDeclared"]=="NOASSERTION" for pkg in d["packages"])>=7
assert all(pkg["licenseConcluded"]=="NOASSERTION" for pkg in d["packages"])
for bad in (
    ("spdxVersion","SPDX-3.0"),
    ("dataLicense","MIT"),
):
    forged=copy.deepcopy(d)
    forged[bad[0]]=bad[1]
    try:validate(forged)
    except AssertionError:pass
    else:raise AssertionError("accepted tampered document metadata")
forged=copy.deepcopy(d)
forged["documentNamespace"]="https://example.invalid/"
try:validate(forged)
except AssertionError:pass
else:raise AssertionError("accepted non-pinned Cargo.lock namespace")
forged=copy.deepcopy(d)
forged["packages"][0]["filesAnalyzed"]=True
try:validate(forged)
except AssertionError:pass
else:raise AssertionError("claimed files analyzed without inspection")
forged=copy.deepcopy(d)
forged["packages"][0]["licenseConcluded"]="MIT"
try:validate(forged)
except AssertionError:pass
else:raise AssertionError("invented reviewed conclusion")
assert all("manifest_path" not in json.dumps(x) for x in d["packages"])
print("SOURCE_SPDX_217_PACKAGES_NO_FALSE_COMPLIANCE_PASS")
