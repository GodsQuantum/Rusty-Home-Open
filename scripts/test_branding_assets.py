#!/usr/bin/env python3
# Validate checked-in real screenshots without image processing dependencies.
from pathlib import Path
import struct
root=Path(__file__).resolve().parent.parent
for name,expected_w,min_h in (('lab-dashboard.png',1365,860),('lab-dashboard-mobile.png',390,844)):
 path=root/'branding'/name
 data=path.read_bytes()
 assert data[:8]==b'\x89PNG\r\n\x1a\n' and data[12:16]==b'IHDR', name
 w,h=struct.unpack('>II',data[16:24])
 assert w==expected_w and h>=min_h,(name,w,h)
 print(name, f'{w}x{h}', 'PASS')
for name in ('README.md','README.fr.md','README.zh-CN.md'):
 s=(root/name).read_text()
 assert 'branding/lab-dashboard.png' in s
 assert 'branding/lab-dashboard-mobile.png' in s
print('BRANDING_DESKTOP_MOBILE_ASSETS_PASS')
