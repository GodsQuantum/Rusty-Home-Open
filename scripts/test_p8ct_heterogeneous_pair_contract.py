#!/usr/bin/env python3
"""P8ct contract: two distinct real OCI engines, SIGKILL, no Cloud9 run."""
from pathlib import Path
r=Path(__file__).resolve().parents[1]
src=(r/"crates/native-hub/src/bin/oci_app_manager_lab.rs").read_text()
sh=(r/"scripts/test_p8ct_heterogeneous_pair_sigkill.sh").read_text()
w=(r/".github/workflows/p8ct-heterogeneous-native-pair.yml").read_text()
for name in ("fn heterogeneous_pair_units(", "fn install_pair_paths(",
 "--install-pair-path-dependent-lab", "fn install_rendered_pair(",
 "install_rendered_pair(", "fn start_pair(",
 "HARS_P8CS_KILL_AFTER_PROVIDER", "fn recover_pair(", "PAIR_START_JOURNAL",
 "fn heterogeneous_pair_offline_quiescent_transaction_and_path_refusal()"):
 assert name in src,name
for name in ("docker.io/library/redis:7.4-alpine",
 "docker.io/getmeili/meilisearch:v1.55.0",
 "docker.io/library/python:3.12-alpine", "/meili_data", "/data",
 "HARS_DISPOSABLE_OCI_LAB=1", "HARS_P8CS_KILL_AFTER_PROVIDER=1",
 "podman exec", "sha256:", "redis-cli SAVE", "p8ct_native",
 "container:rh-", "NetworkMode", "ReadonlyRootfs",
 "P8CT_ATOMIC_TWO_DIFFERENT_NATIVE_APP_DESCRIPTORS_OFFLINE_INERT_PASS",
 "P8CT_TRUE_REDIS_PLUS_MEILISEARCH_BOTH_STATEFUL_ENGINES_DURABLE_DATA_PASS",
 "P8CT_NATIVE_COORDINATOR_SIGKILL_BETWEEN_DIFFERENT_ENGINES_FENCED_PASS",
 "P8CT_DIFFERENT_APP_PARTIAL_START_CRASH_STOP_ONLY_RECOVERY_PASS",
 "P8CT_DIFFERENT_REDIS_RDB_AND_MEILI_INDEX_SURVIVE_COORDINATOR_RESTART_PASS",
 "P8CT_ORDERED_NATIVE_DIFFERENT_APP_CLEANUP_PRESERVES_BOTH_DATABASES_PASS"):
 assert name in sh,name
for name in ("ubuntu-24.04", "timeout-minutes: 23",
 "cargo test --locked -p hars-native-hub --all-targets",
 "cargo clippy --locked -p hars-native-hub --all-targets",
 "scripts/test_p8ct_heterogeneous_pair_sigkill.sh",
 "scripts/test_p8ct_heterogeneous_pair_contract.py"):
 assert name in w,name
for text in ("VM100", "192.168.", "sudo podman","pct ",
 "systemctl --system", "sudo /usr/sbin"):
 assert text not in sh+src[src.index("// P8ct:"):src.index("// All-or-none publication")],text
assert 'if env::var("GITHUB_ACTIONS").as_deref() != Ok("true")' in src
assert sh.startswith("#!/usr/bin/env bash")
assert '"$(id -u)" != 0' in sh
print("P8CT_DISTINCT_STATEFUL_APP_CRASH_INTENT_AND_SCOPE_CONTRACT_PASS")
