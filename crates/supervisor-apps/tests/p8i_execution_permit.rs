#![cfg(feature = "managed-apps-preview")]
use hars_supervisor_apps::{
    audit_lab::{FileAuditJournal, grant_recorded},
    managed_apps::{AppSlug, Operation, prepare},
    official_transport::{AppSupervisorClient, SupervisorToken, TransportError},
    rbac_lab::{Acknowledgement, Action, Actor, Policy, Role},
};
use std::{fs, os::unix::fs::PermissionsExt};

fn setup() -> (tempfile::TempDir, FileAuditJournal) {
    let dir = tempfile::tempdir().unwrap();
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let journal = FileAuditJournal::open(&dir.path().join("audit.jsonl")).unwrap();
    (dir, journal)
}

#[test]
fn permit_is_minted_only_after_audited_scoped_authorization_and_consumed_by_transport() {
    let (_dir, mut journal) = setup();
    let actor = Actor::new("lab_operator", Role::Administrator).unwrap();
    let app = AppSlug::parse("core_ssh").unwrap();
    let request = prepare(&app, Operation::Start).unwrap();
    let mut policy = Policy::default();
    policy.grant("lab_operator", &app, &[Action::Start]);
    assert!(
        grant_recorded(&policy, None, &request, None, &mut journal)
            .unwrap()
            .is_none()
    );
    let permit = grant_recorded(&policy, Some(&actor), &request, None, &mut journal)
        .unwrap()
        .expect("authorized and durably audited permit");
    let client =
        AppSupervisorClient::new(SupervisorToken::new("synthetic-not-real".to_string()).unwrap());
    // Even a valid permit cannot execute outside an explicitly disposable HAOS.
    assert!(matches!(
        client.preview_execute(permit),
        Err(TransportError::ManagedPreviewDisabled)
    ));
}

#[test]
fn sensitive_operations_require_matching_request_bound_confirmation() {
    let (_dir, mut journal) = setup();
    let actor = Actor::new("lab_operator", Role::Administrator).unwrap();
    let app = AppSlug::parse("core_ssh").unwrap();
    let request = prepare(
        &app,
        Operation::RestorePartial {
            backup: hars_supervisor_apps::managed_apps::BackupSlug::parse("deadbeef").unwrap(),
        },
    )
    .unwrap();
    let mut policy = Policy::default();
    policy.grant("lab_operator", &app, &[Action::Restore]);
    assert!(
        grant_recorded(&policy, Some(&actor), &request, None, &mut journal)
            .unwrap()
            .is_none()
    );
    let acknowledged = Acknowledgement::bind(&actor, &request);
    assert!(
        grant_recorded(
            &policy,
            Some(&actor),
            &request,
            Some(&acknowledged),
            &mut journal
        )
        .unwrap()
        .is_some()
    );
}
