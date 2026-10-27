#!/usr/bin/env python3
"""P8cn safety scope: two REAL stateful Apps, second SHA-valid runtime failure."""
from pathlib import Path
root=Path(__file__).resolve().parent.parent
s=(root/"scripts/test_p8cn_two_app_version_failure_rollback.sh").read_text()
w=(root/".github/workflows/p8cn-two-app-version-failure.yml").read_text()
for token in (
 "GITHUB_ACTIONS", "RUNNER_TEMP", "$(id -u)", "set -euo pipefail",
 "docker.io/library/redis:7.4-alpine", "docker.io/getmeili/meilisearch:v1.55.0",
 "docker.io/getmeili/meilisearch:v1.54.0",
 "docker.io/getmeili/meilisearch@sha256:", "docker.io/library/redis@sha256:",
 "--install-path-resources-lab", "podman volume export --output",
 "podman volume import", "sha256sum -c", "source_ok", "source_only",
 "cleanup_stage", "manager --remove-lab",
 "Your database version (1.55.0) is incompatible with your current engine version (1.54.0)",
 "P8CN_FIRST_RESTORED_REDIS_RUNNING_BEFORE_SECOND_APP_START_PASS",
 "P8CN_SECOND_APP_SHA_VALID_REAL_VERSION_FAILURE_FIRST_APP_STILL_HEALTHY_PASS",
 "P8CN_TWO_APP_RUNTIME_FAILURE_ROLLBACK_PRESERVES_ORIGINALS_PASS",
 "P8CN_COMPATIBLE_TWO_APP_RETRY_POINT_IN_TIME_PASS",
 "P8CN_RETRIED_PAIR_RESTART_DURABLE_SOURCES_UNAFFECTED_PASS",
 "P8CN_FINAL_ORIGINALS_UNTOUCHED_AFTER_STAGE_REMOVAL_PASS",
):
 assert token in s,token
first=s.index("P8CN_FIRST_RESTORED_REDIS_RUNNING_BEFORE_SECOND_APP_START_PASS")
second=s.index("P8CN_SECOND_APP_SHA_VALID_REAL_VERSION_FAILURE_FIRST_APP_STILL_HEALTHY_PASS")
cleanup=s.index("cleanup_stage",second)
retry=s.index("P8CN_COMPATIBLE_TWO_APP_RETRY_POINT_IN_TIME_PASS")
assert first < second < cleanup < retry
for banned in ("INJECTED_FAULT","sudo podman","--privileged","--publish",
               "--network host","0.0.0.0","192.168.","VM100","pct exec","ssh "):
 assert banned not in s,banned
start=s.index("  meili_helper \"$who\" '\n",s.index("meili_probe()"))+len("  meili_helper \"$who\" '\n")
end=s.index("\n' \"$op\"",start)
compile(s[start:end],"p8cn_meili_probe.py","exec")
for token in ("pull_request:","contents: read","runs-on: ubuntu-24.04",
              "persist-credentials: false",
              "python3 scripts/test_p8cn_two_app_version_failure_contract.py",
              "bash -n scripts/test_p8cn_two_app_version_failure_rollback.sh",
              "bash scripts/test_p8cn_two_app_version_failure_rollback.sh",
              "cargo build --locked -p hars-native-hub --bin oci_app_manager_lab"):
 assert token in w,token
assert "workflow_dispatch:" not in w
print("P8CN_TWO_APP_SHA_VALID_RUNTIME_MISMATCH_SCOPE_PASS")
