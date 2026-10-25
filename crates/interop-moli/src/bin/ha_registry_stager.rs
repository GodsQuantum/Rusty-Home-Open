//! Offline CLI for safely staging real HA Core entity registry identities.
//! Explicitly NOT a full HAOS migration, integration runtime or automation engine.
use anyhow::{Context, Result, bail};
use hars_interop_moli::registry_stage::RegistryStager;
use std::{
    env,
    fs::OpenOptions,
    io::Read,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
};
const LIMIT: u64 = 24 * 1024 * 1024;
fn import_file(path: &Path) -> Result<Vec<u8>> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(nix::libc::O_NOFOLLOW | nix::libc::O_NONBLOCK | nix::libc::O_CLOEXEC)
        .open(path)
        .context("unsafe/unavailable registry source")?;
    let m = file.metadata()?;
    if !m.is_file() || m.len() > LIMIT || m.nlink() != 1 {
        bail!("entity registry source must be a bounded regular file without hardlinks");
    }
    let mut bytes = Vec::with_capacity((m.len() as usize).min(1024 * 1024));
    file.take(LIMIT + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > LIMIT {
        bail!("registry source exceeds limit")
    }
    Ok(bytes)
}
fn run() -> Result<()> {
    let mut args = env::args().skip(1);
    let Some(marker) = args.next() else {
        bail!("missing --lab-readonly")
    };
    if marker != "--lab-readonly" {
        bail!("this CLI only supports lab-readonly")
    }
    let dir = args
        .next()
        .ok_or_else(|| anyhow::anyhow!("missing existing private directory"))?;
    let verb = args
        .next()
        .ok_or_else(|| anyhow::anyhow!("missing stage/activate/rollback/status"))?;
    let db = RegistryStager::open(Path::new(&dir))?;
    let output = match verb.as_str() {
        "stage" => {
            let f = args
                .next()
                .ok_or_else(|| anyhow::anyhow!("missing read-only export path"))?;
            if args.next().is_some() {
                bail!("unexpected arguments")
            }
            let bytes = import_file(Path::new(&f))?;
            let stage = db.stage(&bytes)?;
            serde_json::json!({"action":"stage","sha256":stage.batch_sha256,
                "entities":stage.entity_count,"stable_identities":stage.stable_count,
                "unstable_identities":stage.entity_count-stage.stable_count,
                "read_only":true,"live_states_migrated":false,"production_ready":false})
        }
        "import-unprotected-backup" => {
            let path = args
                .next()
                .ok_or_else(|| anyhow::anyhow!("missing full backup archive"))?;
            if args.next().is_some() {
                bail!("unexpected arguments")
            }
            let source = hars_interop_moli::ha_backup_import::registry_from_unprotected_backup(
                Path::new(&path),
            )?;
            let stage = db.stage(&source.raw)?;
            serde_json::json!({"action":"stage_official_style_unprotected_backup",
                "sha256":stage.batch_sha256,"archive_sha256":source.archive_sha256,
                "entities":stage.entity_count,"stable_identities":stage.stable_count,
                "read_only":true,"activated":false,"integrations_migrated":false,
                "automations_migrated":false,"apps_migrated":false,
                "production_ready":false})
        }
        "activate" => {
            let sha = args
                .next()
                .ok_or_else(|| anyhow::anyhow!("missing digest"))?;
            let expected = args
                .next()
                .ok_or_else(|| anyhow::anyhow!("missing expected previous digest or none"))?;
            if args.next().is_some() {
                bail!("unexpected arguments")
            }
            db.activate(
                &sha,
                if expected == "none" {
                    None
                } else {
                    Some(&expected)
                },
            )?;
            serde_json::json!({"action":"activate_shadow","sha256":sha,"read_only":true,
                "no_driver_commands":true,"production_ready":false})
        }
        "rollback" => {
            let sha = args
                .next()
                .ok_or_else(|| anyhow::anyhow!("missing expected active digest"))?;
            if args.next().is_some() {
                bail!("unexpected arguments")
            }
            let previous = db.rollback(&sha)?;
            serde_json::json!({"action":"rollback_shadow","reverted_from":sha,
                "restored":previous,"no_driver_commands":true,"production_ready":false})
        }
        "status" => {
            if args.next().is_some() {
                bail!("unexpected arguments")
            }
            let active = db.active()?;
            serde_json::json!({"action":"status","active":active.as_ref().map(|s|s.batch_sha256.clone()),
                "entities":active.as_ref().map_or(0,|s|s.entity_count),
                "state_known":false,"no_driver_commands":true,"production_ready":false})
        }
        _ => bail!("unknown safe lab verb"),
    };
    println!("{}", serde_json::to_string(&output)?);
    Ok(())
}
fn main() {
    if run().is_err() {
        // The input may be a private .storage file: never echo a path, token,
        // entity name, registry contents, or parse error into logs.
        eprintln!("HA_REGISTRY_STAGING_FAIL_CLOSED");
        std::process::exit(2);
    }
}
