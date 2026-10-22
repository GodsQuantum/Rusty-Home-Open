#![cfg(feature = "managed-apps-preview")]
use hars_supervisor_apps::audit_lab::{AuditSink, FileAuditJournal, authorize_recorded};
use hars_supervisor_apps::managed_apps::{AppSlug, BackupSlug, Operation, prepare};
use hars_supervisor_apps::rbac_lab::{Action, Actor, AuditRecord, DecisionCode, Policy, Role};
use std::{fs, io, os::unix::fs::PermissionsExt};

fn private_dir() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
    dir
}

fn fixture() -> (
    Policy,
    Actor,
    hars_supervisor_apps::managed_apps::PreparedRequest,
) {
    let app = AppSlug::parse("core_ssh").unwrap();
    let actor = Actor::new("alice", Role::Administrator).unwrap();
    let mut policy = Policy::default();
    policy.grant("alice", &app, &[Action::Start]);
    (policy, actor, prepare(&app, Operation::Start).unwrap())
}
#[test]
fn permitted_and_denied_attempts_are_synchronously_logged() {
    let dir = private_dir();
    let path = dir.path().join("audit.jsonl");
    let mut journal = FileAuditJournal::open(&path).unwrap();
    let (policy, actor, req) = fixture();
    assert_eq!(
        authorize_recorded(&policy, Some(&actor), &req, None, &mut journal)
            .unwrap()
            .code,
        DecisionCode::Allowed
    );
    assert_eq!(
        authorize_recorded(&policy, None, &req, None, &mut journal)
            .unwrap()
            .code,
        DecisionCode::Unauthenticated
    );
    drop(journal);
    let log = fs::read_to_string(&path).unwrap();
    let rows = log
        .lines()
        .map(|l| serde_json::from_str::<serde_json::Value>(l).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["event"]["subject"], "alice");
    assert_eq!(rows[0]["event"]["code"], "allowed");
    assert_eq!(rows[1]["event"]["code"], "unauthenticated");
    assert!(rows[0]["time_unix_ms"].is_u64());
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert!(FileAuditJournal::open(&path).is_ok());
}
struct Broken;
impl AuditSink for Broken {
    fn append_sync(&mut self, _: &AuditRecord) -> io::Result<()> {
        Err(io::Error::from(io::ErrorKind::PermissionDenied))
    }
}
#[test]
fn permission_failure_blocks_an_allowed_decision_and_a_denial() {
    let (p, actor, req) = fixture();
    for who in [Some(&actor), None] {
        assert_eq!(
            authorize_recorded(&p, who, &req, None, &mut Broken)
                .unwrap_err()
                .kind(),
            io::ErrorKind::PermissionDenied
        );
    }
}
#[test]
fn rejects_insecure_symlinked_and_corrupt_logs_without_truncation() {
    let dir = private_dir();
    let path = dir.path().join("audit.jsonl");
    let mut journal = FileAuditJournal::open(&path).unwrap();
    let (p, a, r) = fixture();
    authorize_recorded(&p, Some(&a), &r, None, &mut journal).unwrap();
    drop(journal);
    std::os::unix::fs::symlink(&path, dir.path().join("link")).unwrap();
    assert!(FileAuditJournal::open(&dir.path().join("link")).is_err());
    let old = fs::read(&path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(FileAuditJournal::open(&path).is_err());
    assert_eq!(fs::read(&path).unwrap(), old);
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    fs::write(&path, b"broken-json\n").unwrap();
    assert!(FileAuditJournal::open(&path).is_err());
    assert_eq!(fs::read(&path).unwrap(), b"broken-json\n");
    let d = dir.path().join("public");
    fs::create_dir(&d).unwrap();
    fs::set_permissions(&d, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(FileAuditJournal::open(&d.join("new.jsonl")).is_err());
}
#[test]
fn backup_ids_and_tokens_never_appear_in_logged_denials() {
    let dir = private_dir();
    let path = dir.path().join("audit.jsonl");
    let mut journal = FileAuditJournal::open(&path).unwrap();
    let req = prepare(
        &AppSlug::parse("core_ssh").unwrap(),
        Operation::RestorePartial {
            backup: BackupSlug::parse("deadbeef").unwrap(),
        },
    )
    .unwrap();
    assert_eq!(
        authorize_recorded(&Policy::default(), None, &req, None, &mut journal)
            .unwrap()
            .code,
        DecisionCode::Unauthenticated
    );
    drop(journal);
    let log = fs::read_to_string(&path).unwrap();
    assert!(!log.contains("deadbeef"));
    assert!(!log.contains("SUPERVISOR_TOKEN"));
    assert!(log.contains("restore"));
}
