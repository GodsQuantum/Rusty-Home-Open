#!/usr/bin/env python3
from pathlib import Path
r=Path(__file__).resolve().parent.parent
builder=(r/'scripts/build_guest_debian_package_lab.sh').read_text()
test=(r/'scripts/test_guest_debian_package_lab.sh').read_text()
workflow=(r/'.github/workflows/p8ak-guest-debian.yml').read_text()
unit=(r/'packaging/systemd/rusty-home-haos-lab.service').read_text()
assert '/usr/libexec/rusty-home/haos_guest_service_lab' in builder
assert '/usr/libexec/rusty-home/haos_guest_service_lab' in unit
assert '/usr/lib/systemd/system/rusty-home-haos-lab.service' in builder
assert '/usr/lib/sysusers.d/rusty-home-guest-lab.conf' in builder
for mark in ('--release --locked','dpkg-deb --root-owner-group','SOURCE_DATE_EPOCH',
             'systemd-sysusers','postinst','qemu-system-x86','ovmf'):
 assert mark in builder,mark
for not_allowed in ('systemctl start','systemctl enable','systemctl restart','/etc/systemd/system'):
 assert not_allowed not in builder,not_allowed
active=[l.strip() for l in unit.splitlines() if l.strip() and not l.strip().startswith('#')]
assert '[Install]' not in active
assert any(l=='NoNewPrivileges=yes' for l in active)
assert any(l=='DeviceAllow=/dev/kvm rw' for l in active)
for mark in ('cmp "$OUTDIR/', 'sudo dpkg -i','sudo dpkg --purge','systemctl is-enabled',
             'systemctl is-active','P8AK_ACTUAL_DPKG_UPGRADE_DOWNGRADE_PURGE_NO_AUTOSTART_PASS'):
 assert mark in test,mark
for mark in ('test_guest_debian_package_lab.sh','test_p8ak_guest_debian_contract.py',
             'ubuntu-24.04','qemu-system-x86'):
 assert mark in workflow,mark
assert '192.168.' not in workflow and '/srv/storage' not in workflow
print('P8AK_SAFE_OPT_IN_NATIVE_RUST_DEBIAN_PACKAGE_CONTRACT_PASS')
