//! Offline, non-production Rust-only pipeline test launcher.
/// Keep process-kill lab state only in a disposable child of this repository's
/// runtime directory; refuse symlinks, other projects, and arbitrary paths.
fn lab_registry_dir(raw: &str) -> anyhow::Result<std::path::PathBuf> {
    let base = std::env::current_dir()?.join("runtime").canonicalize()?;
    let root = std::path::Path::new(raw).canonicalize()?;
    if root.parent() != Some(base.as_path()) {
        anyhow::bail!("registry lab path must be a private child of project runtime");
    }
    Ok(root)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() == 4 && args[1] == "--lab-ha-core-state-snapshot-readonly" {
        use std::io::Read;
        use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
        if nix::unistd::Uid::effective().is_root() {
            anyhow::bail!("no root execution for Home Assistant state snapshot");
        }
        let path = std::path::Path::new(&args[3]);
        let file = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(nix::libc::O_NOFOLLOW | nix::libc::O_NONBLOCK | nix::libc::O_CLOEXEC)
            .open(path)?;
        let meta = file.metadata()?;
        if !meta.is_file()
            || meta.len() > 24 * 1024 * 1024
            || meta.nlink() != 1
            || meta.uid() != nix::unistd::Uid::current().as_raw()
            || meta.permissions().mode() & 0o077 != 0
        {
            anyhow::bail!("unsafe private Core state snapshot file")
        }
        let mut data = Vec::new();
        file.take(24 * 1024 * 1024 + 1).read_to_end(&mut data)?;
        if data.len() > 24 * 1024 * 1024 {
            anyhow::bail!("oversized Core state snapshot")
        }
        let proof = hars_native_kernel::ha_registry_shadow::observe_states(
            std::path::Path::new(&args[2]),
            &data,
        )
        .await?;
        println!("{}", serde_json::to_string(&proof)?);
        return Ok(());
    }
    if args.len() == 6 && args[1] == "--lab-ha-ws-core-token-rotation" {
        if nix::unistd::Uid::effective().is_root() {
            anyhow::bail!("Core token rotation lab forbids root");
        }
        let root = lab_registry_dir(&args[2])?;
        let old = std::env::var("HARS_LAB_WS_TOKEN")
            .map_err(|_| anyhow::anyhow!("old Core token unavailable"))?;
        let fresh = std::env::var("HARS_LAB_WS_REPLACEMENT_TOKEN")
            .map_err(|_| anyhow::anyhow!("replacement Core token unavailable"))?;
        let result = hars_native_kernel::ha_ws_runtime_lab::prove_official_core_token_rotation(
            &root, &args[3], &old, &fresh, &args[4], &args[5],
        )
        .await?;
        println!("{}", serde_json::to_string(&result)?);
        return Ok(());
    }
    if args.len() == 6 && args[1] == "--lab-ha-ws-reauth-recovery" {
        if nix::unistd::Uid::effective().is_root() {
            anyhow::bail!("WS recovery lab forbids root");
        }
        let dir = lab_registry_dir(&args[2])?;
        let token = std::env::var("HARS_LAB_WS_TOKEN")
            .map_err(|_| anyhow::anyhow!("lab token unavailable"))?;
        let report = hars_native_kernel::ha_ws_runtime_lab::prove_core_reauth(
            &dir, &args[3], &token, &args[4], &args[5],
        )
        .await?;
        println!("{}", serde_json::to_string(&report)?);
        return Ok(());
    }
    if args.len() == 7 && args[1] == "--lab-ha-ws-to-moli" {
        if nix::unistd::Uid::effective().is_root() {
            anyhow::bail!("read-only WS lab forbids root");
        }
        let dir = lab_registry_dir(&args[2])?;
        let required: usize = args[6].parse()?;
        // Sandbox token comes only from environment, not argv, URL,
        // subprocess flags or public output. Never log its contents.
        let token = std::env::var("HARS_LAB_WS_TOKEN")
            .map_err(|_| anyhow::anyhow!("lab WS token unavailable"))?;
        let proof = hars_native_kernel::ha_ws_runtime_lab::run(
            &dir, &args[3], &token, &args[4], &args[5], required,
        )
        .await?;
        println!("{}", serde_json::to_string(&proof)?);
        return Ok(());
    }
    if args.len() == 3 && args[1] == "--lab-ha-registry-shadow-readonly" {
        if nix::unistd::Uid::effective().is_root() {
            anyhow::bail!("no root execution for Core identity shadow");
        }
        let proof =
            hars_native_kernel::ha_registry_shadow::observe(std::path::Path::new(&args[2])).await?;
        println!("{}", serde_json::to_string(&proof)?);
        return Ok(());
    }
    if args.len() == 4 && args[1] == "--lab-ha-backup-automation-moli" {
        use anyhow::{Context, bail};
        use hars_interop_moli::registry_stage::RegistryStager;
        if nix::unistd::Uid::effective().is_root() {
            bail!("backup automation Moli lab cannot run as root");
        }
        let root = lab_registry_dir(&args[2])?;
        let db = RegistryStager::open(&root)?;
        let (source, bytes) = db
            .read_backup_automations(&args[3])?
            .context("missing protected and inactive HAOS automation source")?;
        if !source.inactive {
            bail!("automations must be inactive after import");
        }
        let active = db
            .active()?
            .context("HAOS Core identity stage not active")?;
        if active.batch_sha256 != source.registry_sha256 || !active.no_device_commands {
            bail!("untrusted or mismatched backup registry binding");
        }
        let original = std::str::from_utf8(&bytes)?;
        let plan = hars_native_kernel::automation_yaml::parse_disabled_backup_plan(original)?;
        for entity in [
            plan.source_entity.as_str(),
            plan.proposed_target.as_str(),
            plan.condition_entity.as_deref().unwrap_or(""),
        ] {
            if !entity.is_empty()
                && !active
                    .entities
                    .iter()
                    .any(|x| x.entity_id == entity && x.stable)
            {
                bail!("backup automation referenced identity absent from original registry");
            }
        }
        let runtime = std::env::current_dir()?.join("runtime").canonicalize()?;
        let folder = tempfile::Builder::new()
            .prefix("p8bi-")
            .tempdir_in(runtime)?;
        let proof = hars_native_kernel::automation_moli::prove_plan(folder.path(), &plan).await?;
        println!(
            "{}",
            serde_json::json!({
                "source":"genuine_haos_supervisor_full_archive",
                "archive_sha256":source.archive_sha256,
                "original_yaml_sha256":source.automation_sha256,
                "original_registry_sha256":source.registry_sha256,
                "one_explicitly_disabled_automation":true,
                "compiled_to_native_moli":proof.actual_native_moli_automation,
                "synthetic_actuator_confirmed":proof.synthetic_actuator_confirmed,
                "approval_required":proof.required_human_approval,
                "edit_revokes_approval":proof.approval_invalidated_by_edit,
                "persisted_edit_inactive":proof.persisted_edit_inactive,
                "stale_approval_rejected":proof.stale_approval_rejected,
                "no_run_after_edit":proof.no_run_after_edit,
                "restart_kept_edit_inactive":proof.restart_kept_edit_inactive,
                "real_hardware_touched":proof.hardware_touched,
                "production_ready":false
            })
        );
        return Ok(());
    }
    if args.len() == 2 && args[1] == "--lab-ha-automation-moli" {
        if nix::unistd::Uid::effective().is_root() {
            anyhow::bail!("P3b Moli YAML lab must run unprivileged");
        }
        let runtime = std::env::current_dir()?.join("runtime").canonicalize()?;
        let folder = tempfile::Builder::new()
            .prefix("hars-p3b-")
            .tempdir_in(runtime)?;
        let proof = hars_native_kernel::automation_moli::prove(folder.path()).await?;
        println!("{}", serde_json::to_string(&proof)?);
        return Ok(());
    }
    if args.len() == 7 && args[1] == "--lab-ha-automation-yaml" {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        let project_runtime = std::env::current_dir()?.join("runtime").canonicalize()?;
        let file = std::path::Path::new(&args[2]);
        let metadata = std::fs::symlink_metadata(file)?;
        if !metadata.is_file()
            || metadata.file_type().is_symlink()
            || metadata.len() > 8192
            || metadata.uid() != nix::unistd::Uid::current().as_raw()
            || metadata.permissions().mode() & 0o077 != 0
        {
            anyhow::bail!("unsafe HA YAML lab fixture file");
        }
        let resolved = file.canonicalize()?;
        let parent = resolved
            .parent()
            .ok_or_else(|| anyhow::anyhow!("no fixture parent"))?;
        let dir_metadata = std::fs::symlink_metadata(parent)?;
        if parent.parent() != Some(project_runtime.as_path())
            || dir_metadata.file_type().is_symlink()
            || dir_metadata.uid() != nix::unistd::Uid::current().as_raw()
            || dir_metadata.permissions().mode() & 0o077 != 0
        {
            anyhow::bail!("HA YAML lab requires a private runtime child");
        }
        let yaml = std::fs::read_to_string(&resolved)?;
        let plan = hars_native_kernel::automation_yaml::parse_plan(&yaml)?;
        let simulation = hars_native_kernel::automation_yaml::simulate(
            &plan, &args[3], &args[4], &args[5], &args[6],
        );
        println!(
            "{}",
            serde_json::json!({"plan":plan, "simulation":simulation})
        );
        return Ok(());
    }
    if args.len() == 4 && args[1] == "--lab-radio-lease" {
        let path = std::path::Path::new(&args[2]);
        let held = hars_native_kernel::radio_lease_lab::acquire(path)?;
        let result = serde_json::json!({
            "exclusive_writer":true, "lab_only":true,
            "real_radio_access":false, "production_ready":false
        });
        match args[3].as_str() {
            "probe" => {
                println!("{result}");
                drop(held);
                return Ok(());
            }
            "hold" => {
                println!("{result}");
                use std::io::Write;
                std::io::stdout().flush()?;
                loop {
                    std::thread::park();
                }
            }
            _ => anyhow::bail!("unsupported synthetic writer lease action"),
        }
    }
    if args.len() == 3 && args[1] == "--lab-mqtt-registry-stage" {
        use std::io::Write;
        let root = lab_registry_dir(&args[2])?;
        let store = hars_native_kernel::mqtt_registry::DiscoveryRegistry::open(&root)?;
        store.retain_validated(
            "homeassistant/sensor/bedroom/temp/config",
            br#"{"unique_id":"lab_bedroom","state_topic":"lab/readonly/bedroom"}"#,
        )?;
        println!("LAB_MQTT_REGISTRY_COMMITTED");
        std::io::stdout().flush()?;
        // Parent test will deliver true SIGKILL. No hardware/backend access.
        loop {
            std::thread::park();
        }
    }
    if args.len() == 3 && args[1] == "--lab-mqtt-registry-recover" {
        let root = lab_registry_dir(&args[2])?;
        let store = hars_native_kernel::mqtt_registry::DiscoveryRegistry::open(&root)?;
        let recovered = store.recover_unverified()?;
        println!(
            "{}",
            serde_json::json!({
                "registry_entries": recovered.len(),
                "stale_unverified": recovered.iter().filter(|r| r.stale_unverified).count(),
                "device_ownership_granted": false,
                "production_ready": false
            })
        );
        return Ok(());
    }
    let args: Vec<_> = std::env::args().collect();
    if args.len() == 4 && args[1] == "--lab-mqtt5-registry-observe" {
        let packets: usize = args[3].parse()?;
        let report =
            hars_native_kernel::mqtt5_reconcile::observe(std::path::Path::new(&args[2]), packets)
                .await?;
        println!("{}", serde_json::to_string(&report)?);
        return Ok(());
    }
    if args.len() == 3 && args[1] == "--lab-mqtt5-rap-observe" {
        let expected: usize = args[2].parse()?;
        let report = hars_native_kernel::mqtt5_rap::observe(expected).await?;
        println!("{}", serde_json::to_string(&report)?);
        return Ok(());
    }
    if args.len() == 4 && args[1] == "--lab-mqtt-sensor-observe" {
        let configs: usize = args[2].parse()?;
        let states: usize = args[3].parse()?;
        let report = hars_native_kernel::mqtt_live::observe_sensor_values(configs, states).await?;
        println!("{}", serde_json::to_string(&report)?);
        return Ok(());
    }
    if args.len() == 3 && args[1] == "--lab-mqtt-observe-ready" {
        let number: usize = args[2].parse()?;
        let report = hars_native_kernel::mqtt_live::observe_with_ready(number, true).await?;
        println!("{}", serde_json::to_string(&report)?);
        return Ok(());
    }
    if args.len() == 3 && args[1] == "--lab-mqtt-observe" {
        let number: usize = args[2].parse()?;
        let report = hars_native_kernel::mqtt_live::observe(number).await?;
        println!("{}", serde_json::to_string(&report)?);
        return Ok(());
    }
    if args.len() == 5 && args[1] == "--lab-mqtt-publish" {
        let file = std::path::Path::new(&args[3]);
        let metadata = std::fs::symlink_metadata(file)?;
        if !metadata.is_file() || metadata.len() > 65_536 {
            anyhow::bail!("invalid synthetic MQTT payload");
        }
        let payload = std::fs::read(file)?;
        let retained = match args[4].as_str() {
            "true" => true,
            "false" => false,
            _ => anyhow::bail!("retained must be true/false"),
        };
        hars_native_kernel::mqtt_live::publish(&args[2], &payload, retained).await?;
        println!(
            "{}",
            serde_json::json!({"published":true,"synthetic":true,"production_ready":false})
        );
        return Ok(());
    }
    if args.len() == 5 && args[1] == "--lab-mqtt-discovery" {
        let topic = &args[2];
        let path = std::path::Path::new(&args[3]);
        let bytes = std::fs::metadata(path)?.len();
        if bytes > 65536 {
            anyhow::bail!("MQTT fixture too large");
        }
        let payload = std::fs::read(path)?;
        let retained = match args[4].as_str() {
            "true" => true,
            "false" => false,
            _ => anyhow::bail!("retained must be true/false"),
        };
        use hars_native_kernel::mqtt_discovery::{Intent, preview};
        let result = preview(topic, &payload, retained)
            .map_err(|e| anyhow::anyhow!("synthetic MQTT discovery rejected: {e:?}"))?;
        let (kind, count) = match result {
            Intent::Preview { sensors, .. } => ("preview", sensors.len()),
            Intent::Remove { .. } => ("remove_intent", 0),
            Intent::Migrate { .. } => ("migration_intent", 0),
        };
        println!(
            "{}",
            serde_json::json!({
                "kind":kind,"sensors":count,
                "read_only":true,"no_broker":true,"native_rust":true,
                "production_ready":false,
            })
        );
        return Ok(());
    }
    if args.len() == 2 && args[1] == "--lab-driver-fencing" {
        let tmp = tempfile::tempdir()?;
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(tmp.path(), std::fs::Permissions::from_mode(0o700))?;
        let proof = hars_native_kernel::driver_fencing::prove(tmp.path()).await?;
        println!("{}", proof);
        return Ok(());
    }
    if args.len() >= 4 && args[1] == "--lab-fence" {
        let root = std::path::Path::new(&args[2]);
        let action = args[3].as_str();
        // A suspended worker may carry a stale epoch, but must not hold the
        // exclusive redb file lock. Open the database only when acting.
        if action == "delayed-apply" && args.len() == 8 {
            println!("LAB_FENCE_DEFERRED_READY");
            use std::io::Write;
            std::io::stdout().flush()?;
            std::thread::sleep(std::time::Duration::from_millis(1300));
        }
        let fence = hars_native_kernel::fencing::FencedSink::open(root)?;
        let output = match (action, args.len()) {
            ("first", 5) => serde_json::to_value(fence.grant_first(&args[4])?)?,
            ("transfer", 7) => {
                let n: u64 = args[5].parse()?;
                serde_json::to_value(fence.transfer(&args[4], n, &args[6])?)?
            }
            ("apply", 8) | ("delayed-apply", 8) => {
                let n: u64 = args[5].parse()?;
                let requested_on = match args[7].as_str() {
                    "true" => true,
                    "false" => false,
                    _ => anyhow::bail!("invalid bool"),
                };
                let committed = fence.apply(&args[4], n, &args[6], requested_on)?;
                serde_json::json!({"committed":committed,"synthetic":true})
            }
            ("state", 4) => serde_json::json!({
                "owner":fence.current_epoch()?,
                "actuator":fence.actuator()?,
                "synthetic":true,
                "production_ready":false
            }),
            _ => anyhow::bail!("unsupported local fencing command"),
        };
        println!("{}", serde_json::to_string(&output)?);
        return Ok(());
    }
    if args.len() >= 4 && args[1] == "--lab-ledger" {
        let root = std::path::Path::new(&args[2]);
        let ledger = hars_native_kernel::command_ledger::CommandLedger::open(root)?;
        let action = args[3].as_str();
        if action == "pending" && args.len() == 4 {
            println!(
                "{}",
                serde_json::to_string(&ledger.needs_reconciliation()?)?
            );
            return Ok(());
        }
        if action == "review" && args.len() == 5 {
            println!(
                "{}",
                serde_json::to_string(&ledger.reconciliation_review(&args[4])?)?
            );
            return Ok(());
        }
        let record = match (action, args.len()) {
            ("prepare", 6) => {
                let value = match args[5].as_str() {
                    "true" => true,
                    "false" => false,
                    _ => anyhow::bail!("expected true or false"),
                };
                ledger.prepare(&args[4], "hars-lab:actuator/on", value)?
            }
            ("claim", 5) => ledger.claim_once(&args[4])?,
            ("ack", 5) => ledger.acknowledge(&args[4])?,
            ("reject", 5) => ledger.reject(&args[4])?,
            ("quarantine", 6) => ledger.quarantine_uncertain(&args[4], &args[5])?,
            ("status", 5) => ledger
                .lookup(&args[4])?
                .ok_or_else(|| anyhow::anyhow!("missing id"))?,
            ("hold-inflight", 5) => {
                let result = ledger.claim_once(&args[4])?;
                println!("{}", serde_json::to_string(&result)?);
                use std::io::Write;
                std::io::stdout().flush()?;
                std::thread::sleep(std::time::Duration::from_secs(90));
                return Ok(());
            }
            _ => anyhow::bail!("unsupported local ledger fixture command"),
        };
        println!("{}", serde_json::to_string(&record)?);
        return Ok(());
    }
    if (args.len() == 5 || args.len() == 6) && args[1] == "--lab-daemon" {
        let state_dir = std::path::Path::new(&args[2]);
        let token = std::path::Path::new(&args[3]);
        let port: u16 = args[4].parse()?;
        let seed = args.len() == 6 && args[5] == "--seed-synthetic";
        if args.len() == 6 && !seed {
            anyhow::bail!("unsupported daemon argument");
        }
        return hars_native_kernel::daemon::run_lab_daemon(state_dir, token, port, seed).await;
    }
    if args.len() != 2 || args[1] != "--lab-self-test" {
        anyhow::bail!("lab only; usage: hars-native-kernel --lab-self-test");
    }
    let temp = tempfile::tempdir()?;
    let v = hars_native_kernel::native_cycle(temp.path()).await?;
    println!(
        "{}",
        serde_json::json!({
            "stage":"P1-native-synthetic",
            "native_rust":true,
            "used_python":v.used_python,
            "automation_live":v.automation_live,
            "actuator_state_confirmed":v.actuator_state_confirmed,
            "restored_sensor":v.restored_sensor,
            "restored_actuator":v.restored_actuator,
            "state_events":v.events_seen,
        "journal_entries_recovered":v.journal_entries_recovered,
        "http_read_ok":v.http_read_ok,
        "http_mutation_denied":v.http_mutation_denied,
            "production_ready":false
        })
    );
    Ok(())
}
