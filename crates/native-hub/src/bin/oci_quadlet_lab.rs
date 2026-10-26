//! Explicitly disposable lab-only rootless OCI Quadlet writer.
//! It does not start a service, install apps or touch systemd.
use hars_native_hub::oci_quadlet::OciApp;
use std::{
    env,
    fs::{self, OpenOptions},
    io::Write,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
};
fn main() {
    if execute().is_err() {
        eprintln!("OCI_QUADLET_LAB_FAIL_CLOSED");
        std::process::exit(1);
    }
    println!("P8AO_GENERIC_DIGEST_PINNED_ROOTLESS_QUADLET_GENERATED_PASS");
}
fn execute() -> Result<(), ()> {
    if env::var("HARS_DISPOSABLE_OCI_LAB").as_deref() != Ok("1") {
        return Err(());
    }
    let a = env::args().collect::<Vec<_>>();
    if a.len() != 5 || a[1] != "--generate-lab" {
        return Err(());
    }
    let units = OciApp {
        slug: &a[2],
        image: &a[3],
        memory_mib: 256,
        pids_limit: 128,
    }
    .render()
    .map_err(|_| ())?;
    let out = Path::new(&a[4]);
    let m = fs::symlink_metadata(out).map_err(|_| ())?;
    if !m.file_type().is_dir()
        || m.uid() != unsafe { nix::libc::geteuid() }
        || m.mode() & 0o077 != 0
    {
        return Err(());
    }
    for (name, data) in [
        (units.container_name, units.container),
        (units.volume_name, units.volume),
    ] {
        let file = out.join(name);
        let mut f = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(file)
            .map_err(|_| ())?;
        f.write_all(data.as_bytes()).map_err(|_| ())?;
        f.sync_all().map_err(|_| ())?;
    }
    Ok(())
}
