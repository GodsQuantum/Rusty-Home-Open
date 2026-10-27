#!/usr/bin/env python3
"""P8cg: ninth distinct stateful upstream App, real full-text search + independent restore."""
from pathlib import Path

root = Path(__file__).resolve().parent.parent
script = (root / "scripts/test_p8cg_meilisearch_fulltext_restore.sh").read_text()
workflow = (root / ".github/workflows/p8cg-meilisearch-fulltext-restore.yml").read_text()
for token in (
    '[[ "${GITHUB_ACTIONS:-}" == true',
    '"${RUNNER_TEMP:-}"',
    '$(id -u)" != 0',
    'docker.io/getmeili/meilisearch:v1.55.0',
    'docker.io/getmeili/meilisearch@sha256:',
    'docker.io/library/python@sha256:',
    '--install-path-resources-lab "$source_slug" "$IMAGE" 1000 /meili_data 1024 256',
    '--install-path-resources-lab "$restore_slug" "$IMAGE" 1000 /meili_data 1024 256',
    '--network "container:$container"',
    '--read-only',
    '--cap-drop ALL',
    '--security-opt no-new-privileges',
    '/indexes/"+index+"/documents',
    '/indexes/"+index+"/search?q="+term',
    'await_task(task["taskUid"])',
    'first={"id":1,"title":"copper lantern","phase":"before"}',
    'second={"id":2,"title":"violet nebula","phase":"after"}',
    'probe rh-fulltext-source init',
    'probe rh-fulltext-source after',
    'probe rh-fulltext-restore first',
    'probe rh-fulltext-source both',
    'podman volume export --output "$ARCHIVE" "$source_vol"',
    'podman volume import "$restore_vol" "$ARCHIVE"',
    'test "$(stat -c %a "$ARCHIVE")" = 600',
    "grep -Eq '(^|/)data.ms/'",
    'sha256sum -c',
    'P8CG_NINTH_REAL_FULLTEXT_APP_INDEPENDENT_INDEX_RESTORE_POINT_IN_TIME_PASS',
    'P8CG_RESTORED_SEARCH_APP_REMOVAL_PRESERVES_ORIGINAL_INDEX_PASS',
):
    assert token in script, token
assert "sudo podman" not in script
assert "PublishPort=" not in script and "--publish" not in script
assert "192.168." not in script and "VM100" not in script
assert 'podman run --rm --network "container:$container"' in script
assert 'test "$(podman inspect "$1" --format \'{{.HostConfig.NetworkMode}}\')" = none' in script
assert 'test "$(podman inspect "$1" --format \'{{.HostConfig.ReadonlyRootfs}}\')" = true' in script
start = script.index("  helper \"$container\" '\n", script.index("probe()")) + len("  helper \"$container\" '\n")
end = script.index("\n' \"$action\"", start)
compile(script[start:end], "real_meilisearch_rest_probe.py", "exec")
for token in (
    "test_p8cg_meilisearch_restore_contract.py",
    "bash -n scripts/test_p8cg_meilisearch_fulltext_restore.sh",
    "bash scripts/test_p8cg_meilisearch_fulltext_restore.sh",
    "runs-on: ubuntu-24.04",
    "timeout-minutes: 20",
):
    assert token in workflow, token
assert "workflow_dispatch:" not in workflow
print("P8CG_NINTH_MEILISEARCH_FULLTEXT_OFFLINE_REAL_API_RESTORE_CONTRACT_PASS")
