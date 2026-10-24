//! Executable proof of actual Supervisor async App install -> official job
//! GET -> independent installed inventory GET -> durable fence release.
//! ONLY within a disposable HAOS QEMU guest; the human Core identity/consent
//! production gateway is intentionally out of scope for this fixture.
use hars_supervisor_apps::{
    audit_lab::{FileAuditJournal, grant_recorded},
    durable_dispatch_fence_lab::{DispatchState, DurableDispatchFence},
    durable_job_watch::DurableJobWatches,
    managed_apps::{Action, AppSlug, Operation, prepare_background_job},
    official_store::ApiVersion,
    official_transport::AppSupervisorClient,
    rbac_lab::{Acknowledgement, Actor, Policy, Role},
};
use std::{
    path::Path,
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

fn now_ms() -> Result<u64, ()> {
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| ())?;
    u64::try_from(t.as_millis()).map_err(|_| ())
}
fn run() -> Result<(), ()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 2
        || args[0] != "--acknowledge-disposable-qemu"
        || std::env::var("HARS_P8H_QEMU_ONLY").ok().as_deref() != Some("1")
    {
        return Err(());
    }
    // Restriction belongs ONLY to the one-off guest test executable, not to
    // the generic Supervisor Store library or supported App vendors.
    if !args[1].starts_with("local_hars_") {
        return Err(());
    }
    let app = AppSlug::parse(&args[1]).map_err(|_| ())?;
    let requested = prepare_background_job(&app, Operation::Install).map_err(|_| ())?;
    let mut journal =
        FileAuditJournal::open(&Path::new("/hars-lab-audit").join("verified-async-install.jsonl"))
            .map_err(|_| {
                eprintln!("P8N_FAIL_AUDIT_OPEN");
            })?;
    let actor = Actor::new("qemu_verified_fixture", Role::Administrator).map_err(|_| ())?;
    let mut policy = Policy::default();
    policy.grant(actor.subject(), &app, &[Action::Install]);
    let consent = Acknowledgement::bind(&actor, &requested);
    let permit = grant_recorded(
        &policy,
        Some(&actor),
        &requested,
        Some(&consent),
        &mut journal,
    )
    .map_err(|_| {
        eprintln!("P8N_FAIL_AUDIT_WRITE");
    })?
    .ok_or_else(|| {
        eprintln!("P8N_FAIL_POLICY_DENIED");
    })?;
    println!("P8N_STAGE_AUTH_AUDIT_PASS");
    let path = Path::new("/hars-lab-audit");
    let mut fence =
        DurableDispatchFence::open(&path.join("verified-async-intents.redb")).map_err(|_| {
            eprintln!("P8N_FAIL_FENCE_OPEN");
        })?;
    let mut watches =
        DurableJobWatches::open(&path.join("verified-async-watches.redb")).map_err(|_| {
            eprintln!("P8N_FAIL_WATCH_OPEN");
        })?;
    let client = AppSupervisorClient::from_environment().map_err(|_| {
        eprintln!("P8N_FAIL_TOKEN");
    })?;
    let armed = fence.arm(permit, now_ms()?).map_err(|_| {
        eprintln!("P8N_FAIL_FENCE_ARM");
    })?;
    println!("P8N_STAGE_DURABLE_ARM_PASS");
    let id = armed.id().to_owned();
    // Exactly ONE audited background POST. Never repeat on ambiguous response.
    let posted = client
        .preview_execute_fenced(&mut fence, armed)
        .map_err(|error| match error {
            hars_supervisor_apps::durable_dispatch_fence_lab::TrackedError::Transport(e) => {
                eprintln!("P8N_FAIL_POST_TRANSPORT_{e:?}")
            }
            hars_supervisor_apps::durable_dispatch_fence_lab::TrackedError::Storage(_) => {
                eprintln!("P8N_FAIL_POST_PERSISTENCE")
            }
        })?;
    println!("P8N_STAGE_POST_RECEIPT_PASS");
    if posted.receipt.job_id().is_none() || posted.operation_id != id {
        eprintln!("P8N_FAIL_UNCORRELATED_JOB");
        return Err(());
    }
    println!("P8N_STAGE_JOB_UUID_PASS");
    let mut reported_pending = false;
    let mut reported_failure = false;
    for _ in 0..50 {
        let result =
            fence.verify_and_close_install(&id, &mut watches, &client, ApiVersion::V1, now_ms()?);
        match result {
            Ok(Some(closed)) => {
                if closed.state != DispatchState::Confirmed || closed.app != app.as_str() {
                    return Err(());
                }
                drop(watches);
                drop(fence);
                // The real postcondition and ACID transition MUST survive
                // reopening both separate private database files.
                let reopened =
                    DurableDispatchFence::open(&path.join("verified-async-intents.redb"))
                        .map_err(|_| ())?;
                let persisted = reopened.get(&id).map_err(|_| ())?.ok_or(())?;
                if persisted != closed {
                    return Err(());
                }
                println!("P8N_REAL_SUPERVISOR_ASYNC_INSTALL_JOB_APP_AND_REDB_PASS");
                return Ok(());
            }
            Ok(None) => {
                if !reported_pending {
                    println!("P8N_POLL_WAITING_FOR_VERIFIED_POSTCONDITION");
                    reported_pending = true;
                }
                thread::sleep(Duration::from_secs(2));
            }
            Err(error) => {
                if !reported_failure {
                    match error {
                        hars_supervisor_apps::durable_dispatch_fence_lab::VerifiedCloseError::Poll(_) =>
                            eprintln!("P8N_POLL_REMOTE_READ_FAILED"),
                        hars_supervisor_apps::durable_dispatch_fence_lab::VerifiedCloseError::Storage(_) =>
                            eprintln!("P8N_POLL_DURABLE_STORE_FAILED"),
                    }
                    reported_failure = true;
                }
                // A read outage cannot justify resubmitting the mutation.
                thread::sleep(Duration::from_secs(2));
            }
        }
    }
    Err(())
}

fn main() {
    if run().is_err() {
        // Never print raw Supervisor JSON, token, actor IDs or paths.
        eprintln!("P8N_ASYNC_INSTALL_VERIFICATION_INCOMPLETE");
        std::process::exit(2);
    }
}
