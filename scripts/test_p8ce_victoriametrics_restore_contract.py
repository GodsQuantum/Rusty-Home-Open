#!/usr/bin/env python3
"""Seventh distinct real time-series engine, rootless no-egress CI only."""
from pathlib import Path
r=Path(__file__).resolve().parent.parent
t=(r/"scripts/test_p8ce_victoriametrics_timeseries_restore.sh").read_text()
ci=(r/".github/workflows/p8aw-custom-volume-registry.yml").read_text()
for item in (
    '[[ "$(printenv GITHUB_ACTIONS || true)" == true',
    'docker.io/victoriametrics/victoria-metrics:v1.155.0',
    'docker.io/victoriametrics/victoria-metrics@sha256:',
    'docker.io/library/python@sha256:',
    '--install-path-lab',
    '10001 /victoria-metrics-data',
    '--network "container:$container"',
    'base="http://127.0.0.1:8428"',
    'base+"/api/v1/import/prometheus"',
    'http://127.0.0.1:8428',
    '/api/v1/query?',
    'p8ce_fixture_value',
    'tsdb_http rh-timeseries-source write before',
    'tsdb_http rh-timeseries-source write after',
    'tsdb_http rh-timeseries-restore get before',
    'tsdb_http rh-timeseries-restore absent after',
    'podman volume export --output "$ARCHIVE" "$source_vol"',
    'podman volume import "$restore_vol" "$ARCHIVE"',
    'sha256sum -c',
    'P8CE_REAL_VICTORIAMETRICS_STOPPED_VOLUME_SHA_TAMPER_REFUSED_PASS',
    'P8CE_SEVENTH_REAL_TSDB_INDEPENDENT_METRICS_CHECKPOINT_RESTORE_PASS',
    'P8CE_RESTORED_TIMESERIES_APP_REMOVAL_PRESERVES_SOURCE_PASS',
    "test \"$(podman inspect \"$1\" --format '{{.HostConfig.NetworkMode}}')\" = none",
    "test \"$(podman inspect \"$1\" --format '{{.HostConfig.ReadonlyRootfs}}')\" = true",
):
    assert item in t,item
assert "sudo podman" not in t and "192.168." not in t
assert "PublishPort=" not in t
for item in ("test_p8ce_victoriametrics_restore_contract.py",
             "bash -n scripts/test_p8ce_victoriametrics_timeseries_restore.sh",
             "bash scripts/test_p8ce_victoriametrics_timeseries_restore.sh"):
    assert item in ci,item
print("P8CE_SEVENTH_REAL_VICTORIAMETRICS_NO_NETWORK_PROMETHEUS_QUERY_RESTORE_CONTRACT_PASS")
