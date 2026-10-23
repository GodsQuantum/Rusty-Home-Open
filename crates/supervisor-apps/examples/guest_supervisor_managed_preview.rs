//! Entirely disposable HAOS QEMU test binary, not a production App manager.
//! The real release still needs authenticated user RBAC, confirmation, audit,
//! Job tracking, transactional rollback and comprehensive security review.
use hars_supervisor_apps::{
    audit_lab::FileAuditJournal,
    managed_apps::{AppSlug, BackupSlug, Operation, prepare},
    official_transport::{AppSupervisorClient, TransportError},
    rbac_lab::{Actor, Policy, Role},
    session_lab::LabSessionRegistry,
};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 3 || args[0] != "--acknowledge-disposable-qemu" {
        eprintln!("P8H: QEMU-only approval and action required");
        std::process::exit(2);
    }
    let action = args[1].as_str();
    let app = match AppSlug::parse(&args[2]) {
        Ok(a) => a,
        Err(_) => {
            eprintln!("P8H: invalid app identifier");
            std::process::exit(2);
        }
    };
    // LAB READ ONLY: no Supervisor mutation, no capability minted.
    if action == "probe-update" && args.len() == 3 {
        match AppSupervisorClient::from_environment()
            .and_then(|client| client.preview_probe_update_eligibility(&app))
        {
            Ok(()) => println!("P8JD_REAL_SUPERVISOR_UPDATE_READY"),
            Err(TransportError::BadResponse) => {
                println!("P8JD_REAL_SUPERVISOR_UPDATE_DENIED")
            }
            Err(kind) => {
                eprintln!("P8JD_UPDATE_PROBE_ERROR={kind:?}");
                std::process::exit(2);
            }
        }
        return;
    }
    let operation = match (action, args.len()) {
        ("install", 3) => Operation::Install,
        ("start", 3) => Operation::Start,
        ("stop", 3) => Operation::Stop,
        ("restart", 3) => Operation::Restart,
        ("update-with-backup", 3) => Operation::UpdateWithBackup,
        ("backup-partial", 3) => Operation::BackupPartial,
        ("restore-partial", 4) => {
            let backup = match BackupSlug::parse(&args[3]) {
                Ok(id) => id,
                Err(_) => {
                    eprintln!("P8H: invalid backup identifier");
                    std::process::exit(2);
                }
            };
            Operation::RestorePartial { backup }
        }
        _ => {
            eprintln!("P8H: unsupported or unsafe argument combination");
            std::process::exit(2);
        }
    };
    let result = AppSupervisorClient::from_environment().and_then(|client| {
        let request = prepare(&app, operation)
            .map_err(|_| hars_supervisor_apps::official_transport::TransportError::BadResponse)?;
        // Disposable HAOS fixture actor and grants are NOT a real identity
        // provider. Only this guest example may mint fixture authority.
        // Image-owned private audit path (UID 1000/0700): Supervisor's /data
        // is root-owned and deliberately not chmodded. This fixture journal
        // is NOT durable across App container recreation or a production log.
        let audit_dir = Path::new("/hars-lab-audit");
        let mut journal = FileAuditJournal::open(&audit_dir.join("decisions.jsonl"))
            .map_err(|_| TransportError::Io)?;
        let actor = Actor::new("qemu_manager", Role::Administrator)
            .map_err(|_| TransportError::Unauthorized)?;
        let mut policy = Policy::default();
        policy.grant("qemu_manager", &app, &[request.action()]);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| TransportError::Unauthorized)?
            .as_secs();
        let mut sessions = LabSessionRegistry::default();
        let token = sessions
            .issue_fixture(actor, now, 300, 120)
            .map_err(|_| TransportError::Unauthorized)?;
        // For sensitive operations the fixture creates a separate, short-lived
        // one-use confirmation bound to this exact App/action/backup slug.
        let confirmation = if matches!(
            request.action(),
            hars_supervisor_apps::managed_apps::Action::Install
                | hars_supervisor_apps::managed_apps::Action::Stop
                | hars_supervisor_apps::managed_apps::Action::Update
                | hars_supervisor_apps::managed_apps::Action::Restore
        ) {
            Some(
                sessions
                    .issue_confirmation(&policy, token.as_str(), &request, now)
                    .map_err(|_| TransportError::Unauthorized)?,
            )
        } else {
            None
        };
        let permit = sessions
            .grant_recorded_session(
                &policy,
                token.as_str(),
                &request,
                confirmation.as_ref().map(|c| c.as_str()),
                &mut journal,
                now,
            )
            .map_err(|_| TransportError::Io)?
            .ok_or(TransportError::Unauthorized)?;
        // No boolean bypass. Fixture identity only; still QEMU-only.
        client.preview_execute(permit)
    });
    match result {
        Ok(data) => {
            // This proves the QEMU fixture session + one-use confirmation
            // precede a real Supervisor result; it is NOT user authentication.
            println!("P8I_QEMU_FIXTURE_SESSION_GATE_PASS");
            // Printed only after Rust's *real* Supervisor availability GET
            // AND subsequent INSTALL POST both returned success.
            if action == "install" {
                println!("P8J_REAL_INSTALL_AVAILABILITY_PREFLIGHT_PASS");
            }
            if action == "backup-partial" {
                if let Some(slug) = data.backup_slug() {
                    // Typed receipt from Supervisor: no options or credentials.
                    println!("P8H_BACKUP_SLUG={slug}");
                    return;
                }
                eprintln!("P8H: response had no usable backup slug");
                std::process::exit(2);
            }
            println!(
                "P8H_REAL_SUPERVISOR_{}_PASS",
                action.to_ascii_uppercase().replace('-', "_")
            );
        }
        Err(kind) => {
            // Only a code/variant, never a token, response body or user data.
            eprintln!("P8H_PREVIEW_ERROR={kind:?}");
            std::process::exit(2);
        }
    }
}
