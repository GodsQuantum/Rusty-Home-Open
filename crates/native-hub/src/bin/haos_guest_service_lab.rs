//! Experimental Rust host-controlled HAOS guest lifecycle; disposable lab only.
use hars_native_hub::haos_guest::{
    GuestSpec, run_guarded_guest_service_http_loopback_lab, run_guarded_guest_service_lab,
};
use std::{env, path::PathBuf};
fn main() {
    let result = run();
    match result {
        Ok(out) => println!(
            "{}",
            serde_json::json!({
             "native_rust_continuous_guest_manager":true,
             "guest_process_reaped":out.guest_process_reaped,
             "official_haos_base_image_unchanged":out.base_image_unchanged,
             "acpi_powerdown_requested":out.acpi_shutdown_requested,
             "local_private_qmp_only":true,
             "loopback_frontend_forward_requested":out.loopback_frontend_forward_requested,
             "loopback_listener_reached":out.loopback_listener_reached,
             "radio_passthrough":false,"production_ready":false
            })
        ),
        Err(e) => {
            eprintln!("HAOS_GUEST_SERVICE_LAB_FAIL_CLOSED: {e}");
            std::process::exit(1)
        }
    }
}
fn run() -> Result<hars_native_hub::haos_guest::GuestServiceOutcome, Box<dyn std::error::Error>> {
    let a = env::args().collect::<Vec<_>>();
    if a.len() != 7
        || !matches!(
            a[1].as_str(),
            "--lab-service-run" | "--lab-service-loopback-ui"
        )
    {
        return Err("requires lab service QEMU OVMF QCOW2 SHA256 private-overlay-dir".into());
    }
    let s = GuestSpec {
        emulator: PathBuf::from(&a[2]),
        firmware: PathBuf::from(&a[3]),
        source_image: PathBuf::from(&a[4]),
        image_sha256: a[5].clone(),
    };
    if a[1] == "--lab-service-loopback-ui" {
        Ok(run_guarded_guest_service_http_loopback_lab(
            &s,
            &PathBuf::from(&a[6]),
        )?)
    } else {
        Ok(run_guarded_guest_service_lab(&s, &PathBuf::from(&a[6]))?)
    }
}
