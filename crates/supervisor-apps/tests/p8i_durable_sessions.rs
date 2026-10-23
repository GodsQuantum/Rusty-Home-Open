#![cfg(feature = "persistent-session-lab")]
use hars_supervisor_apps::{
    audit_lab::FileAuditJournal,
    managed_apps::{AppSlug, BackupSlug, Operation, prepare},
    persistent_session_lab::DurableFixtureSessions,
    rbac_lab::{Action, Actor, Policy, Role},
};
use std::{fs, os::unix::fs::PermissionsExt};
#[test]
fn persisted_revocation_and_non_replay_after_process_restart() {
    let dir = tempfile::tempdir().unwrap();
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let path = dir.path().join("sessions.redb");
    let mut policy = Policy::default();
    let app = AppSlug::parse("core_ssh").unwrap();
    policy.grant("fixture_user", &app, &[Action::Restore]);
    let req = prepare(
        &app,
        Operation::RestorePartial {
            backup: BackupSlug::parse("aabbccdd").unwrap(),
        },
    )
    .unwrap();
    let mut journal = FileAuditJournal::open(&dir.path().join("audit.jsonl")).unwrap();
    let (token, ack) = {
        let mut db = DurableFixtureSessions::open(&path).unwrap();
        let token = db
            .issue_fixture(
                Actor::new("fixture_user", Role::Administrator).unwrap(),
                100,
                300,
                100,
            )
            .unwrap();
        let ack = db
            .issue_confirmation(&policy, token.as_str(), &req, 102)
            .unwrap()
            .unwrap();
        (token, ack)
    };
    let mut db = DurableFixtureSessions::open(&path).unwrap();
    assert!(
        db.grant_recorded_session(
            &policy,
            token.as_str(),
            &req,
            Some(ack.as_str()),
            &mut journal,
            103
        )
        .unwrap()
        .is_some()
    );
    drop(db);
    let mut db = DurableFixtureSessions::open(&path).unwrap();
    assert!(
        db.grant_recorded_session(
            &policy,
            token.as_str(),
            &req,
            Some(ack.as_str()),
            &mut journal,
            104
        )
        .unwrap()
        .is_none()
    );
    assert!(db.revoke(token.as_str()).unwrap());
    drop(db);
    let mut db = DurableFixtureSessions::open(&path).unwrap();
    assert!(
        db.grant_recorded_session(&policy, token.as_str(), &req, None, &mut journal, 105)
            .unwrap()
            .is_none()
    );
    let raw = fs::read(&path).unwrap();
    assert!(
        !raw.windows(token.as_str().len())
            .any(|w| w == token.as_str().as_bytes())
    );
    assert_eq!(
        fs::metadata(path).unwrap().permissions().mode() & 0o777,
        0o600
    );
}

#[test]
fn expired_and_household_sessions_are_denied_after_reopen() {
    let d = tempfile::tempdir().unwrap();
    fs::set_permissions(d.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let path = d.path().join("sessions.redb");
    let mut policy = Policy::default();
    let app = AppSlug::parse("core_ssh").unwrap();
    policy.grant("fixture_user", &app, &[Action::Start]);
    let req = prepare(&app, Operation::Start).unwrap();
    let mut journal = FileAuditJournal::open(&d.path().join("audit.jsonl")).unwrap();
    let (old, household) = {
        let mut db = DurableFixtureSessions::open(&path).unwrap();
        let old = db
            .issue_fixture(
                Actor::new("fixture_user", Role::Administrator).unwrap(),
                100,
                300,
                10,
            )
            .unwrap();
        let household = db
            .issue_fixture(
                Actor::new("fixture_user", Role::Household).unwrap(),
                100,
                300,
                60,
            )
            .unwrap();
        (old, household)
    };
    let mut db = DurableFixtureSessions::open(&path).unwrap();
    assert!(
        db.grant_recorded_session(&policy, household.as_str(), &req, None, &mut journal, 101)
            .unwrap()
            .is_none()
    );
    assert!(
        db.grant_recorded_session(&policy, old.as_str(), &req, None, &mut journal, 110)
            .unwrap()
            .is_none()
    );
    assert!(
        db.grant_recorded_session(&policy, old.as_str(), &req, None, &mut journal, 111)
            .unwrap()
            .is_none()
    );
}
#[test]
fn confirmation_wrong_session_or_backup_cannot_be_redeemed_later() {
    let d = tempfile::tempdir().unwrap();
    fs::set_permissions(d.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let path = d.path().join("sessions.redb");
    let mut policy = Policy::default();
    let app = AppSlug::parse("core_ssh").unwrap();
    policy.grant("fixture_user", &app, &[Action::Restore]);
    let req = prepare(
        &app,
        Operation::RestorePartial {
            backup: BackupSlug::parse("aabbccdd").unwrap(),
        },
    )
    .unwrap();
    let wrong = prepare(
        &app,
        Operation::RestorePartial {
            backup: BackupSlug::parse("deadbeef").unwrap(),
        },
    )
    .unwrap();
    let mut journal = FileAuditJournal::open(&d.path().join("audit.jsonl")).unwrap();
    let mut db = DurableFixtureSessions::open(&path).unwrap();
    let a = db
        .issue_fixture(
            Actor::new("fixture_user", Role::Administrator).unwrap(),
            100,
            300,
            200,
        )
        .unwrap();
    let b = db
        .issue_fixture(
            Actor::new("fixture_user", Role::Administrator).unwrap(),
            100,
            300,
            200,
        )
        .unwrap();
    let first = db
        .issue_confirmation(&policy, a.as_str(), &req, 101)
        .unwrap()
        .unwrap();
    assert!(
        db.grant_recorded_session(
            &policy,
            b.as_str(),
            &req,
            Some(first.as_str()),
            &mut journal,
            102
        )
        .unwrap()
        .is_none()
    );
    assert!(
        db.grant_recorded_session(
            &policy,
            a.as_str(),
            &req,
            Some(first.as_str()),
            &mut journal,
            103
        )
        .unwrap()
        .is_none()
    );
    let second = db
        .issue_confirmation(&policy, a.as_str(), &req, 104)
        .unwrap()
        .unwrap();
    drop(db);
    let mut db = DurableFixtureSessions::open(&path).unwrap();
    assert!(
        db.grant_recorded_session(
            &policy,
            a.as_str(),
            &wrong,
            Some(second.as_str()),
            &mut journal,
            105
        )
        .unwrap()
        .is_none()
    );
    assert!(
        db.grant_recorded_session(
            &policy,
            a.as_str(),
            &req,
            Some(second.as_str()),
            &mut journal,
            106
        )
        .unwrap()
        .is_none()
    );
}
#[test]
fn rejects_unprotected_and_symlink_database_paths() {
    let d = tempfile::tempdir().unwrap();
    let file = d.path().join("durable.redb");
    // tempdir defaults 0700 on Unix. Temporarily make it unsafe.
    fs::set_permissions(d.path(), fs::Permissions::from_mode(0o755)).unwrap();
    assert!(DurableFixtureSessions::open(&file).is_err());
    fs::set_permissions(d.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let link = d.path().join("linked.redb");
    std::os::unix::fs::symlink(&file, &link).unwrap();
    assert!(DurableFixtureSessions::open(&link).is_err());
}

fn count_entries(path: &std::path::Path, name: &'static str) -> u64 {
    use redb::{ReadableDatabase as _, ReadableTableMetadata as _};
    let database = redb::Database::open(path).unwrap();
    let tx = database.begin_read().unwrap();
    let table = tx
        .open_table(redb::TableDefinition::<&str, &str>::new(name))
        .unwrap();
    table.len().unwrap()
}

#[test]
fn durable_approvals_are_bounded_pruned_and_atomically_revoked() {
    let dir = tempfile::tempdir().unwrap();
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let path = dir.path().join("sessions.redb");
    let app = AppSlug::parse("future_vendor_7241").unwrap();
    let request = prepare(&app, Operation::Install).unwrap();
    let mut policy = Policy::default();
    policy.grant("session_operator", &app, &[Action::Install]);
    let mut db = DurableFixtureSessions::open(&path).unwrap();
    let session = db
        .issue_fixture(
            Actor::new("session_operator", Role::Administrator).unwrap(),
            100,
            600,
            600,
        )
        .unwrap();

    // A single valid session cannot persist more than 32 outstanding requests.
    let mut old_approval = None;
    for i in 0..32 {
        let token = db
            .issue_confirmation(&policy, session.as_str(), &request, 101)
            .unwrap()
            .unwrap();
        if i == 0 {
            old_approval = Some(token);
        }
    }
    let denied = db.issue_confirmation(&policy, session.as_str(), &request, 101);
    assert!(
        denied.is_err(),
        "durable per-session approval limit not enforced"
    );
    drop(db);
    assert_eq!(count_entries(&path, "lab_approvals_v1"), 32);

    let mut db = DurableFixtureSessions::open(&path).unwrap();
    // At the exact 60-second boundary every old confirmation is expired.
    let new_approval = db
        .issue_confirmation(&policy, session.as_str(), &request, 161)
        .unwrap()
        .unwrap();
    drop(db);
    assert_eq!(count_entries(&path, "lab_approvals_v1"), 1);

    let mut db = DurableFixtureSessions::open(&path).unwrap();
    let mut journal = FileAuditJournal::open(&dir.path().join("audit.jsonl")).unwrap();
    assert!(db.revoke(session.as_str()).unwrap());
    drop(db);
    assert_eq!(count_entries(&path, "lab_sessions_v1"), 0);
    assert_eq!(count_entries(&path, "lab_approvals_v1"), 0);

    let mut db = DurableFixtureSessions::open(&path).unwrap();
    for approval in [old_approval.unwrap(), new_approval] {
        assert!(
            db.grant_recorded_session(
                &policy,
                session.as_str(),
                &request,
                Some(approval.as_str()),
                &mut journal,
                162
            )
            .unwrap()
            .is_none()
        );
    }
}

#[test]
fn durable_sessions_reject_capacity_then_reuse_expired_slots() {
    let dir = tempfile::tempdir().unwrap();
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let path = dir.path().join("sessions.redb");
    let user = Actor::new("fixture_limit", Role::Administrator).unwrap();
    let mut db = DurableFixtureSessions::open(&path).unwrap();
    for _ in 0..1024 {
        db.issue_fixture(user.clone(), 100, 200, 200).unwrap();
    }
    assert!(db.issue_fixture(user.clone(), 100, 200, 200).is_err());
    drop(db);
    assert_eq!(count_entries(&path, "lab_sessions_v1"), 1024);
    let mut db = DurableFixtureSessions::open(&path).unwrap();
    // The 1024 sessions have now expired, including across restart.
    assert!(db.issue_fixture(user, 300, 200, 200).is_ok());
    drop(db);
    assert_eq!(count_entries(&path, "lab_sessions_v1"), 1);
}
