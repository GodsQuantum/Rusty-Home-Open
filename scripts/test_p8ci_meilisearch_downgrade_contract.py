#!/usr/bin/env python3
"""P8ci: checksum-valid application-version downgrade is not a safe restore."""
from pathlib import Path
root=Path(__file__).resolve().parent.parent
s=(root/"scripts/test_p8ci_meilisearch_downgrade.sh").read_text()
w=(root/".github/workflows/p8ci-meilisearch-downgrade.yml").read_text()
for token in (
 "GITHUB_ACTIONS", "RUNNER_TEMP", "$(id -u)", "set -euo pipefail",
 "docker.io/getmeili/meilisearch:v1.55.0",
 "docker.io/getmeili/meilisearch:v1.54.0",
 "docker.io/getmeili/meilisearch@sha256:",
 "--install-path-resources-lab", "--network \"container:rh-$who\"",
 "--read-only", "--cap-drop ALL", "--security-opt no-new-privileges",
 "podman volume export --output", "podman volume import",
 "sha256sum -c", "journalctl --user", "cleanup_stage",
 "P8CI_REAL_155_OFFLINE_SHA_CHECKED_PRIVATE_CHECKPOINT_PASS",
 "P8CI_REAL_154_REFUSES_155_CHECKSUM_VALID_DATABASE_DOWNGRADE_PASS",
 "P8CI_FAILED_DOWNGRADE_STAGE_REMOVED_ORIGINAL_155_LIVE_PASS",
 "P8CI_COMPATIBLE_155_RETRY_RESTORES_OLD_INDEX_ORIGINAL_UNCHANGED_PASS",
 "P8CI_RESTORED_APP_REMOVAL_PRESERVES_NEWER_SOURCE_DATA_PASS",
):
 assert token in s,token
# The failed old-binary start must occur AFTER a verified archive import,
# and the cleanup BEFORE retry with the compatible image.
a=s.index('podman volume import "rh-$STAGE-data" "$WORK/checkpoint.tar"')
b=s.index('install "$STAGE" "$OLD"',a)
c=s.index('P8CI_REAL_154_REFUSES_155_CHECKSUM_VALID_DATABASE_DOWNGRADE_PASS',b)
d=s.index("cleanup_stage",c)
e=s.index('install "$STAGE" "$NEW"',d)
assert a < b < c < d < e
for banned in ("sudo podman","--privileged","--publish","--network host",
               "0.0.0.0","192.168.","pct exec","ssh "):
 assert banned not in s,banned
start=s.index("  helper \"$who\" '\n",s.index("probe()"))+len("  helper \"$who\" '\n")
end=s.index("\n' \"$mode\"",start)
compile(s[start:end],"p8ci_genuine_meili_probe.py","exec")
for token in ("pull_request:","permissions:","contents: read",
 "runs-on: ubuntu-24.04","persist-credentials: false",
 "python3 scripts/test_p8ci_meilisearch_downgrade_contract.py",
 "bash -n scripts/test_p8ci_meilisearch_downgrade.sh",
 "bash scripts/test_p8ci_meilisearch_downgrade.sh",
 "cargo build --locked -p hars-native-hub --bin oci_app_manager_lab",
 "podman uidmap slirp4netns fuse-overlayfs"):
 assert token in w,token
assert "workflow_dispatch:" not in w
print("P8CI_REAL_DOWNGRADE_REFUSAL_ISOLATED_CONTRACT_PASS")
