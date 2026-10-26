from pathlib import Path
root=Path(__file__).resolve().parent.parent
w=(root/'.github/workflows/p8ai-real-core-frontend.yml').read_text()
src=(root/'crates/native-hub/src/haos_guest.rs').read_text()
cli=(root/'crates/native-hub/src/bin/haos_guest_service_lab.rs').read_text()
for marker in ('--lab-core-frontend-ready','HARS_LAB_AUTO_STOP_SECONDS=240','P8AI_REAL_OFFICIAL_HAOS_CORE_FRONTEND_HTTP_VIA_RUST_PASS','core_frontend_http_responded'):
 assert marker in w,marker
for marker in ('local_core_frontend_responded','verify_core_frontend_via_rust_guest_lab','127.0.0.1','GuestHealthTimeout'):
 assert marker in src,marker
assert '--lab-core-frontend-ready' in cli
assert 'hostfwd=tcp:127.0.0.1:18180-:80' in src
assert '192.168.' not in w
print('P8AI_GENUINE_HAOS_READONLY_CORE_HTTP_PROOF_CONTRACT_PASS')
