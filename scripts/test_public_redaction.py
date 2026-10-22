#!/usr/bin/env python3
import json
import re
from pathlib import Path
root=Path(__file__).resolve().parent.parent
inv=json.loads((root/'audits/haos-apps.json').read_text())
assert inv['source']=='SYNTHETIC_PUBLIC_EXAMPLES_ONLY_NOT_REAL_HOME'
assert len(inv['apps'])==9
assert all(a['slug']==f'demo_app_{i:02d}' and a['state']=='stopped' for i,a in enumerate(inv['apps'],1))
for p in root.rglob('*'):
 if not p.is_file() or '/.git/' in str(p) or '/target/' in str(p) or '/runtime/' in str(p):continue
 if p.suffix in ('.png','.jpg','.jpeg','.webp') or p.stat().st_size>2500000:continue
 try:s=p.read_text()
 except (UnicodeDecodeError,PermissionError):continue
 assert not re.search(r'\b192\.168\.\d{1,3}\.\d{1,3}\b',s),p
 assert not re.search(r'github_pat_[A-Za-z0-9_]{20,}|-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----',s),p
 assert ('real HAOS ' + 'VM100, read-only') not in s,p if p != Path(__file__) else True
print('PUBLIC_FIXTURE_GUARD_PASS')
