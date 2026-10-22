#![no_main]
use hars_supervisor_apps::core_user_snapshot_lab::CoreUserSnapshot;
use libfuzzer_sys::fuzz_target;

// Genuine production-format decoder exercised under libFuzzer/ASan instrumentation,
// but with fixture-only Core user data and no network, token or permissions issued.
fuzz_target!(|wire: &[u8]| {
    let _ = CoreUserSnapshot::parse_fixture(wire, 17, 100);
});
