//! No general-purpose guest launcher. Official QEMU lab-only proof.
use hars_native_hub::haos_guest::{
    GuestSpec, boot_guest_in_isolated_lab, verify_supervisor_and_core_in_isolated_lab,
};
use std::{env, path::PathBuf};
fn main() {
    let result = run();
    let health = result.as_ref().is_ok_and(|mode| mode.as_str() == "health");
    match result {
        Ok(_) => println!(
            "{}",
            serde_json::json!({
                "official_haos_guest_login":true,
                "native_rust_qemu_process":true,
                "read_only_ephemeral":true,
                "no_host_device_passthrough":true,
                "no_production_cutover":true,
                "official_supervisor_cli_ok": health,
                "official_core_cli_ok": health,
                "production_ready":false
            })
        ),
        Err(err) => {
            eprintln!("HAOS_GUEST_LAB_FAIL_CLOSED: {err}");
            std::process::exit(1)
        }
    }
}
fn run() -> Result<String, Box<dyn std::error::Error>> {
    let args = env::args().collect::<Vec<_>>();
    if args.len() != 6
        || !matches!(
            args[1].as_str(),
            "--lab-boot-ephemeral" | "--lab-supervisor-core-health"
        )
    {
        return Err("requires --lab-boot-ephemeral QEMU OVMF QCOW2 SHA256".into());
    }
    let spec = GuestSpec {
        emulator: PathBuf::from(&args[2]),
        firmware: PathBuf::from(&args[3]),
        source_image: PathBuf::from(&args[4]),
        image_sha256: args[5].clone(),
    };
    if args[1] == "--lab-supervisor-core-health" {
        verify_supervisor_and_core_in_isolated_lab(&spec)?;
        Ok("health".into())
    } else {
        boot_guest_in_isolated_lab(&spec)?;
        Ok("boot".into())
    }
}
