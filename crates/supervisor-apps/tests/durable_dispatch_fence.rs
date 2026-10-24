#![cfg(feature = "durable-dispatch-fence-lab")]
use hars_supervisor_apps::{
    audit_lab::{FileAuditJournal, grant_recorded},
    durable_dispatch_fence_lab::{DispatchState, DurableDispatchFence, TrackedError},
    managed_apps::{AppSlug, Operation, prepare},
    official_transport::{AppSupervisorClient, SupervisorToken, TransportError},
    rbac_lab::{Action, Actor, Policy, Role},
};
use std::{fs, os::unix::fs::PermissionsExt};

fn private_dir() -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    fs::set_permissions(d.path(), fs::Permissions::from_mode(0o700)).unwrap();
    d
}
fn permitted(dir: &std::path::Path, app: &str) -> hars_supervisor_apps::audit_lab::AuditedPermit {
    let app = AppSlug::parse(app).unwrap();
    let mut journal = FileAuditJournal::open(&dir.join("audit.jsonl")).unwrap();
    let actor = Actor::new("core_fixture_admin", Role::Administrator).unwrap();
    let mut policy = Policy::default();
    policy.grant(actor.subject(), &app, &[Action::Start]);
    let request = prepare(&app, Operation::Start).unwrap();
    assert!(
        grant_recorded(&policy, None, &request, None, &mut journal)
            .unwrap()
            .is_none()
    );
    grant_recorded(&policy, Some(&actor), &request, None, &mut journal)
        .unwrap()
        .unwrap()
}

#[test]
fn durable_fence_prevents_second_post_after_crash_or_restart() {
    let dir = private_dir();
    let path = dir.path().join("dispatch.redb");
    let armed_id = {
        let mut fence = DurableDispatchFence::open(&path).unwrap();
        let armed = fence
            .arm(permitted(dir.path(), "new_vendor_x"), 100)
            .unwrap();
        let id = armed.id().to_string();
        assert_eq!(id.len(), 64);

        assert_eq!(
            fence.get(&id).unwrap().unwrap().state,
            DispatchState::Uncertain
        );
        assert!(
            fence
                .arm(permitted(dir.path(), "new_vendor_x"), 101)
                .is_err()
        );
        // Dropping a one-shot permit without a POST must not silently remove the lock.
        drop(armed);
        id
    };
    let mut reopened = DurableDispatchFence::open(&path).unwrap();
    assert_eq!(
        reopened.get(&armed_id).unwrap().unwrap().app,
        "new_vendor_x"
    );
    assert!(
        reopened
            .arm(permitted(dir.path(), "new_vendor_x"), 102)
            .is_err()
    );
    assert!(
        reopened
            .arm(permitted(dir.path(), "other_vendor"), 103)
            .is_ok()
    );
    assert!(reopened.get("bad-id").is_err());
}

#[test]
fn even_preview_disabled_transport_error_retains_uncertain_fence() {
    let dir = private_dir();
    let db = dir.path().join("fence.redb");
    let mut fence = DurableDispatchFence::open(&db).unwrap();
    let armed = fence
        .arm(permitted(dir.path(), "brand_new_2040"), 100)
        .unwrap();
    let id = armed.id().to_owned();
    let client = AppSupervisorClient::new(SupervisorToken::new("synthetic_secret".into()).unwrap());
    // Tests run outside disposable HAOS: the preview transport MUST deny.
    assert!(matches!(
        client.preview_execute_fenced(&mut fence, armed),
        Err(TrackedError::Transport(
            TransportError::ManagedPreviewDisabled
        ))
    ));
    assert_eq!(
        fence.get(&id).unwrap().unwrap().state,
        DispatchState::Uncertain
    );
    drop(fence);
    let mut fence = DurableDispatchFence::open(&db).unwrap();
    assert!(
        fence
            .arm(permitted(dir.path(), "brand_new_2040"), 101)
            .is_err()
    );
}

#[test]
fn unsafe_db_permissions_symlink_and_hardlink_are_denied() {
    let d = tempfile::tempdir().unwrap();
    assert!(DurableDispatchFence::open(&d.path().join("x.redb")).is_err());
    fs::set_permissions(d.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let target = d.path().join("original");
    fs::write(&target, b"invalid").unwrap();
    std::os::unix::fs::symlink(&target, d.path().join("link.redb")).unwrap();
    assert!(DurableDispatchFence::open(&d.path().join("link.redb")).is_err());
    fs::hard_link(&target, d.path().join("hard.redb")).unwrap();
    assert!(DurableDispatchFence::open(&d.path().join("hard.redb")).is_err());
}

#[test]
fn invalid_existing_database_cannot_be_claimed() {
    let d = private_dir();
    let path = d.path().join("fence.redb");
    fs::write(&path, b"corrupt-redb-data").unwrap();
    assert!(DurableDispatchFence::open(&path).is_err());
}
