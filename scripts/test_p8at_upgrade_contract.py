#!/usr/bin/env python3
from pathlib import Path
b=Path(__file__).resolve().parent.parent
manager=(b/'crates/native-hub/src/bin/oci_app_manager_lab.rs').read_text()
script=(b/'scripts/test_p8ar_native_oci_manager.sh').read_text()
workflow=(b/'.github/workflows/p8at-image-rollback.yml').read_text()
for v in ('--switch-image-lab','--rollback-image-lab','flock','O_NOFOLLOW',
          'replace_private_atomic','Pull=never','restart_from_unit',
          'image", "exists','rollback_image'):
 assert v in manager,v
for v in ('HARS_UPGRADE_ROLLBACK_LAB','redis:7.4-bookworm',
          'P8AT_MUTABLE_OR_STALE_OCI_UPGRADE_REJECTED_NO_MUTATION_PASS',
          'P8AT_REAL_DIGEST_PINNED_IMAGE_SWITCH_PERSISTENT_DATA_PASS',
          'P8AT_REAL_OCI_IMAGE_ROLLBACK_EXACT_UNIT_AND_DATA_RETAINED_PASS',
          'manager-proof upgraded-data','podman image inspect'):
 assert v in script,v
for v in ('ubuntu-24.04','scripts/test_p8at_upgrade_contract.py',
          'HARS_UPGRADE_ROLLBACK_LAB=1'):
 assert v in workflow,v
assert 'podman image rm' not in manager
assert 'sudo podman' not in script
assert "192.168." not in workflow
print('P8AT_PINNED_OCI_DIGEST_SWITCH_ROLLBACK_FAIL_CLOSED_CONTRACT_PASS')
