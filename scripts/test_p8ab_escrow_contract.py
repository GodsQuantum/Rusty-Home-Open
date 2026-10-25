#!/usr/bin/env python3
from pathlib import Path
root=Path(__file__).resolve().parent.parent
w=(root/'.github/workflows/p8ab-native-escrow-rollback.yml').read_text()
c=(root/'crates/interop-moli/src/backup_escrow.rs').read_text()
s=(root/'scripts/test_p8ab_escrow_relay_bridge.py').read_text()
for needle in ('90fa213c3441e585d5a60387e9244d0ed38efaf8','fae6a728768cc10aff60d4820bfcd40d64cd77fab82c8bd92af13b3d9d414090','-snapshot -accel kvm','::test_hars_cross_vm_source','::test_hars_cross_vm_target','test_p8ab_escrow_relay_bridge.py','P8AB_GENUINE_HAOS_RUST_ESCROW_FRESH_GUEST_RESTORED_PASS'):
    assert needle in w,needle
for needle in ('escrow-ingest','escrow-export','archive.unlink()', 'file_digest'):
    assert needle in s,needle
for needle in ('persist_noclobber','O_NOFOLLOW','sync_all()', 'Sha256','opaque_no_decryption'):
    assert needle in c,needle
assert w.index('test_p8ab_escrow_relay_bridge.py') < w.rindex('::test_hars_cross_vm_target')
assert '192.168.' not in w
print('P8AB_NATIVE_RUST_CROSS_VM_CONTRACT_PASS')
