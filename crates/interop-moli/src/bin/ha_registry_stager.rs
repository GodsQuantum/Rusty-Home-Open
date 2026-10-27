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
            let (stage, automation) = db.stage_backup_source(
                &source.raw,
                source
                    .automations_yaml
                    .as_ref()
                    .map(|_| source.archive_sha256.as_str()),
                source.automations_yaml.as_deref(),
            )?;
            serde_json::json!({"action":"stage_official_style_unprotected_backup",
                "opaque_automations_inactive": automation.is_some(),
                "automation_source_sha256": automation.as_ref().map(|x| x.automation_sha256.as_str()),
                "automation_source_bytes": automation.as_ref().map(|x| x.bytes).unwrap_or(0),
                "sha256":stage.batch_sha256,"archive_sha256":source.archive_sha256,
                "entities":stage.entity_count,"stable_identities":stage.stable_count,
                "read_only":true,"activated":false,"integrations_migrated":false,
                "automations_migrated":false,"apps_migrated":false,
                "production_ready":false})
        }
        "automation-source-status" => {
            let archive = args
                .next()
                .ok_or_else(|| anyhow::anyhow!("missing archive digest"))?;
            if args.next().is_some() {
                bail!("unexpected arguments")
            }
            let result = db.read_backup_automations(&archive)?;
            serde_json::json!({"action":"automation_source_status",
                "source":result.as_ref().map(|x| &x.0),
                "content_returned":false,
                "actions_executable":false,"production_ready":false})
        }
        "app-source-ingest" => {
            let source = args
                .next()
                .ok_or_else(|| anyhow::anyhow!("missing App-only backup"))?;
            if args.next().is_some() {
                bail!("extra App source arguments")
            }
            let info = hars_interop_moli::app_backup_import::inspect_unprotected_app_only(
                Path::new(&source),
            )?;
            let escrow = hars_interop_moli::backup_escrow::BackupEscrow::open(Path::new(&dir))?;
            let receipt = escrow.ingest(Path::new(&source))?;
            if receipt.sha256 != info.archive_sha256 {
                bail!("App escrow source changed after validation")
            }
            let copied = hars_interop_moli::app_backup_import::inspect_unprotected_app_only(
                &Path::new(&dir).join(format!("{}.tar", receipt.sha256)),
            )?;
            if copied.archive_sha256 != receipt.sha256 || copied.files != info.files {
                bail!("private App archive verification mismatch")
            }
            serde_json::json!({"action":"private_app_backup_escrow_ingest",
                "sha256":receipt.sha256,"bytes":receipt.bytes,
                "app_count":info.app_count,"files":info.files,
                "declared_payload_bytes":info.declared_payload_bytes,
                "read_only":true,"app_runtime_migrated":false,"apps_restored":false,
                "actions_executable":false,"production_ready":false})
        }
        "app-source-stage-inactive" => {
            let digest = args
                .next()
                .ok_or_else(|| anyhow::anyhow!("missing App archive digest"))?;
            let slug = args
                .next()
                .ok_or_else(|| anyhow::anyhow!("missing single App slug"))?;
            let target = args
                .next()
                .ok_or_else(|| anyhow::anyhow!("missing precreated private native target root"))?;
            if args.next().is_some() {
                bail!("unexpected App stage arguments")
            }
            if digest.len() != 64
                || !digest
                    .bytes()
                    .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
            {
                bail!("invalid App archive digest")
            }
            let root = Path::new(&target);
            if root == Path::new(&dir) {
                bail!("source and native destination must differ")
            }
            let source = Path::new(&dir).join(format!("{digest}.tar"));
            let stage = hars_interop_moli::app_backup_stage::stage_inactive_app(
                &source, &digest, &slug, root,
            )?;
            serde_json::json!({"action":"stage_inactive_native_app_files",
                "sha256":stage.archive_sha256, "app_count":1,"files":stage.files,
                "bytes":stage.file_bytes,"staged_private_regular_files":stage.copied_as_private_regular_files,
                "app_image_installed":false,"app_runtime_started":false,
                "automation_executed":false,"production_ready":false})
        }
        "app-source-stage-pair-inactive" => {
            let pair_id = args
                .next()
                .ok_or_else(|| anyhow::anyhow!("missing pair ID"))?;
            let first_slug = args
                .next()
                .ok_or_else(|| anyhow::anyhow!("missing first App"))?;
            let first_digest = args
                .next()
                .ok_or_else(|| anyhow::anyhow!("missing first digest"))?;
            let second_slug = args
                .next()
                .ok_or_else(|| anyhow::anyhow!("missing second App"))?;
            let second_digest = args
                .next()
                .ok_or_else(|| anyhow::anyhow!("missing second digest"))?;
            let target = args
                .next()
                .ok_or_else(|| anyhow::anyhow!("missing private target"))?;
            if args.next().is_some() {
                bail!("extra pair arguments");
            }
            let receipt = hars_interop_moli::app_backup_pair_stage::stage_inactive_pair(
                Path::new(&dir),
                (&first_slug, &first_digest),
                (&second_slug, &second_digest),
                Path::new(&target),
                &pair_id,
            )?;
            serde_json::to_value(receipt)?
        }
        "app-source-verify" => {
            let digest = args
                .next()
                .ok_or_else(|| anyhow::anyhow!("missing content digest"))?;
            if args.next().is_some() {
                bail!("extra App verification arguments")
            }
            if digest.len() != 64
                || !digest
                    .bytes()
                    .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
            {
                bail!("invalid source digest")
            }
            let path = Path::new(&dir).join(format!("{digest}.tar"));
            let info = hars_interop_moli::app_backup_import::inspect_unprotected_app_only(&path)?;
            if info.archive_sha256 != digest {
                bail!("private archive was altered")
            }
            serde_json::json!({"action":"private_app_backup_escrow_verify",
                "archive_sha256":digest,"app_count":info.app_count,
                "files":info.files,"declared_payload_bytes":info.declared_payload_bytes,
                "read_only":true,"apps_restored":false,"actions_executable":false,
                "production_ready":false})
        }
        "escrow-ingest" => {
            let source = args
                .next()
                .ok_or_else(|| anyhow::anyhow!("missing full archive source"))?;
            if args.next().is_some() {
                bail!("extra arguments")
            }
            let escrow = hars_interop_moli::backup_escrow::BackupEscrow::open(Path::new(&dir))?;
            serde_json::to_value(escrow.ingest(Path::new(&source))?)?
        }
        "escrow-export" => {
            let key = args
                .next()
                .ok_or_else(|| anyhow::anyhow!("missing escrow digest"))?;
            let dest = args
                .next()
                .ok_or_else(|| anyhow::anyhow!("missing private export directory"))?;
            if args.next().is_some() {
                bail!("extra arguments")
            }
            let escrow = hars_interop_moli::backup_escrow::BackupEscrow::open(Path::new(&dir))?;
            serde_json::to_value(escrow.export(&key, Path::new(&dest))?)?
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
