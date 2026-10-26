//! No general-purpose guest launcher. Official QEMU lab-only proof.
use hars_native_hub::haos_guest::{
    GuestSpec, boot_guest_in_isolated_lab, guest_marker_in_separate_manager_lab,
    prove_guest_cow_state_across_restarts_in_lab, verify_supervisor_and_core_in_isolated_lab,
};
use std::{env, path::PathBuf};
fn main() {
    let result = run();
    let health = result.as_ref().is_ok_and(|mode| mode.as_str() == "health");
    let persistent = result
        .as_ref()
        .is_ok_and(|mode| mode.as_str() == "persisted");
    let provisioned = result
        .as_ref()
        .is_ok_and(|mode| mode.as_str() == "provisioned");
    let reattached = result
        .as_ref()
        .is_ok_and(|mode| mode.as_str() == "reattached");
    match result {
        Ok(_) => println!(
            "{}",
            serde_json::json!({
                "official_haos_guest_login":true,
                "native_rust_qemu_process":true,
                "read_only_ephemeral":!(persistent || provisioned || reattached),
                "immutable_source_image":true,
                "no_host_device_passthrough":true,
                "no_production_cutover":true,
                "official_supervisor_cli_ok": health,
                "official_core_cli_ok": health,
                "guest_state_persisted_across_restart":persistent,
                "guest_base_image_unchanged":persistent || provisioned || reattached,
                "guest_marker_created_by_first_rust_manager":provisioned,
                "guest_marker_verified_by_second_rust_manager":reattached,
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
    if (args.len() != 6 && args.len() != 7)
        || !matches!(
            args[1].as_str(),
            "--lab-boot-ephemeral"
                | "--lab-supervisor-core-health"
                | "--lab-persistent-restart"
                | "--lab-provision-marker"
                | "--lab-resume-marker"
        )
        || (matches!(
            args[1].as_str(),
            "--lab-persistent-restart" | "--lab-provision-marker" | "--lab-resume-marker"
        ) && args.len() != 7)
        || (matches!(
            args[1].as_str(),
            "--lab-boot-ephemeral" | "--lab-supervisor-core-health"
        ) && args.len() != 6)
    {
        return Err("requires --lab-boot-ephemeral QEMU OVMF QCOW2 SHA256".into());
    }
    let spec = GuestSpec {
        emulator: PathBuf::from(&args[2]),
        firmware: PathBuf::from(&args[3]),
        source_image: PathBuf::from(&args[4]),
        image_sha256: args[5].clone(),
    };
    if args[1] == "--lab-provision-marker" || args[1] == "--lab-resume-marker" {
        guest_marker_in_separate_manager_lab(
            &spec,
            &PathBuf::from(&args[6]),
            args[1] == "--lab-provision-marker",
        )?;
        Ok(if args[1] == "--lab-provision-marker" {
            "provisioned"
        } else {
            "reattached"
        }
        .into())
    } else if args[1] == "--lab-persistent-restart" {
        if args.len() != 7 {
            return Err("persistent restart requires private overlay directory".into());
        }
        prove_guest_cow_state_across_restarts_in_lab(&spec, &PathBuf::from(&args[6]))?;
        Ok("persisted".into())
    } else if args[1] == "--lab-supervisor-core-health" {
        verify_supervisor_and_core_in_isolated_lab(&spec)?;
        Ok("health".into())
    } else {
        boot_guest_in_isolated_lab(&spec)?;
        Ok("boot".into())
    }
}
