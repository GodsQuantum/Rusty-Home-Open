#!/usr/bin/env python3
"""Reject spoofed/duplicated laboratory attestation records."""
from pathlib import Path
import sys,json,copy
ROOT=Path(__file__).resolve().parent.parent
sys.path.insert(0,str(ROOT/"scripts"))
from verify_lab_evidence import validate
a=json.loads((ROOT/"config/lab-evidence.json").read_text())
assert len(validate(a))>=10
b=copy.deepcopy(a)
b["claims"][1]["id"]=b["claims"][0]["id"]
try:validate(b)
except AssertionError:pass
else:raise AssertionError("duplicate lab evidence accepted")
b=copy.deepcopy(a)
b["claims"][0]["status"]="certified"
try:validate(b)
except AssertionError:pass
else:raise AssertionError("lab falsely promoted to production certification")
b=copy.deepcopy(a)
b["claims"][0]["url"]="https://example.invalid"
try:validate(b)
except AssertionError:pass
else:raise AssertionError("forged evidence URL accepted")
b=copy.deepcopy(a)
b["claims"][0]["head_sha"]="0"*40
try:validate(b)
except AssertionError:pass
else:raise AssertionError("mismatched pinned apt SHA accepted")
print("PASS: lab evidence provenance shape, duplicate and tamper guards")
