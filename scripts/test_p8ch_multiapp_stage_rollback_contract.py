#!/usr/bin/env python3
"""P8ch scope contract: new two-App staged rollback, NOT production transaction."""
from pathlib import Path
root=Path(__file__).resolve().parent.parent
s=(root/"scripts/test_p8ch_multiapp_stage_rollback.sh").read_text()
w=(root/".github/workflows/p8ch-multiapp-stage-rollback.yml").read_text()
for token in (
 "GITHUB_ACTIONS", "RUNNER_TEMP", "$(id -u)", "set -euo pipefail",
 "docker.io/library/redis:7.4-alpine",
 "docker.io/getmeili/meilisearch:v1.55.0",
 "docker.io/library/python:3.12-alpine",
 "docker.io/library/redis@sha256:",
 "docker.io/getmeili/meilisearch@sha256:",
 "--install-lab", "--install-path-resources-lab",
 "--network \"container:rh-$who\"", "--read-only",
 "--security-opt no-new-privileges", "--cap-drop ALL",
 "podman volume export --output", "podman volume import",
 "sha256sum -c", "INJECTED_FAULT", "cleanup_stage",
 "manager --remove-lab", "source_ok", "source_only",
 "P8CH_TWO_STATEFUL_APPS_SAME_QUIESCED_CHECKPOINT_PASS",
 "P8CH_SECOND_ARCHIVE_TAMPER_REJECTED_AFTER_FIRST_RESTORE_STARTED_PASS",
 "P8CH_FAILED_PARTIAL_BATCH_CLEANUP_ORIGINALS_PRESERVED_PASS",
 "P8CH_DIVERSE_PAIR_INDEPENDENT_RECOVERY_SHARED_POINT_IN_TIME_PASS",
 "P8CH_RESTORED_PAIR_RESTART_DURABLE_SOURCES_UNAFFECTED_PASS",
 "P8CH_TWO_ORIGINALS_UNTOUCHED_AFTER_STAGE_REMOVAL_PASS",
):
 assert token in s, token
# An actual failed SECOND stage must occur after the first restored service
# is running; cleanup follows the failure. No false atomic live cutover claim.
a=s.index('manager --start-lab "$RS"')
b=s.index('P8CH_SECOND_ARCHIVE_TAMPER_REJECTED_AFTER_FIRST_RESTORE_STARTED_PASS')
c=s.index("cleanup_stage",b)
d=s.index("P8CH_FAILED_PARTIAL_BATCH_CLEANUP_ORIGINALS_PRESERVED_PASS",c)
assert a < b < c < d
for banned in ("sudo podman","--privileged","--publish","--network host",
               "0.0.0.0","192.168.","VM100","pct exec","ssh "):
 assert banned not in s,banned
start=s.index("  meili_helper \"$who\" '\n",s.index("meili_probe()"))+len("  meili_helper \"$who\" '\n")
end=s.index("\n' \"$op\"",start)
compile(s[start:end],"p8ch_genuine_meili_rest.py","exec")
for token in ("pull_request:","permissions:","contents: read",
 "runs-on: ubuntu-24.04","persist-credentials: false",
 "python3 scripts/test_p8ch_multiapp_stage_rollback_contract.py",
 "bash -n scripts/test_p8ch_multiapp_stage_rollback.sh",
 "bash scripts/test_p8ch_multiapp_stage_rollback.sh",
 "cargo build --locked -p hars-native-hub --bin oci_app_manager_lab",
 "podman uidmap slirp4netns fuse-overlayfs"):
 assert token in w,token
assert "workflow_dispatch:" not in w
print("P8CH_TWO_REAL_OCI_APPS_PARTIAL_ROLLBACK_STATIC_CONTRACT_PASS")
