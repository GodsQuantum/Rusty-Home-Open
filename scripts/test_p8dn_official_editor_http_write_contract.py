#!/usr/bin/env python3
"""P8dn: official disposable File Editor HTTP POST, denial and durable restart."""
import ast
from pathlib import Path

root = Path(__file__).resolve().parents[1]
guest = (root / "scripts/p8dl_official_core_ingress_denial_pytest.py").read_text()
workflow = (root / ".github/workflows/p8dl-real-core-ingress-denial.yml").read_text()
for marker in (
    "P8DN_REAL_CORE_INGRESS_UNAUTHENTICATED_POST_NO_GUEST_FILE_PASS",
    "P8DN_REAL_OFFICIAL_EDITOR_HTTP_POST_SAVED_SYNTHETIC_FILE_PASS",
    "P8DN_REAL_OFFICIAL_EDITOR_HTTP_WRITE_GUEST_SHA256_PASS",
    "P8DN_REAL_OFFICIAL_EDITOR_HTTP_WRITTEN_DATA_SURVIVES_SUPERVISOR_RESTART_PASS",
    "test ! -e ",
    "sha256sum ",
    "ha apps stop ",
    "ha apps start ",
    "/mnt/data/supervisor/homeassistant/",
    "http://homeassistant:8123",
    "http://core-configurator:8099/api/save",
    "ingress_session=invalid-lab-session-value",
):
    assert marker in guest, marker
for marker in (
    "scripts/test_p8dn_official_editor_http_write_contract.py",
    "P8DN_REAL_OFFICIAL_FILE_EDITOR_HTTP_POST_AND_RESTART_LAB_PASS",
    "test_hars_official_file_editor_ingress_unauthenticated_denial",
):
    assert marker in workflow, marker
for forbidden in ("192.168.", "qm stop 100", "pct exec", "pull_request_target:",
                  "workflow_dispatch:", "secrets.", "--privileged"):
    assert forbidden not in guest + workflow, forbidden
tree = ast.parse(guest)
scripts = [n.value.value for n in ast.walk(tree)
           if isinstance(n, ast.Assign)
           and any(isinstance(t, ast.Name) and t.id == "script" for t in n.targets)
           and isinstance(n.value, ast.Constant)]
assert len(scripts) == 1
embedded = scripts[0]
ast.parse(embedded)
assert '"/homeassistant/"+name' in embedded
assert '"http://homeassistant:8123"+path.rstrip("/")+"/api/save"' in embedded
assert '"http://core-configurator:8099/api/save"' in embedded
assert "body.get(\"error\") is False" in embedded
# Exact test byte contract: the guest Python and QEMU host SHA fixture agree.
value_nodes = [n.value for n in ast.walk(ast.parse(embedded))
               if isinstance(n, ast.Assign)
               and any(isinstance(t, ast.Name) and t.id == "value" for t in n.targets)]
assert len(value_nodes) == 1
guest_bytes = ast.literal_eval(value_nodes[0]).encode("utf-8")
host_nodes = [n.value for n in ast.walk(tree)
              if isinstance(n, ast.Assign)
              and any(isinstance(t, ast.Name) and t.id == "p8dn_value" for t in n.targets)]
assert len(host_nodes) == 1 and guest_bytes == ast.literal_eval(host_nodes[0])
print("P8DN_OFFICIAL_REAL_HTTP_WRITE_SCOPE_AND_BYTES_CONTRACT_PASS")
