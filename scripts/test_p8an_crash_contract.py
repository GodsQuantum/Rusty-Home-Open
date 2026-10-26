#!/usr/bin/env python3
from pathlib import Path
root=Path(__file__).resolve().parent.parent
workflow=(root/'.github/workflows/p8an-crash-restart.yml').read_text()
smoke=(root/'scripts/test_p8aj_real_systemd_guest.sh').read_text()
unit=(root/'packaging/systemd/rusty-home-haos-lab.service').read_text()
for marker in ('HARS_CRASH_RECOVERY_LAB=1','test_p8an_crash_contract.py',
 'haos_ova-18.3.qcow2.xz','fae6a728768cc10aff60d4820bfcd40d64cd77fab82c8bd92af13b3d9d414090'):
 assert marker in workflow,marker
for marker in ('GITHUB_ACTIONS','sudo kill -KILL "$qemu_pid"',
 'NRestarts','P8AN_REAL_QEMU_CRASH_SYSTEMD_AUTO_RESTART_SAME_DISK_PASS',
 'P8AN_RESTARTED_GUEST_CLEAN_STOP_NO_ORPHANS_PASS',
 'guest-overlay.qcow2','systemctl is-active'):
 assert marker in smoke,marker
assert 'systemctl reset-failed "$unit"' not in smoke
assert 'Restart=on-failure' in unit
assert 'WatchdogSec=30s' in unit
assert '[Install]' not in [line.strip() for line in unit.splitlines()
                          if not line.lstrip().startswith('#')]
assert '192.168.' not in workflow
print('P8AN_REAL_QEMU_CRASH_SYSTEMD_RECOVERY_ISOLATED_CONTRACT_PASS')
