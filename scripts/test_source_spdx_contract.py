#!/usr/bin/env python3
"""No fake SPDX license conclusions, deterministic graph and lock provenance."""
from pathlib import Path
import copy,json,sys
sys.path.insert(0,str(Path(__file__).resolve().parent))
from generate_source_spdx import OUTPUT,validate
d=json.loads(OUTPUT.read_text())
validate(d)
assert len(d["packages"])==227
assert len(d["relationships"])>=400
assert sum(pkg["licenseDeclared"]=="NOASSERTION" for pkg in d["packages"])==7
assert next(x for x in d["packages"] if x["name"]=="ha-rust-bridge")["licenseDeclared"]=="MIT"
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
# Absolute paths in Cargo metadata differ between CI and Cloud9.
# Simulate an entirely different checkout while preserving identical
# dependency graph coordinates. The generated SPDX must be byte identical.
import copy as _copy
from generate_source_spdx import build,cargo_metadata
metadata=cargo_metadata()
portable=_copy.deepcopy(metadata)
replaced={}
for package in portable["packages"]:
    if package.get("source") is None:
        original=package["id"]
        relocated="path+file:///another_machine/independent_checkout/"+package["name"]+"#"+package["version"]
        replaced[original]=relocated
        package["id"]=relocated
for node in portable["resolve"]["nodes"]:
    node["id"]=replaced.get(node["id"],node["id"])
    for dep in node.get("deps",[]):
        dep["pkg"]=replaced.get(dep["pkg"],dep["pkg"])
actual=build(metadata)
relocated=build(portable)
assert actual==relocated,"SBOM depends on absolute workstation checkout path"
assert actual==d,"Pinned SBOM not reproducible at current lock"

print("SOURCE_SPDX_227_PACKAGES_NO_FALSE_COMPLIANCE_PASS")
