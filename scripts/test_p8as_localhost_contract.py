#!/usr/bin/env python3
from pathlib import Path
r=Path(__file__).resolve().parent.parent
unit=(r/'crates/native-hub/src/oci_quadlet.rs').read_text()
manager=(r/'crates/native-hub/src/bin/oci_app_manager_lab.rs').read_text()
cli=(r/'crates/native-hub/src/bin/oci_quadlet_lab.rs').read_text()
script=(r/'scripts/test_p8ar_native_oci_manager.sh').read_text()
workflow=(r/'.github/workflows/p8as-oci-localhost-tcp.yml').read_text()
for v in ('AppNetwork::Offline','AppNetwork::LoopbackTcp','PublishPort=127.0.0.1:',
          'Network=slirp4netns','Network=none'):
 assert v in unit,v
assert 'host_port < 1024' in unit or 'host_port<1024' in unit
assert 'network:AppNetwork::Offline' in cli.replace(' ','')
for v in ('--local-tcp','AppNetwork::LoopbackTcp','AppNetwork::Offline'):
 assert v in manager,v
for v in ('HARS_LOCAL_TCP_LAB','127.0.0.1:18879','socket.create_connection',
          'P8AS_REAL_ROOTLESS_REDIS_LOCALHOST_ONLY_TCP_PASS'):
 assert v in script,v
assert '0.0.0.0:18879' not in script
assert 'hostfwd' not in script
assert 'HARS_LOCAL_TCP_LAB=1 bash' in workflow
assert 'ubuntu-24.04' in workflow
print('P8AS_OPT_IN_LOCALHOST_TCP_ROOTLESS_QUADLET_AND_NETWORK_POLICY_CONTRACT_PASS')
