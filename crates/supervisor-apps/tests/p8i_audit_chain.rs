#![cfg(feature = "managed-apps-preview")]
use hars_supervisor_apps::audit_lab::{AuditSink, FileAuditJournal};
use hars_supervisor_apps::rbac_lab::{Action, AuditRecord, DecisionCode};
use std::{fs, os::unix::fs::PermissionsExt, process::Command};

fn event(subject: &str) -> AuditRecord {
    AuditRecord {
        subject: subject.to_owned(),
        app: "core_ssh".into(),
        action: Action::Start,
        code: DecisionCode::Allowed,
    }
}
fn private_dir() -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    fs::set_permissions(d.path(), fs::Permissions::from_mode(0o700)).unwrap();
    d
}
#[test]
fn detects_valid_json_tampering_on_reopen_and_before_next_write() {
    let d = private_dir();
    let path = d.path().join("audit.jsonl");
    let mut j = FileAuditJournal::open(&path).unwrap();
    j.append_sync(&event("alice")).unwrap();
    j.append_sync(&event("bravo")).unwrap();
    let original = fs::read_to_string(&path).unwrap();
    assert!(original.contains("prev_hash"));
    assert!(original.contains("hash"));
    assert!(original.contains("alice"));
    let forged = original.replace("alice", "alien");
    assert_eq!(forged.len(), original.len());
    fs::write(&path, &forged).unwrap();
    assert!(FileAuditJournal::open(&path).is_err());
    assert!(j.append_sync(&event("charlie")).is_err());
    assert_eq!(fs::read_to_string(&path).unwrap(), forged);
}
#[test]
fn detects_missing_middle_or_first_record_even_when_json_valid() {
    let d = private_dir();
    let path = d.path().join("audit.jsonl");
    let mut j = FileAuditJournal::open(&path).unwrap();
    for subject in ["alice", "bravo", "charlie"] {
        j.append_sync(&event(subject)).unwrap();
    }
    drop(j);
    let lines = fs::read_to_string(&path)
        .unwrap()
        .lines()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    assert_eq!(lines.len(), 3);
    fs::write(&path, format!("{}\n{}\n", lines[0], lines[2])).unwrap();
    assert!(FileAuditJournal::open(&path).is_err());
    fs::write(&path, format!("{}\n{}\n", lines[1], lines[2])).unwrap();
    assert!(FileAuditJournal::open(&path).is_err());
}
#[test]
fn child_writer_fixture() {
    let Some(path) = std::env::var_os("P8ID2_TEST_JOURNAL_PATH") else {
        return;
    };
    let path = std::path::PathBuf::from(path);
    for i in 0..10 {
        let mut journal = FileAuditJournal::open(&path).unwrap();
        let _ = i;
        journal.append_sync(&event("multiprocess")).unwrap();
    }
}
#[test]
fn separate_processes_append_without_interleaving_or_lost_records() {
    let d = private_dir();
    let path = d.path().join("audit.jsonl");
    let exe = std::env::current_exe().unwrap();
    let mut children = Vec::new();
    for _ in 0..4 {
        children.push(
            Command::new(&exe)
                .arg("--exact")
                .arg("child_writer_fixture")
                .env("P8ID2_TEST_JOURNAL_PATH", &path)
                .spawn()
                .unwrap(),
        );
    }
    for mut child in children {
        assert!(child.wait().unwrap().success());
    }
    let rows = fs::read_to_string(&path).unwrap();
    assert_eq!(rows.lines().count(), 40);
    FileAuditJournal::open(&path).expect("serialized chained journal must revalidate");
}
