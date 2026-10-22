#![cfg(feature = "managed-apps-preview")]
use hars_supervisor_apps::managed_apps::{AppSlug, BackupSlug, Operation, prepare};
use hars_supervisor_apps::rbac_lab::{Acknowledgement, Action, Actor, DecisionCode, Policy, Role};

fn actor(name: &str, role: Role) -> Actor {
    Actor::new(name, role).unwrap()
}
fn app(name: &str) -> AppSlug {
    AppSlug::parse(name).unwrap()
}

#[test]
fn access_is_denied_by_default_and_unauthenticated_users_are_never_allowed() {
    let p = Policy::default();
    let req = prepare(&app("core_ssh"), Operation::Start).unwrap();
    assert_eq!(
        p.evaluate(None, &req, None).code,
        DecisionCode::Unauthenticated
    );
    assert_eq!(
        p.evaluate(Some(&actor("admin_1", Role::Administrator)), &req, None)
            .code,
        DecisionCode::UnscopedApp
    );
}

#[test]
fn role_and_app_scope_are_both_enforced_for_every_operation() {
    let mut p = Policy::default();
    p.grant("owner", &app("core_ssh"), &[Action::Start, Action::Restart]);
    p.grant(
        "worker",
        &app("core_ssh"),
        &[Action::Start, Action::Restart],
    );
    p.grant("family", &app("core_ssh"), &[Action::Start]);
    let start = prepare(&app("core_ssh"), Operation::Start).unwrap();
    let different = prepare(&app("other_app"), Operation::Start).unwrap();
    assert_eq!(
        p.evaluate(Some(&actor("owner", Role::Administrator)), &start, None)
            .code,
        DecisionCode::Allowed
    );
    assert_eq!(
        p.evaluate(Some(&actor("worker", Role::AppOperator)), &start, None)
            .code,
        DecisionCode::Allowed
    );
    assert_eq!(
        p.evaluate(Some(&actor("family", Role::Household)), &start, None)
            .code,
        DecisionCode::RoleDenied
    );
    assert_eq!(
        p.evaluate(Some(&actor("family", Role::ReadOnly)), &start, None)
            .code,
        DecisionCode::RoleDenied
    );
    assert_eq!(
        p.evaluate(Some(&actor("owner", Role::Administrator)), &different, None)
            .code,
        DecisionCode::UnscopedApp
    );
    let backup = prepare(&app("core_ssh"), Operation::BackupPartial).unwrap();
    assert_eq!(
        p.evaluate(Some(&actor("owner", Role::Administrator)), &backup, None)
            .code,
        DecisionCode::UnscopedOperation
    );
    let restart = prepare(&app("core_ssh"), Operation::Restart).unwrap();
    assert_eq!(
        p.evaluate(Some(&actor("worker", Role::AppOperator)), &restart, None)
            .code,
        DecisionCode::Allowed
    );
}

#[test]
fn unsafe_actions_need_request_bound_acknowledgement_in_addition_to_role_and_grant() {
    let mut p = Policy::default();
    p.grant(
        "owner",
        &app("core_ssh"),
        &[Action::Restore, Action::Update, Action::Install],
    );
    p.grant("owner", &app("other_app"), &[Action::Restore]);
    let a = actor("owner", Role::Administrator);
    let restore = prepare(
        &app("core_ssh"),
        Operation::RestorePartial {
            backup: BackupSlug::parse("12345678").unwrap(),
        },
    )
    .unwrap();
    let different_backup = prepare(
        &app("core_ssh"),
        Operation::RestorePartial {
            backup: BackupSlug::parse("aaaaaaaa").unwrap(),
        },
    )
    .unwrap();
    assert_eq!(
        p.evaluate(Some(&a), &restore, None).code,
        DecisionCode::AcknowledgementRequired
    );
    let ack = Acknowledgement::bind(&a, &restore);
    assert_eq!(
        p.evaluate(Some(&a), &restore, Some(&ack)).code,
        DecisionCode::Allowed
    );
    assert_eq!(
        p.evaluate(Some(&a), &different_backup, Some(&ack)).code,
        DecisionCode::AcknowledgementMismatch
    );
    let different_app = prepare(
        &app("other_app"),
        Operation::RestorePartial {
            backup: BackupSlug::parse("12345678").unwrap(),
        },
    )
    .unwrap();
    assert_eq!(
        p.evaluate(Some(&a), &different_app, Some(&ack)).code,
        DecisionCode::AcknowledgementMismatch
    );
    assert_eq!(
        p.evaluate(
            Some(&actor("other_user", Role::Administrator)),
            &restore,
            Some(&ack)
        )
        .code,
        DecisionCode::UnscopedApp
    );
    let update = prepare(&app("core_ssh"), Operation::UpdateWithBackup).unwrap();
    assert_eq!(
        p.evaluate(Some(&a), &update, Some(&ack)).code,
        DecisionCode::AcknowledgementMismatch
    );
}

#[test]
fn operator_can_manage_only_non_privileged_operations_on_explicit_apps() {
    let mut p = Policy::default();
    let slug = app("core_ssh");
    p.grant(
        "op",
        &slug,
        &[
            Action::Start,
            Action::Stop,
            Action::Restart,
            Action::Backup,
            Action::Install,
            Action::Update,
            Action::Restore,
        ],
    );
    let a = actor("op", Role::AppOperator);
    assert_eq!(
        p.evaluate(Some(&a), &prepare(&slug, Operation::Start).unwrap(), None)
            .code,
        DecisionCode::Allowed
    );
    assert_eq!(
        p.evaluate(Some(&a), &prepare(&slug, Operation::Install).unwrap(), None)
            .code,
        DecisionCode::RoleDenied
    );
    assert_eq!(
        p.evaluate(
            Some(&a),
            &prepare(&slug, Operation::UpdateWithBackup).unwrap(),
            None
        )
        .code,
        DecisionCode::RoleDenied
    );
    let stop = prepare(&slug, Operation::Stop).unwrap();
    assert_eq!(
        p.evaluate(Some(&a), &stop, None).code,
        DecisionCode::AcknowledgementRequired
    );
    assert_eq!(
        p.evaluate(Some(&a), &stop, Some(&Acknowledgement::bind(&a, &stop)))
            .code,
        DecisionCode::Allowed
    );
}

#[test]
fn audit_events_are_sanitized_even_on_denials() {
    assert!(Actor::new("invalid user\nSECRET", Role::Administrator).is_err());
    assert!(Actor::new("", Role::Administrator).is_err());
    let p = Policy::default();
    let a = actor("verified_subject", Role::Administrator);
    let req = prepare(
        &app("core_ssh"),
        Operation::RestorePartial {
            backup: BackupSlug::parse("12345678").unwrap(),
        },
    )
    .unwrap();
    let d = p.evaluate(Some(&a), &req, None);
    assert_eq!(d.audit.subject, "verified_subject");
    assert_eq!(d.audit.app, "core_ssh");
    assert_eq!(d.audit.action, Action::Restore);
    assert_eq!(d.audit.code, DecisionCode::UnscopedApp);
    let trace = serde_json::to_string(&d.audit).unwrap();
    assert!(!trace.contains("12345678"));
    assert!(!trace.contains("Bearer"));
    assert!(!trace.contains("token"));
}
