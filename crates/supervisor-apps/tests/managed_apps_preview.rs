#![cfg(feature = "managed-apps-preview")]
use hars_supervisor_apps::managed_apps::{AppSlug, BackupSlug, ManagerError, Operation, prepare};
use hars_supervisor_apps::official_transport::{AppSupervisorClient, SupervisorToken};

#[test]
fn rejects_path_injection_self_management_and_unsafe_slugs() {
    for slug in [
        "",
        "_bad",
        "Bad",
        "bad/path",
        "../core_ssh",
        "core_ssh?admin=true",
        "local_hars_rust_probe",
        "other%2Fapps",
        "bad\nInjected: yes",
        "é",
    ] {
        assert!(
            AppSlug::parse(slug).is_err(),
            "accepted unsafe slug {slug:?}"
        );
    }
    assert!(AppSlug::parse(&"a".repeat(82)).is_err());
    assert!(AppSlug::parse("core_ssh").is_ok());
    assert!(AppSlug::parse("a0d7b954_vscode").is_ok());
}
#[test]
fn routes_install_start_stop_restart_and_backup_first_update_to_official_supervisor() {
    let app = AppSlug::parse("a0d7b954_vscode").unwrap();
    for (action, path, body) in [
        (
            Operation::Install,
            "/store/addons/a0d7b954_vscode/install",
            "{}",
        ),
        (Operation::Start, "/addons/a0d7b954_vscode/start", ""),
        (Operation::Stop, "/addons/a0d7b954_vscode/stop", ""),
        (Operation::Restart, "/addons/a0d7b954_vscode/restart", ""),
        (
            Operation::UpdateWithBackup,
            "/store/addons/a0d7b954_vscode/update",
            r#"{"backup":true}"#,
        ),
    ] {
        let request = prepare(&app, action).unwrap();
        assert_eq!(request.method(), "POST");
        assert_eq!(request.path(), path);
        assert_eq!(request.body(), body.as_bytes());
        assert!(request.requires_explicit_authorization());
    }
}
#[test]
fn partial_backup_and_targeted_restore_are_scoped_to_only_one_app() {
    let app = AppSlug::parse("core_ssh").unwrap();
    let backup = prepare(&app, Operation::BackupPartial).unwrap();
    assert_eq!(backup.path(), "/backups/new/partial");
    let body: serde_json::Value = serde_json::from_slice(backup.body()).unwrap();
    assert_eq!(body["addons"], serde_json::json!(["core_ssh"]));
    assert_eq!(body["homeassistant"], false);
    assert_eq!(body["folders"], serde_json::json!([]));
    let slug = BackupSlug::parse("a1b2c3d4").unwrap();
    let restore = prepare(&app, Operation::RestorePartial { backup: slug }).unwrap();
    assert_eq!(restore.path(), "/backups/a1b2c3d4/restore/partial");
    let body: serde_json::Value = serde_json::from_slice(restore.body()).unwrap();
    assert_eq!(body["addons"], serde_json::json!(["core_ssh"]));
    assert_eq!(body["homeassistant"], false);
    assert_eq!(body["folders"], serde_json::json!([]));
    for bad in ["/", "hello?world", "a\nb", "", "abc"] {
        assert!(BackupSlug::parse(bad).is_err());
    }
}
#[test]
fn generic_operations_must_fail_closed_without_actual_authorization() {
    let app = AppSlug::parse("core_ssh").unwrap();
    let req = prepare(&app, Operation::UpdateWithBackup).unwrap();
    assert_eq!(
        req.assert_authorized(false),
        Err(ManagerError::NotAuthorized)
    );
    assert_eq!(req.assert_authorized(true), Ok(()));
}

#[test]
fn preview_transport_always_refuses_when_not_on_disposable_qemu() {
    if std::env::var("HARS_P8H_QEMU_ONLY").is_err() {
        let client = AppSupervisorClient::new(
            SupervisorToken::new("synthetic-fixture-not-a-real-token".to_owned()).unwrap(),
        );
        let request = prepare(&AppSlug::parse("core_ssh").unwrap(), Operation::Start).unwrap();
        use hars_supervisor_apps::audit_lab::{FileAuditJournal, grant_recorded};
        use hars_supervisor_apps::rbac_lab::{Actor, Policy, Role};
        use std::{fs, os::unix::fs::PermissionsExt};
        let dir = tempfile::tempdir().unwrap();
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let mut journal = FileAuditJournal::open(&dir.path().join("audit.jsonl")).unwrap();
        let actor = Actor::new("lab_fixture", Role::Administrator).unwrap();
        let mut policy = Policy::default();
        policy.grant(
            "lab_fixture",
            &AppSlug::parse("core_ssh").unwrap(),
            &[request.action()],
        );
        assert!(
            grant_recorded(&policy, None, &request, None, &mut journal)
                .unwrap()
                .is_none()
        );
        let permit = grant_recorded(&policy, Some(&actor), &request, None, &mut journal)
            .unwrap()
            .unwrap();
        assert!(client.preview_execute(permit).is_err());
    }
}
