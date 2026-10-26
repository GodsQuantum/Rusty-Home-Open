#!/usr/bin/env python3
from pathlib import Path
r=Path(__file__).resolve().parent.parent
src=(r/"crates/native-hub/src/bin/oci_app_manager_lab.rs").read_text()
script=(r/"scripts/test_p8ar_native_oci_manager.sh").read_text()
wf=(r/".github/workflows/p8ar-real-oci-manager.yml").read_text()
for token in ('GITHUB_ACTIONS','HARS_DISPOSABLE_OCI_LAB','geteuid','owner_dir(',
              'symlink_metadata','create_new(true)','OpenOptions','sync_all',
              'daemon-reload','"/usr/bin/systemctl"','"/usr/bin/podman"',
              '"--install-lab"','"--start-lab"','"--stop-lab"','"--remove-lab"'):
 assert token in src,token
assert "systemctl enable" not in src
assert "volume rm" not in src
for token in ('P8AR_NATIVE_ROOTLESS_APP_INSTALL_NO_AUTOSTART_PASS',
              'P8AR_RUST_MANAGER_NATIVE_START_STOP_PERSISTENT_APP_DATA_PASS',
              'P8AR_RUST_MANAGER_REMOVE_PRESERVES_USER_APP_VOLUME_PASS',
              'P8AR_APP_MANAGER_REJECTS_MUTABLE_AND_UNAUTHORIZED_INPUT_PASS',
              'podman volume exists rh-managedredis-data'):
 assert token in script,token
for token in ('ubuntu-24.04','scripts/test_p8ar_native_oci_manager.sh',
              'test_p8ar_manager_contract.py','podman uidmap'):
 assert token in wf,token
assert "192.168." not in wf and "sudo podman" not in script
print("P8AR_NATIVE_GENERIC_OCI_MANAGER_AUTH_AND_LIFECYCLE_CONTRACT_PASS")
