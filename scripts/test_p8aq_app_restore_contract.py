#!/usr/bin/env python3
from pathlib import Path
r=Path(__file__).resolve().parent.parent
s=(r/"scripts/test_p8ap_real_redis_quadlet.sh").read_text()
w=(r/".github/workflows/p8aq-real-oci-restore.yml").read_text()
for token in ('HARS_VOLUME_RESTORE_LAB','podman volume export','podman volume import',
              'podman volume exists "$original_volume"','! podman volume exists "$restored_volume"',
              'podman volume create "$restored_volume"','sha256sum -c',
              'P8AQ_CORRUPTED_OCI_BACKUP_REJECTED_PASS',
              'P8AQ_INDEPENDENT_RESTORED_REDIS_DATA_AND_PRIMARY_UNCHANGED_PASS',
              'podman exec rh-redis redis-cli SAVE','systemctl --user stop rh-redis.service',
              'post-backup-change','redis-restore'):
 assert token in s,token
assert 'sudo podman' not in s
assert "192.168." not in s
assert 'podman volume import "$original_volume"' not in s
for token in ('ubuntu-24.04','test_p8aq_app_restore_contract.py','HARS_VOLUME_RESTORE_LAB=1',
              'podman uidmap'):
 assert token in w,token
print('P8AQ_NATIVE_PODMAN_VOLUME_BACKUP_INDEPENDENT_RESTORE_CONTRACT_PASS')
