#!/usr/bin/env python3
"""Grafana stateful recovery integration and scope contract, no household."""
from pathlib import Path
r=Path(__file__).resolve().parent.parent
shell=(r/"scripts/test_p8cd_grafana_dashboard_sqlite_restore.sh").read_text()
ci=(r/".github/workflows/p8aw-custom-volume-registry.yml").read_text()
for v in (
    '[[ "$(printenv GITHUB_ACTIONS || true)" == true',
    'docker.io/grafana/grafana:12.2.0', 'docker.io/grafana/grafana@sha256:',
    '--install-path-lab', '/var/lib/grafana', ' 472 ',
    'http://127.0.0.1:3000/api/health',
    'http://127.0.0.1:3000/api/dashboards/db',
    'http://127.0.0.1:3000/api/dashboards/uid/',
    'p8cd-before', 'p8cd-after', 'absent_dashboard rh-dashboard-restore p8cd-after',
    'podman volume export --output "$ARCHIVE" "$primary_vol"',
    'podman volume import "$restored_vol" "$ARCHIVE"',
    'grep -Eq \'(^|/)grafana.db$\'',
    'sha256sum -c',
    'P8CD_GRAFANA_STOPPED_SQLITE_SHA256_TAMPER_DENIAL_PASS',
    'P8CD_SIXTH_REAL_GRAFANA_SQLITE_INDEPENDENT_DASHBOARD_RESTORE_PASS',
    'P8CD_RESTORED_GRAFANA_REMOVAL_LEAVES_SOURCE_AND_DATA_PASS',
    "test \"$(podman inspect \"$1\" --format '{{.HostConfig.NetworkMode}}')\" = none",
    "test \"$(podman inspect \"$1\" --format '{{.HostConfig.ReadonlyRootfs}}')\" = true",
):
    assert v in shell,v
assert 'sudo podman' not in shell
assert '192.168.' not in shell
assert 'PublishPort=' not in shell
assert "test_p8cd_grafana_restore_contract.py" in ci
assert "bash -n scripts/test_p8cd_grafana_dashboard_sqlite_restore.sh" in ci
assert "bash scripts/test_p8cd_grafana_dashboard_sqlite_restore.sh" in ci
print("P8CD_SIXTH_OFFICIAL_GRAFANA_SQLITE_REAL_REST_API_RESTORE_SCOPE_PASS")
