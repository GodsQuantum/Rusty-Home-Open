#!/usr/bin/env python3
from pathlib import Path
r=Path(__file__).resolve().parent.parent
rust=(r/'crates/native-hub/src/oci_quadlet.rs').read_text()
manager=(r/'crates/native-hub/src/bin/oci_app_manager_lab.rs').read_text()
script=(r/'scripts/test_p8aw_real_registry_custom_volume.sh').read_text()
http=(r/'scripts/p8aw_registry_http_proof.py').read_text()
wf=(r/'.github/workflows/p8aw-custom-volume-registry.yml').read_text()
for v in ("render_at", "reserved", "InvalidResources", "volume_name.trim_end_matches",
          '"/proc","/sys"' ):
 assert v.replace('","','", "') in rust or v in rust,v
for v in ('--install-path-lab','render_at(path)','AppNetwork::LoopbackTcp',
          'podman(&["image", "exists", image])'):
 assert v in manager,v
for v in ('registry:3.1.2','/var/lib/registry','P8AW_CUSTOM_NAMED_VOLUME_REGISTRY_INSTALL_NO_AUTOSTART_PASS',
          'P8AW_REAL_REGISTRY_NAMED_VOLUME_AND_LOCALHOST_ONLY_BIND_PASS',
          'P8AW_ACTUAL_DOCKER_REGISTRY_BLOB_SURVIVES_RUST_APP_RESTART_PASS'):
 assert v in script,v
for v in ('/v2/','blobs/uploads/','sha256:','--verify','--upload'):
 assert v in http,v
for v in ('ubuntu-24.04','test_p8aw_data_path_contract.py',
          'test_p8aw_real_registry_custom_volume.sh','podman uidmap'):
 assert v in wf,v
assert '0.0.0.0:18884' not in script
assert 'sudo podman' not in script
assert '192.168.' not in wf
print('P8AW_GENERIC_SAFE_APP_STORAGE_VOLUME_DESTINATION_CONTRACT_PASS')
