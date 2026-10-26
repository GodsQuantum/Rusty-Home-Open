#!/usr/bin/env python3
"""New SECOND stateful OCI App restore, beyond Redis and Registry restart."""
from pathlib import Path
r=Path(__file__).resolve().parent.parent
main=(r/"scripts/test_p8aw_real_registry_custom_volume.sh").read_text()
new=(r/"scripts/test_p8ax_registry_independent_restore.sh").read_text()
http=(r/"scripts/p8aw_registry_http_proof.py").read_text()
ci=(r/".github/workflows/p8aw-custom-volume-registry.yml").read_text()
for value in (
    "source scripts/test_p8ax_registry_independent_restore.sh",
    "--remove-lab oci-registry-restore",
):
    assert value in main,value
for value in (
    '$(printenv GITHUB_ACTIONS || true)', '!= 0 ]] || exit 2',
    '--stop-lab oci-registry',
    'podman volume export --output',
    'chmod 0600 "$snapshot"',
    'sha256sum -c',
    'printf \'CORRUPTED\'',
    'podman volume import "$recovered" "$snapshot"',
    '--install-path-lab',
    '18885:5000',
    '--upload-later',
    '--verify-later-absent',
    'P8AX_REGISTRY_INDEPENDENT_PERSISTENT_VOLUME_RESTORE_POINT_IN_TIME_PASS',
    'P8AX_RESTORED_APP_REMOVAL_PRESERVES_ORIGINAL_AND_BACKUP_DATA_PASS',
):
    assert value in new,value
assert 'PORT in ("18884", "18885")' in http
assert 'b"rusty-home-after-backup-blob' in http
for value in ("test_p8ax_independent_registry_restore_contract.py",
              "bash -n scripts/test_p8ax_registry_independent_restore.sh",
              "test_p8aw_real_registry_custom_volume.sh"):
    assert value in ci
assert "sudo podman" not in new
assert "192.168." not in new
print("P8AX_SECOND_REAL_STATEFUL_APP_CHECKPOINT_RESTORE_AND_FORK_CONTRACT_PASS")
