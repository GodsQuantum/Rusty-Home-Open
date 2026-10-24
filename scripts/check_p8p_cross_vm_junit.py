#!/usr/bin/env python3
import sys
from pathlib import Path
from xml.etree import ElementTree as ET
def verify(path,phase):
    if phase not in ("source","target"):raise ValueError("invalid phase")
    if not path.is_file() or path.stat().st_size>1024*1024:raise ValueError("missing JUnit")
    cases=ET.parse(path).getroot().findall(".//testcase")
    if len(cases)!=3 or {c.get("name") for c in cases}!={"test_start_supervisor","test_check_supervisor","test_hars_cross_vm_"+phase}:raise ValueError("wrong cases")
    if any(any(c.find(x) is not None for x in ("failure","error","skipped")) for c in cases):raise ValueError("failed/skipped")
    return "P8P_"+phase.upper()+"_REAL_QEMU_3_OF_3_PASS"
if __name__=="__main__":
    try:
        print(verify(Path(sys.argv[1]),sys.argv[2]))
    except (ValueError,OSError,ET.ParseError,IndexError) as ex:
        print("P8P_JUNIT_FAIL: "+str(ex),file=sys.stderr);sys.exit(2)
