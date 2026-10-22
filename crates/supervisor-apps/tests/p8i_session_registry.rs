#![cfg(feature = "managed-apps-preview")]
use hars_supervisor_apps::{
    audit_lab::FileAuditJournal,
    managed_apps::{AppSlug, BackupSlug, Operation, prepare},
    rbac_lab::{Action, Actor, Policy, Role},
    session_lab::LabSessionRegistry,
};
use std::{fs, os::unix::fs::PermissionsExt};

fn journal() -> (tempfile::TempDir, FileAuditJournal) {
    let dir = tempfile::tempdir().unwrap();
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let log = FileAuditJournal::open(&dir.path().join("audit.jsonl")).unwrap();
    (dir, log)
}
fn setup() -> (LabSessionRegistry, Policy, AppSlug) {
    let app = AppSlug::parse("core_ssh").unwrap();
    let mut policy = Policy::default();
    policy.grant("lab_owner", &app, &[Action::Start, Action::Restore]);
    (LabSessionRegistry::default(), policy, app)
}
#[test]
fn session_csprng_hashed_storage_absolute_idle_timeout_and_revoke() {
    let (mut store, policy, app) = setup();
    let actor = Actor::new("lab_owner", Role::Administrator).unwrap();
    let token = store.issue_fixture(actor.clone(), 100, 120, 10).unwrap();
    let token2 = store.issue_fixture(actor, 100, 120, 10).unwrap();
    assert_ne!(token.as_str(), token2.as_str());
    assert_eq!(token.as_str().len(), 64);
    assert!(!format!("{store:?}").contains(token.as_str()));
    assert!(!format!("{token:?}").contains(token.as_str()));
    let (_dir, mut log) = journal();
    let req = prepare(&app, Operation::Start).unwrap();
    assert!(
        store
            .grant_recorded_session(&policy, token.as_str(), &req, None, &mut log, 101)
            .unwrap()
            .is_some()
    );
    assert!(
        store
            .grant_recorded_session(&policy, token.as_str(), &req, None, &mut log, 110)
            .unwrap()
            .is_some()
    );
    assert!(
        store
            .grant_recorded_session(&policy, token.as_str(), &req, None, &mut log, 121)
            .unwrap()
            .is_none()
    );
    assert!(
        store
            .grant_recorded_session(&policy, token2.as_str(), &req, None, &mut log, 220)
            .unwrap()
            .is_none()
    );
    let token3 = store
        .issue_fixture(
            Actor::new("lab_owner", Role::Administrator).unwrap(),
            300,
            120,
            10,
        )
        .unwrap();
    assert!(store.revoke(token3.as_str()));
    assert!(!store.revoke(token3.as_str()));
    assert!(
        store
            .grant_recorded_session(&policy, token3.as_str(), &req, None, &mut log, 301)
            .unwrap()
            .is_none()
    );
    assert!(
        store
            .grant_recorded_session(&policy, "not-a-real-token", &req, None, &mut log, 301)
            .unwrap()
            .is_none()
    );
}
#[test]
fn one_time_confirmation_is_request_bound_and_cannot_replay_or_cross_session() {
    let (mut store, policy, app) = setup();
    let actor = Actor::new("lab_owner", Role::Administrator).unwrap();
    let first = store.issue_fixture(actor.clone(), 100, 300, 180).unwrap();
    let second = store.issue_fixture(actor, 100, 300, 180).unwrap();
    let req = prepare(
        &app,
        Operation::RestorePartial {
            backup: BackupSlug::parse("deadbeef").unwrap(),
        },
    )
    .unwrap();
    let different = prepare(
        &app,
        Operation::RestorePartial {
            backup: BackupSlug::parse("faceface").unwrap(),
        },
    )
    .unwrap();
    let (_dir, mut log) = journal();
    assert!(
        store
            .grant_recorded_session(&policy, first.as_str(), &req, None, &mut log, 101)
            .unwrap()
            .is_none()
    );
    let approval = store
        .issue_confirmation(&policy, first.as_str(), &req, 102)
        .unwrap();
    assert!(
        store
            .grant_recorded_session(
                &policy,
                second.as_str(),
                &req,
                Some(approval.as_str()),
                &mut log,
                103
            )
            .unwrap()
            .is_none()
    );
    assert!(
        store
            .grant_recorded_session(
                &policy,
                first.as_str(),
                &different,
                Some(approval.as_str()),
                &mut log,
                104
            )
            .unwrap()
            .is_none()
    );
    assert!(
        store
            .grant_recorded_session(
                &policy,
                first.as_str(),
                &req,
                Some(approval.as_str()),
                &mut log,
                105
            )
            .unwrap()
            .is_none()
    );
    let approval2 = store
        .issue_confirmation(&policy, first.as_str(), &req, 106)
        .unwrap();
    assert!(
        store
            .grant_recorded_session(
                &policy,
                first.as_str(),
                &req,
                Some(approval2.as_str()),
                &mut log,
                107
            )
            .unwrap()
            .is_some()
    );
    assert!(
        store
            .grant_recorded_session(
                &policy,
                first.as_str(),
                &req,
                Some(approval2.as_str()),
                &mut log,
                108
            )
            .unwrap()
            .is_none()
    );
    let approval3 = store
        .issue_confirmation(&policy, first.as_str(), &req, 109)
        .unwrap();
    assert!(
        store
            .grant_recorded_session(
                &policy,
                first.as_str(),
                &req,
                Some(approval3.as_str()),
                &mut log,
                170
            )
            .unwrap()
            .is_none()
    );
    assert!(
        store
            .grant_recorded_session(
                &policy,
                first.as_str(),
                &req,
                Some(approval3.as_str()),
                &mut log,
                410
            )
            .unwrap()
            .is_none()
    );
}
#[test]
fn no_role_from_client_and_file_audit_no_session_secrets() {
    let (mut store, policy, app) = setup();
    let actor = Actor::new("lab_owner", Role::Household).unwrap();
    let token = store.issue_fixture(actor, 100, 100, 60).unwrap();
    let (dir, mut log) = journal();
    let req = prepare(&app, Operation::Start).unwrap();
    assert!(
        store
            .grant_recorded_session(&policy, token.as_str(), &req, None, &mut log, 101)
            .unwrap()
            .is_none()
    );

    // A successful, scoped laboratory principal is still not a production IdP.
    let good = store
        .issue_fixture(
            Actor::new("lab_owner", Role::Administrator).unwrap(),
            102,
            100,
            60,
        )
        .unwrap();
    assert!(
        store
            .grant_recorded_session(&policy, good.as_str(), &req, None, &mut log, 103)
            .unwrap()
            .is_some()
    );
    drop(log);
    let bytes = fs::read_to_string(dir.path().join("audit.jsonl")).unwrap();
    assert!(bytes.contains("role_denied"));
    assert!(bytes.contains("allowed"));
    assert!(!bytes.contains(token.as_str()));
    assert!(!bytes.contains(good.as_str()));
    assert!(!format!("{store:?}").contains(good.as_str()));
}
