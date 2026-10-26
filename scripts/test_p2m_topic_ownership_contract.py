#!/usr/bin/env python3
"""Scope guard: read-only native Moli state topic ownership, not full MQTT stack."""
from pathlib import Path
root=Path(__file__).resolve().parent.parent
impl=(root/"crates/native-kernel/src/mqtt_hub.rs").read_text()
workflow=(root/".github/workflows/p2l-moli-atomic-discovery.yml").read_text()
for value in (
    "let mut globally_owned_topics: HashSet<String> = HashSet::new();",
    "let mut batch_topics = HashSet::new();",
    "!batch_topics.insert(s.state_topic.as_str())",
    "globally_owned_topics.contains(&s.state_topic)",
    "globally_owned_topics.insert(sensor.state_topic.clone());",
    "globally_owned_topics.remove(&state_topic);",
    "identical_topic_cannot_publish_state_to_two_unique_ids_or_steal_owner",
    "a shared topic must never create another device implicitly",
    "reassigning a topic must not copy old Moli state",
):
    assert value in impl,value
for required in ("test_p2m_topic_ownership_contract.py", "identical_topic_cannot_publish_state_to_two_unique_ids_or_steal_owner"):
    assert required in workflow,required
assert 'write: false' in impl
assert '--network' not in impl
print("P2M_REAL_UPSTREAM_MOLI_TOPIC_OWNERSHIP_NO_STATE_ALIAS_CONTRACT_PASS")
