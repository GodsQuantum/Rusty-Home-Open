#!/usr/bin/env python3
"""P8cf: eighth distinct real stateful OCI App, official Qdrant vector API."""
from pathlib import Path
r=Path(__file__).resolve().parent.parent
s=(r/"scripts/test_p8cf_qdrant_vector_restore.sh").read_text()
ci=(r/".github/workflows/p8aw-custom-volume-registry.yml").read_text()
for item in (
    '[[ "$(printenv GITHUB_ACTIONS || true)" == true',
    'ghcr.io/qdrant/qdrant/qdrant:v1.20.0-unprivileged',
    'ghcr.io/qdrant/qdrant/qdrant@sha256:',
    'docker.io/library/python@sha256:',
    '--install-path-lab',
    '1000 /qdrant/storage',
    '--network "container:$container"',
    'base="http://127.0.0.1:6333"',
    '/collections/"+collection+"/points?wait=true',
    '/collections/"+collection+"/points',
    '"distance":"Dot"',
    '"id":1', '"id":2',
    'probe rh-vector-source init',
    'probe rh-vector-source after',
    'probe rh-vector-restore first',
    'probe rh-vector-source both',
    'podman volume export --output "$ARCHIVE" "$source_vol"',
    'podman volume import "$restore_vol" "$ARCHIVE"',
    'grep -Eq \'(^|/)collections/\'',
    'sha256sum -c',
    'P8CF_QDRANT_STOPPED_REAL_VECTOR_WAL_VOLUME_SHA_TAMPER_REFUSED_PASS',
    'P8CF_EIGHTH_REAL_VECTOR_DATABASE_INDEPENDENT_RESTORE_POINT_IN_TIME_PASS',
    'P8CF_RESTORED_VECTOR_APP_REMOVAL_PRESERVES_SOURCE_AND_CHECKPOINT_PASS',
    "test \"$(podman inspect \"$1\" --format '{{.HostConfig.NetworkMode}}')\" = none",
    "test \"$(podman inspect \"$1\" --format '{{.HostConfig.ReadonlyRootfs}}')\" = true",
):
    assert item in s,item
assert "sudo podman" not in s and "192.168." not in s
assert "PublishPort=" not in s
for item in ("test_p8cf_qdrant_restore_contract.py",
             "bash -n scripts/test_p8cf_qdrant_vector_restore.sh",
             "bash scripts/test_p8cf_qdrant_vector_restore.sh"):
    assert item in ci,item
start=s.index(" python -B -c '\n")+len(" python -B -c '\n")
end=s.index("\n' \"$action\"",start)
compile(s[start:end],"qdrant_probe.py","exec")
print("P8CF_EIGHTH_QDRANT_REAL_VECTOR_REST_API_OFFLINE_INDEPENDENT_RESTORE_CONTRACT_PASS")
