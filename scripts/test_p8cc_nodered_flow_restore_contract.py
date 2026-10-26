#!/usr/bin/env python3
"""P8cc: fifth distinct real stateful App, official Node-RED HTTP API."""
from pathlib import Path
r=Path(__file__).resolve().parent.parent
test=(r/"scripts/test_p8cc_node_red_stateful_flows_restore.sh").read_text()
wf=(r/".github/workflows/p8aw-custom-volume-registry.yml").read_text()
for item in (
    '[[ "$(printenv GITHUB_ACTIONS || true)" == true',
    'docker.io/nodered/node-red:latest',
    'docker.io/nodered/node-red@sha256:',
    '--install-lab "$source_slug" "$IMAGE" 1000',
    '--install-lab "$restore_slug" "$IMAGE" 1000',
    'http://127.0.0.1:1880/flows',
    '"Node-RED-API-Version":"v1"',
    'flow_api rh-red-flow-source post first',
    'flow_api rh-red-flow-source post both',
    'flow_api rh-red-flow-restore get first',
    'flow_api rh-red-flow-source get both',
    'podman volume export --output "$ARCHIVE" "$source_vol"',
    'podman volume import "$restore_vol" "$ARCHIVE"',
    'grep -Eq \'(^|/)flows.json$\'',
    'sha256sum -c',
    'test "$(stat -c %a "$ARCHIVE")" = 600',
    'P8CC_OFFICIAL_NODE_RED_STOPPED_FLOW_VOLUME_SHA_TAMPER_REFUSAL_PASS',
    'P8CC_FIFTH_REAL_AUTOMATION_APP_NATIVE_FLOW_INDEPENDENT_RESTORE_PASS',
    'P8CC_NODE_RED_RESTORED_APP_REMOVE_PRESERVES_SOURCE_AND_SNAPSHOT_PASS',
    'test "$(podman inspect "$1" --format \'{{.HostConfig.NetworkMode}}\')" = none',
    'test "$(podman inspect "$1" --format \'{{.HostConfig.ReadonlyRootfs}}\')" = true',
):
    assert item in test,item
assert "sudo podman" not in test
assert "192.168." not in test
assert 'PublishPort=' not in test
for item in (
    "scripts/test_p8cc_node_red_stateful_flows_restore.sh",
    "scripts/test_p8cc_nodered_flow_restore_contract.py",
    "bash scripts/test_p8cc_node_red_stateful_flows_restore.sh",
):
    assert item in wf,item
print("P8CC_FIFTH_OFFICIAL_NODE_RED_REAL_HTTP_FLOW_RESTORE_OFFLINE_CONTRACT_PASS")
