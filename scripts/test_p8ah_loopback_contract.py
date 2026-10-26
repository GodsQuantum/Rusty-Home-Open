from pathlib import Path
root=Path(__file__).resolve().parent.parent
wf=(root/'.github/workflows/p8ah-loopback-ui.yml').read_text()
src=(root/'crates/native-hub/src/haos_guest.rs').read_text()
assert 'P8AH_QEMU_HOST_LOCALHOST_UI_FORWARD_PASS' in wf
assert 'loopback_frontend_forward_requested' in wf
assert 'loopback_listener_reached' in wf
assert '--lab-service-loopback-ui' in wf
assert 'run_guarded_guest_service_http_loopback_lab' in src
assert '127.0.0.1:18180' in src
assert 'hostfwd=tcp:127.0.0.1:18180-:80' in src
print('P8AH_EXACT_LOOPBACK_ONLY_CORE_FORWARD_CONTRACT_PASS')
