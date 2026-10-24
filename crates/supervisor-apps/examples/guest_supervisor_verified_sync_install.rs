//! Disposable HAOS QEMU proof: use official synchronous Store Install,
//! which does NOT emit a job ID that may disappear when cleanup=True.
//! Real protected manager-role App and authenticated independent inventory.
use hars_supervisor_apps::{
    audit_lab::{FileAuditJournal, grant_recorded},
    durable_dispatch_fence_lab::{CompletionEvidence, DispatchState, DurableDispatchFence},
    managed_apps::{Action, AppSlug, Operation, prepare},
    official_transport::AppSupervisorClient,
    rbac_lab::{Acknowledgement, Actor, Policy, Role},
};
use std::{
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

fn now_ms() -> Result<u64, ()> {
    u64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| ())?
            .as_millis(),
    )
    .map_err(|_| ())
}
fn run() -> Result<(), ()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 2
        || args[0] != "--acknowledge-disposable-qemu"
        || std::env::var("HARS_P8H_QEMU_ONLY").ok().as_deref() != Some("1")
    {
        return Err(());
    }
    // Test App fixture restriction is NOT a library vendor allowlist.
    if !args[1].starts_with("local_hars_") {
        return Err(());
    }
    let app = AppSlug::parse(&args[1]).map_err(|_| ())?;
    let request = prepare(&app, Operation::Install).map_err(|_| ())?;
    if request.background_job() {
        return Err(());
    }
    let root = Path::new("/hars-lab-audit");
    let mut audit = FileAuditJournal::open(&root.join("sync-install.jsonl")).map_err(|_| ())?;
    let actor = Actor::new("qemu_sync_fixture", Role::Administrator).map_err(|_| ())?;
    let mut policy = Policy::default();
    policy.grant(actor.subject(), &app, &[Action::Install]);
    let consent = Acknowledgement::bind(&actor, &request);
    let permit = grant_recorded(&policy, Some(&actor), &request, Some(&consent), &mut audit)
        .map_err(|_| ())?
        .ok_or(())?;
    let mut fence =
        DurableDispatchFence::open(&root.join("sync-install-intents.redb")).map_err(|_| ())?;
    let client = AppSupervisorClient::from_environment().map_err(|_| ())?;
    let armed = fence.arm(permit, now_ms()?).map_err(|_| ())?;
    let id = armed.id().to_owned();
    println!("P8O_STAGE_SYNC_PREPOST_FENCE_PASS");
    let completed = client
        .preview_execute_fenced_sync_install(&mut fence, armed)
        .map_err(|error| match error {
            hars_supervisor_apps::durable_dispatch_fence_lab::TrackedError::Transport(code) => {
                eprintln!("P8O_FAIL_SYNC_TRANSPORT_{code:?}")
            }
            hars_supervisor_apps::durable_dispatch_fence_lab::TrackedError::Storage(_) => {
                eprintln!("P8O_FAIL_SYNC_POSTFLIGHT_OR_STORAGE")
            }
        })?;
    if completed.state != DispatchState::Confirmed
        || completed.job_id.is_some()
        || completed.app != app.as_str()
        || completed.completion_evidence
            != Some(CompletionEvidence::SynchronousTransportAndInventory)
    {
        return Err(());
    }
    drop(fence);
    let mut reopened =
        DurableDispatchFence::open(&root.join("sync-install-intents.redb")).map_err(|_| ())?;
    if reopened.get(&id).map_err(|_| ())? != Some(completed) {
        return Err(());
    }
    let another = grant_recorded(&policy, Some(&actor), &request, Some(&consent), &mut audit)
        .map_err(|_| ())?
        .ok_or(())?;
    if reopened.arm(another, now_ms()?).is_err() {
        return Err(());
    }
    println!("P8O_REAL_SUPERVISOR_SYNC_INSTALL_AND_DURABLE_RELEASE_PASS");
    Ok(())
}
fn main() {
    if run().is_err() {
        eprintln!("P8O_SYNC_INSTALL_UNVERIFIED");
        std::process::exit(2);
    }
}
