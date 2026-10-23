use hars_supervisor_apps::{
    installed_inventory::{RunningApp, RunningInventory},
    job_status::{JobStatus, State},
    official_store::ApiVersion,
    operation_reconcile::{Operation, Outcome, Watch, WatchError, reconcile},
};
const JOB: &str = "0123456789abcdef0123456789abcdef";
fn app(name: &str, state: &str, version: &str, updates: bool) -> RunningApp {
    RunningApp {
        slug: name.into(),
        name: format!("Unlisted {name}"),
        repository: Some("any_new_vendor".into()),
        state: Some(state.into()),
        version: Some(version.into()),
        update_available: Some(updates),
    }
}
fn inventory(apps: Vec<RunningApp>) -> RunningInventory {
    RunningInventory {
        api_version: ApiVersion::V2,
        apps,
        read_only: true,
    }
}
fn job(state: State, id: &str, progress: f64) -> JobStatus {
    JobStatus {
        id: id.into(),
        state,
        progress,
        error_count: if state == State::Failed { 1 } else { 0 },
        stage: None,
        read_only: true,
    }
}
#[test]
fn all_generic_actions_require_real_job_and_observed_postconditions() {
    let desired = [
        (
            Operation::Install,
            app("unlisted_future", "stopped", "1", false),
            None,
        ),
        (
            Operation::Start,
            app("unlisted_future", "started", "1", false),
            None,
        ),
        (
            Operation::Stop,
            app("unlisted_future", "stopped", "1", false),
            None,
        ),
        (
            Operation::Restart,
            app("unlisted_future", "started", "1", false),
            None,
        ),
        (
            Operation::Update,
            app("unlisted_future", "started", "2", false),
            Some("1"),
        ),
    ];
    for (action, ready, prior) in desired {
        let w = Watch::new("unlisted_future", JOB, action, prior).unwrap();
        let done = job(State::Succeeded, JOB, 100.0);
        let snapshot = inventory(vec![ready]);
        assert_eq!(
            reconcile(&w, Some(&done), Some(&snapshot)).outcome,
            Outcome::Confirmed
        );
        assert!(reconcile(&w, Some(&done), Some(&snapshot)).completion_verified);
        assert_eq!(
            reconcile(&w, Some(&done), None).outcome,
            Outcome::AwaitingState
        );
        assert_eq!(
            reconcile(&w, None, Some(&snapshot)).outcome,
            Outcome::NoEvidence
        );
        assert_eq!(
            reconcile(&w, Some(&job(State::Running, JOB, 100.0)), Some(&snapshot)).outcome,
            Outcome::Running
        );
        assert_eq!(
            reconcile(&w, Some(&job(State::Failed, JOB, 100.0)), Some(&snapshot)).outcome,
            Outcome::Failed
        );
        assert_eq!(
            reconcile(
                &w,
                Some(&job(
                    State::Succeeded,
                    "ffffffffffffffffffffffffffffffff",
                    100.0
                )),
                Some(&snapshot)
            )
            .outcome,
            Outcome::NoEvidence
        );
    }
}
#[test]
fn install_stale_update_stopped_and_missing_apps_are_not_false_success() {
    let cases = [
        (Operation::Install, None, None),
        (
            Operation::Start,
            Some(app("any", "stopped", "1", false)),
            None,
        ),
        (
            Operation::Stop,
            Some(app("any", "started", "1", false)),
            None,
        ),
        (
            Operation::Restart,
            Some(app("any", "stopped", "1", false)),
            None,
        ),
        (
            Operation::Update,
            Some(app("any", "started", "1", false)),
            Some("1"),
        ),
        (
            Operation::Update,
            Some(app("any", "started", "2", true)),
            Some("1"),
        ),
    ];
    for (action, live, prior) in cases {
        let w = Watch::new("any", JOB, action, prior).unwrap();
        let snapshot = inventory(live.into_iter().collect());
        let result = reconcile(
            &w,
            Some(&job(State::Succeeded, JOB, 100.0)),
            Some(&snapshot),
        );
        assert_eq!(result.outcome, Outcome::AwaitingState);
        assert!(!result.completion_verified);
    }
}
#[test]
fn arbitrary_vendor_fleet_is_not_a_supported_app_whitelist() {
    let names = (0..903)
        .map(|i| format!("future_vendor_{i:04}"))
        .collect::<Vec<_>>();
    let snapshot = inventory(
        names
            .iter()
            .map(|slug| app(slug, "started", "12", false))
            .collect(),
    );
    for name in &names {
        let w = Watch::new(name, JOB, Operation::Start, None).unwrap();
        assert_eq!(
            reconcile(
                &w,
                Some(&job(State::Succeeded, JOB, 100.0)),
                Some(&snapshot)
            )
            .outcome,
            Outcome::Confirmed
        );
    }
}
#[test]
fn invalid_requests_never_claim_completion() {
    assert_eq!(
        Watch::new("../target", JOB, Operation::Start, None),
        Err(WatchError::InvalidApp)
    );
    assert_eq!(
        Watch::new("valid", "dead", Operation::Start, None),
        Err(WatchError::InvalidJob)
    );
    assert_eq!(
        Watch::new("valid", JOB, Operation::Update, None),
        Err(WatchError::MissingPriorVersion)
    );
    assert_eq!(
        Watch::new("valid", JOB, Operation::Update, Some("")),
        Err(WatchError::MissingPriorVersion)
    );
}

#[test]
fn untrusted_job_or_inventory_never_confirms_completion() {
    let w = Watch::new("future_vendor", JOB, Operation::Start, None).unwrap();
    let clean = job(State::Succeeded, JOB, 100.0);
    let ready = inventory(vec![app("future_vendor", "started", "1", false)]);
    let mut untrusted = clean.clone();
    untrusted.read_only = false;
    assert_eq!(
        reconcile(&w, Some(&untrusted), Some(&ready)).outcome,
        Outcome::NoEvidence
    );
    let mut malformed = clean.clone();
    malformed.progress = f64::NAN;
    assert_eq!(
        reconcile(&w, Some(&malformed), Some(&ready)).outcome,
        Outcome::NoEvidence
    );
    let mut unsafe_inventory = ready.clone();
    unsafe_inventory.read_only = false;
    assert_eq!(
        reconcile(&w, Some(&clean), Some(&unsafe_inventory)).outcome,
        Outcome::AwaitingState
    );
    let mut ambiguous = ready.clone();
    ambiguous
        .apps
        .push(app("FUTURE_VENDOR", "started", "2", false));
    assert_eq!(
        reconcile(&w, Some(&clean), Some(&ambiguous)).outcome,
        Outcome::AwaitingState
    );
}
