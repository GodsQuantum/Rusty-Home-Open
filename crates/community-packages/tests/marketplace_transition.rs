use hars_community_packages::{
    Kind,
    marketplace_transition::{CatalogueState, LegacyEntry, TransitionError, plan_transition},
};

fn repo(id: &str, kind: Kind, custom: bool, catalogue: CatalogueState) -> LegacyEntry {
    LegacyEntry {
        repository: id.to_string(),
        kind,
        custom,
        catalogue,
        update_entity_id: Some(format!("update.{}", id.replace('/', "_"))),
    }
}

#[test]
fn preserves_existing_catalog_and_custom_repo_update_entity_ids_and_links() {
    let entries = vec![
        repo(
            "Alice/thermostat",
            Kind::Integration,
            false,
            CatalogueState::Normal,
        ),
        repo("Bob/my-card", Kind::Dashboard, true, CatalogueState::Normal),
        repo(
            "Carol/quiet-theme",
            Kind::Theme,
            false,
            CatalogueState::Normal,
        ),
        repo(
            "Dee/template",
            Kind::Template,
            false,
            CatalogueState::Normal,
        ),
    ];
    let plan = plan_transition(&entries).unwrap();
    assert_eq!(plan.managed.len(), 4);
    assert_eq!(plan.managed[0].repository, "Alice/thermostat");
    assert_eq!(plan.managed[0].update_entity_id, "update.Alice_thermostat");
    assert!(plan.managed[1].requires_github_connection);
    assert!(!plan.managed[0].requires_github_connection);
    assert_eq!(plan.managed[3].kind, Kind::Template);
    assert!(plan.legacy_hacs_navigation);
    assert!(plan.legacy_hacsfiles_resolution);
    assert!(!plan.activation_authorized);
    assert!(!plan.applies_changes);
}

#[test]
fn keeps_unsupported_legacy_types_as_user_data_but_without_updates() {
    let entries = vec![
        repo("A/python", Kind::PythonScript, true, CatalogueState::Normal),
        repo("B/appdaemon", Kind::AppDaemon, true, CatalogueState::Normal),
        repo(
            "C/removed",
            Kind::Integration,
            false,
            CatalogueState::Removed,
        ),
        repo("D/critical", Kind::Theme, false, CatalogueState::Critical),
    ];
    let plan = plan_transition(&entries).unwrap();
    assert!(plan.managed.is_empty());
    assert_eq!(plan.unmanaged_preserved.len(), 3);
    assert_eq!(plan.critical_hold.len(), 1);
    assert_eq!(plan.critical_hold[0].repository, "D/critical");
    assert!(!plan.applies_changes);
}

#[test]
fn rejects_duplicate_repositories_update_entity_ids_and_bad_input() {
    let good = repo("A/repo", Kind::Integration, false, CatalogueState::Normal);
    let mut duplicate_repo = repo("a/REPO", Kind::Theme, false, CatalogueState::Normal);
    duplicate_repo.update_entity_id = Some("update.other".to_owned());
    assert!(matches!(
        plan_transition(&[good.clone(), duplicate_repo]),
        Err(TransitionError::DuplicateRepository)
    ));
    let mut another = repo("B/other", Kind::Theme, false, CatalogueState::Normal);
    another.update_entity_id = good.update_entity_id.clone();
    assert!(matches!(
        plan_transition(&[good.clone(), another]),
        Err(TransitionError::DuplicateUpdateEntity)
    ));
    for input in [
        "",
        "owner/../escape",
        "owner/valid/extra",
        "../bad",
        "bad name/repo",
        "owner\\repo",
    ] {
        assert!(
            plan_transition(&[repo(
                input,
                Kind::Integration,
                false,
                CatalogueState::Normal
            )])
            .is_err(),
            "{input}"
        );
    }
    let mut malformed = good.clone();
    malformed.update_entity_id = Some("button.wrong_type".into());
    assert!(matches!(
        plan_transition(&[malformed]),
        Err(TransitionError::UnsafeEntityId)
    ));
    let unknown = repo(
        "A/ha-app",
        Kind::SupervisorApp,
        false,
        CatalogueState::Normal,
    );
    assert!(matches!(
        plan_transition(&[unknown]),
        Err(TransitionError::UnsupportedAppKind)
    ));
}

#[test]
fn critical_repository_never_auto_uninstalls_even_if_custom() {
    let plan = plan_transition(&[repo(
        "Bad/risk",
        Kind::Integration,
        true,
        CatalogueState::Critical,
    )])
    .unwrap();
    assert!(plan.managed.is_empty());
    assert_eq!(plan.critical_hold.len(), 1);
    assert!(!plan.activation_authorized && !plan.applies_changes);
}
