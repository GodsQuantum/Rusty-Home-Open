#!/usr/bin/env python3
from pathlib import Path
r=Path(__file__).resolve().parent.parent
unit=(r/'packaging/systemd/rusty-home-haos-lab.service').read_text()
script=(r/'scripts/test_p8aj_real_systemd_guest.sh').read_text()
workflow=(r/'.github/workflows/p8aj-real-systemd.yml').read_text()
active=[line.strip() for line in unit.splitlines() if line.strip() and not line.lstrip().startswith('#')]
for x in ('User=rusty-home-lab','Group=kvm','Type=exec','Environment=HARS_DISPOSABLE_KVM_LAB=1',
 'StateDirectoryMode=0700','Restart=on-failure','StartLimitBurst=3',
 'TimeoutStopSec=45s','KillMode=control-group','NoNewPrivileges=yes',
 'ProtectSystem=strict','ProtectHome=yes','DevicePolicy=closed',
 'DeviceAllow=/dev/kvm rw','EnvironmentFile=/run/rusty-home-lab/haos-guest.env','HAOS_BASE_SHA256'):
 assert any(x in line for line in active),x
assert '[Install]' not in active and not any(x.startswith('WantedBy=') for x in active)
assert not any('hostfwd=' in x for x in active)
for x in ('GITHUB_ACTIONS','systemd-analyze verify','systemctl start','systemctl stop',
 'systemctl is-enabled','pgrep -u rusty-home-lab',
 'P8AJ_REAL_SYSTEMD_RESTART_SAME_OVERLAY_AND_PINNED_SOURCE_PASS'):
 assert x in script,x
assert 'systemctl enable' not in script
for x in ('test_p8aj_real_systemd_guest.sh','haos_ova-18.3.qcow2.xz',
 'fae6a728768cc10aff60d4820bfcd40d64cd77fab82c8bd92af13b3d9d414090'):
 assert x in workflow,x
assert '192.168.' not in workflow
print('P8AJ_SYSTEMD_LAB_ONLY_ISOLATION_RBAC_AND_RESTART_CONTRACT_PASS')
