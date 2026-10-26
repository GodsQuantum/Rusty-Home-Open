#!/usr/bin/env python3
import json
from pathlib import Path
r=Path(__file__).resolve().parent.parent
e=json.loads((r/'config/native-backup-escrow-evidence.json').read_text())
g=json.loads((r/'config/release-gates.json').read_text())
assert e['head']=='58cbfe3023532b7acca68123641a58692fab541d'
assert e['merged']=='4757377d5d773844f6e278af4712597d50cd799a'
assert e['run']==40240344194 and e['result']=='success'
assert e['source_passed']==3 and e['target_passed']==3 and e['checks_passed']==12
assert e['production_gates_passed']==0 and e['total_gates']==15
assert len(e['unproven'])>=3
w=(r/'.github/workflows/p8ab-native-escrow-rollback.yml').read_text()
assert 'P8AB_GENUINE_HAOS_RUST_ESCROW_FRESH_GUEST_RESTORED_PASS' in w
for name in ('docs/WORK_LOG.md','docs/HANDOFF_CURRENT.md','docs/CERTIFICATION_EVIDENCE_REGISTER.md','docs/NATIVE_BACKUP_ESCROW_2026-10-25.md'):
 s=(r/name).read_text()
 assert str(e['run']) in s,name
 assert e['head'] in s,name
assert len(g['required_gates'])==15
from check_release_gate import audit
live=audit(g)
assert all(row['status']!='certified' or row['id'] not in live['blocked_ids'] for row in g['required_gates'])
assert live['deployment_authorized'] is False
print('P8AB_IMMUTABLE_6_OF_6_REAL_QEMU_EVIDENCE_WITH_ZERO_PRODUCTION_GATE_PASS')
