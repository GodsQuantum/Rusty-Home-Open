//! Independent CI-only read-only inspection of a synthetic App repository.
//! No arbitrary filesystem traversal, Docker, Supervisor API or installation.
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;

fn main() {
    if inspect().is_err() {
        eprintln!("REFUSED: unsafe, invalid or out-of-scope Supervisor App repository");
        std::process::exit(1);
    }
}
fn inspect() -> Result<(), Box<dyn std::error::Error>> {
    let raw: Vec<String> = std::env::args().collect();
    if raw.len() != 2 {
        return Err("expected one synthetic repository path".into());
    }
    let runtime = std::env::current_dir()?.join("runtime").canonicalize()?;
    let input = Path::new(&raw[1]);
    let metadata = std::fs::symlink_metadata(input)?;
    if !metadata.is_dir()
        || metadata.file_type().is_symlink()
        || metadata.uid() != nix::unistd::Uid::effective().as_raw()
        || metadata.permissions().mode() & 0o077 != 0
    {
        return Err("not a private test directory".into());
    }
    let canonical = input.canonicalize()?;
    if canonical.parent() != Some(runtime.as_path()) {
        return Err("outside synthetic runtime directory".into());
    }
    let report = hars_supervisor_apps::inspect_repository(&canonical)
        .map_err(|_| "invalid Supervisor app configuration")?;
    println!("{}", serde_json::to_string(&report)?);
    Ok(())
}
