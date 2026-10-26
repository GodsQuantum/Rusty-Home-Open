#!/usr/bin/env python3
from pathlib import Path
r=Path(__file__).resolve().parent.parent
manager=(r/'crates/native-hub/src/bin/oci_app_manager_lab.rs').read_text()
script=(r/'scripts/test_p8ar_native_oci_manager.sh').read_text()
workflow=(r/'.github/workflows/p8au-app-health-rollback.yml').read_text()
for v in ('HealthCmd=','HealthInterval=15s','HealthTimeout=5s','HealthRetries=2',
    '--install-health-lab','require_health(', 'healthcheck', '"run"',
    'application health failed', 'restart_from_unit'):
 assert v in manager,v
for v in ('HARS_HEALTH_ROLLBACK_LAB','busybox:1.36','redis-cli ping',
    'P8AU_NATIVE_GENERIC_HEALTHCHECK_ON_REAL_REDIS_PASS',
    'P8AU_UNHEALTHY_OCI_APP_AUTO_IMAGE_RECOVERY_PASS',
    'P8AU_REAL_NATIVE_OCI_HEALTHY_UPGRADE_AND_EXPLICIT_REVERT_PASS'):
 assert v in script,v
for v in ('ubuntu-24.04','test_p8au_health_contract.py',
          'HARS_HEALTH_ROLLBACK_LAB=1'):
 assert v in workflow,v
assert 'sudo podman' not in script
assert '192.168.' not in workflow
print('P8AU_GENERIC_NATIVE_PODMAN_HEALTH_AND_FAIL_CLOSED_ROLLBACK_CONTRACT_PASS')
