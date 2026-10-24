#![cfg(feature = "durable-job-watch")]
use hars_supervisor_apps::{
    durable_job_watch::DurableJobWatches,
    installed_inventory::{RunningApp, RunningInventory},
    job_status::{JobStatus, State},
    official_store::ApiVersion,
    operation_reconcile::{Operation, Outcome},
};
use std::{fs, os::unix::fs::PermissionsExt};

const JOB: &str = "0123456789abcdef0123456789abcdef";
const OTHER: &str = "ffffffffffffffffffffffffffffffff";

fn job(id: &str, state: State) -> JobStatus {
    JobStatus {
        id: id.into(),
        progress: 100.0,
        state,
        error_count: if state == State::Failed { 1 } else { 0 },
        stage: None,
        read_only: true,
    }
}
fn installed(slug: &str, state: &str) -> RunningInventory {
    RunningInventory {
        api_version: ApiVersion::V2,
        read_only: true,
        apps: vec![RunningApp {
            slug: slug.into(),
            name: "Any community App".into(),
            repository: Some("arbitrary_vendor".into()),
            state: Some(state.into()),
            version: Some("1.0".into()),
            update_available: Some(false),
        }],
    }
}
fn private_dir() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
    dir
}

#[test]
fn persisted_generic_watch_survives_restart_and_only_advances_with_evidence() {
    let dir = private_dir();
    let path = dir.path().join("watches.redb");
    {
        let mut ledger = DurableJobWatches::open(&path).unwrap();
        ledger
            .register(
                JOB,
                "future_vendor_42",
                Operation::Start,
                None,
                "core_user_7",
                100,
            )
            .unwrap();
        assert_eq!(
            ledger
                .observe(
                    JOB,
                    Some(&job(OTHER, State::Succeeded)),
                    Some(&installed("future_vendor_42", "started")),
                    101
                )
                .unwrap()
                .unwrap()
                .outcome,
            Some(Outcome::NoEvidence)
        );
        assert_eq!(
            ledger
                .observe(JOB, Some(&job(JOB, State::Running)), None, 102)
                .unwrap()
                .unwrap()
                .outcome,
            Some(Outcome::Running)
        );
    }
    {
        let mut ledger = DurableJobWatches::open(&path).unwrap();
        let old = ledger
            .register(
                JOB,
                "future_vendor_42",
                Operation::Start,
                None,
                "core_user_7",
                100,
            )
            .unwrap();
        assert_eq!(old.outcome, Some(Outcome::Running));
        assert_eq!(
            ledger
                .observe(JOB, Some(&job(JOB, State::Succeeded)), None, 103)
                .unwrap()
                .unwrap()
                .outcome,
            Some(Outcome::AwaitingState)
        );
        let completed = ledger
            .observe(
                JOB,
                Some(&job(JOB, State::Succeeded)),
                Some(&installed("future_vendor_42", "started")),
                104,
            )
            .unwrap()
            .unwrap();
        assert_eq!(completed.outcome, Some(Outcome::Confirmed));
        assert!(completed.postcondition_observed);
    }
    {
        let mut ledger = DurableJobWatches::open(&path).unwrap();
        assert_eq!(
            ledger.get(JOB).unwrap().unwrap().outcome,
            Some(Outcome::Confirmed)
        );
        // Never erase a terminal observation after a transient API outage.
        assert_eq!(
            ledger
                .observe(JOB, None, None, 105)
                .unwrap()
                .unwrap()
                .outcome,
            Some(Outcome::Confirmed)
        );
        assert!(
            ledger
                .register(
                    JOB,
                    "other_vendor",
                    Operation::Stop,
                    None,
                    "core_user_7",
                    106
                )
                .is_err()
        );
        assert!(
            ledger
                .register(
                    JOB,
                    "future_vendor_42",
                    Operation::Start,
                    None,
                    "other_actor",
                    106
                )
                .is_err()
        );
        assert!(ledger.get(OTHER).unwrap().is_none());
    }
    assert_eq!(
        fs::metadata(path).unwrap().permissions().mode() & 0o777,
        0o600
    );
}

#[test]
fn rejects_untrusted_paths_actors_updates_and_false_success() {
    let dir = private_dir();
    let path = dir.path().join("watches.redb");
    let mut ledger = DurableJobWatches::open(&path).unwrap();
    assert!(
        ledger
            .register(JOB, "../bad", Operation::Start, None, "core_user_7", 100)
            .is_err()
    );
    assert!(
        ledger
            .register(JOB, "good", Operation::Start, None, "bad\nactor", 100)
            .is_err()
    );
    assert!(
        ledger
            .register(JOB, "good", Operation::Update, None, "core_user_7", 100)
            .is_err()
    );
    assert!(
        ledger
            .register(
                "bad-job",
                "good",
                Operation::Start,
                None,
                "core_user_7",
                100
            )
            .is_err()
    );
    ledger
        .register(JOB, "good", Operation::Start, None, "core_user_7", 100)
        .unwrap();
    let not_started = installed("good", "stopped");
    assert_eq!(
        ledger
            .observe(
                JOB,
                Some(&job(JOB, State::Succeeded)),
                Some(&not_started),
                101
            )
            .unwrap()
            .unwrap()
            .outcome,
        Some(Outcome::AwaitingState)
    );
    let mut untrusted = installed("good", "started");
    untrusted.read_only = false;
    assert_eq!(
        ledger
            .observe(
                JOB,
                Some(&job(JOB, State::Succeeded)),
                Some(&untrusted),
                102
            )
            .unwrap()
            .unwrap()
            .outcome,
        Some(Outcome::AwaitingState)
    );
    assert!(ledger.observe(JOB, None, None, 99).is_err());
}

#[test]
fn refuses_world_readable_directory_and_symlink_database() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("watches.redb");
    assert!(DurableJobWatches::open(&path).is_err());
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let target = dir.path().join("target");
    fs::write(&target, b"fake").unwrap();
    std::os::unix::fs::symlink(&target, &path).unwrap();
    assert!(DurableJobWatches::open(&path).is_err());
}
