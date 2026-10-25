//! No general-purpose guest launcher. Official QEMU lab-only proof.
use hars_native_hub::haos_guest::{GuestSpec, boot_guest_in_isolated_lab};
use std::{env, path::PathBuf};
fn main() {
    let result = run();
    match result {
        Ok(()) => println!(
            "{}",
            serde_json::json!({
                "official_haos_guest_login":true,
                "native_rust_qemu_process":true,
                "read_only_ephemeral":true,
                "no_host_device_passthrough":true,
                "no_production_cutover":true,
                "production_ready":false
            })
        ),
        Err(err) => {
            eprintln!("HAOS_GUEST_LAB_FAIL_CLOSED: {err}");
            std::process::exit(1)
        }
    }
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = env::args().collect::<Vec<_>>();
    if args.len() != 6 || args[1] != "--lab-boot-ephemeral" {
        return Err("requires --lab-boot-ephemeral QEMU OVMF QCOW2 SHA256".into());
    }
    let spec = GuestSpec {
        emulator: PathBuf::from(&args[2]),
        firmware: PathBuf::from(&args[3]),
        source_image: PathBuf::from(&args[4]),
        image_sha256: args[5].clone(),
    };
    boot_guest_in_isolated_lab(&spec)?;
    Ok(())
}
