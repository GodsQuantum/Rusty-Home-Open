//! Read-only generic Supervisor App lifecycle evidence. All apps and vendors
//! are accepted through their validated slugs; never produce a write permit.
//! Current authorized user, one-time consent, verified backup and rollback are
//! NOT known here. This module does not issue HTTP requests or Supervisor POSTs.
use crate::{
    app_detail::AppDetail,
    installed_inventory::RunningInventory,
    official_store::Catalog,
    store_availability::{AvailabilityError, AvailabilityStatus, assess_store_availability},
};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleAction {
    Install,
    Start,
    Stop,
    Restart,
    Update,
    Backup,
    Restore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceStatus {
    EligibleForReview,
    AlreadyInstalled,
    NotInstalled,
    AlreadyRunning,
    AlreadyStopped,
    UpdateNotConfirmed,
    Unavailable,
    SystemManaged,
    Inconsistent,
    UnknownState,
    VerifiedBackupRequired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LifecycleEvidence {
    pub slug: String,
    pub action: LifecycleAction,
    pub status: EvidenceStatus,
    /// Pure environment signal, never an instruction to execute.
    pub platform_candidate: bool,
    pub verified_backup_required: bool,
    pub read_only: bool,
    pub activation_authorized: bool,
}

/// Reconcile fresh Store catalogue, installed inventory and Store details.
/// The three observations are owned by the official Supervisor client.
pub fn assess_lifecycle_evidence(
    catalog: &Catalog,
    detail: &AppDetail,
    inventory: &RunningInventory,
    slug: &str,
    action: LifecycleAction,
) -> Result<LifecycleEvidence, AvailabilityError> {
    let availability = assess_store_availability(catalog, detail, inventory, slug)?;
    // The availability checker ensures single matching catalogue member,
    // correct owner, API version, repository and the exact requested slug.
    let listed = catalog
        .apps
        .iter()
        .find(|item| item.slug == slug)
        .ok_or(AvailabilityError::NotInCatalog)?;
    let runtime = inventory.apps.iter().find(|item| item.slug == slug);
    let installed = listed.installed || runtime.is_some();
    let conflict = listed.installed != runtime.is_some();
    let system_managed = detail.system_managed == Some(true);
    // An explicitly reported runtime state must not contradict Store detail.
    let state_conflict = runtime.is_some_and(|entry| {
        detail
            .state
            .as_ref()
            .zip(entry.state.as_ref())
            .is_some_and(|(left, right)| left != right)
    });
    let state = runtime
        .and_then(|entry| entry.state.as_deref())
        .or(detail.state.as_deref());

    let status = if conflict || state_conflict {
        EvidenceStatus::Inconsistent
    } else if system_managed {
        EvidenceStatus::SystemManaged
    } else {
        match action {
            LifecycleAction::Install => {
                if installed {
                    EvidenceStatus::AlreadyInstalled
                } else {
                    match availability.status {
                        AvailabilityStatus::Available => EvidenceStatus::EligibleForReview,
                        AvailabilityStatus::SystemManaged => EvidenceStatus::SystemManaged,
                        AvailabilityStatus::Unavailable => EvidenceStatus::Unavailable,
                        _ => EvidenceStatus::UnknownState,
                    }
                }
            }
            _ if !installed => EvidenceStatus::NotInstalled,
            LifecycleAction::Start => match state {
                Some("stopped") => EvidenceStatus::EligibleForReview,
                Some("started") => EvidenceStatus::AlreadyRunning,
                _ => EvidenceStatus::UnknownState,
            },
            LifecycleAction::Stop => match state {
                Some("started") => EvidenceStatus::EligibleForReview,
                Some("stopped") => EvidenceStatus::AlreadyStopped,
                _ => EvidenceStatus::UnknownState,
            },
            LifecycleAction::Restart => match state {
                Some("started") => EvidenceStatus::EligibleForReview,
                _ => EvidenceStatus::UnknownState,
            },
            LifecycleAction::Update => {
                // A bare update_available flag is insufficient; confirm that
                // both installed views know the SAME old version and that a
                // concrete, different target version is advertised.
                let matching_versions = detail
                    .version
                    .as_ref()
                    .zip(runtime.and_then(|entry| entry.version.as_ref()))
                    .zip(detail.latest_version.as_ref())
                    .is_some_and(|((installed, running), latest)| {
                        !installed.is_empty()
                            && !latest.is_empty()
                            && installed == running
                            && installed != latest
                    });
                if listed.available
                    && detail.available == Some(true)
                    && listed.update_available
                    && detail.update_available == Some(true)
                    && runtime.and_then(|app| app.update_available) == Some(true)
                    && matching_versions
                {
                    // An update cannot be treated as ready until the actual
                    // backup destination and rollback path are verified.
                    EvidenceStatus::VerifiedBackupRequired
                } else {
                    EvidenceStatus::UpdateNotConfirmed
                }
            }
            LifecycleAction::Backup => EvidenceStatus::EligibleForReview,
            LifecycleAction::Restore => EvidenceStatus::VerifiedBackupRequired,
        }
    };
    Ok(LifecycleEvidence {
        slug: slug.to_owned(),
        action,
        status,
        platform_candidate: status == EvidenceStatus::EligibleForReview,
        verified_backup_required: matches!(
            action,
            LifecycleAction::Update | LifecycleAction::Restore
        ),
        read_only: true,
        activation_authorized: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        installed_inventory::RunningApp,
        official_store::{ApiVersion, App, Repo},
    };

    fn fixture(slug: &str, installed: bool) -> (Catalog, AppDetail, RunningInventory) {
        let repo = "unfamiliar_vendor_918";
        let catalog = Catalog {
            api_version: ApiVersion::V2,
            official_supervisor_owner: true,
            activation_authorized: false,
            repositories: vec![Repo {
                slug: repo.into(),
                name: "Unknown".into(),
            }],
            apps: vec![App {
                slug: slug.into(),
                repository: repo.into(),
                name: "Unknown App".into(),
                available: true,
                installed,
                update_available: false,
            }],
        };
        let detail = AppDetail {
            slug: slug.into(),
            name: "Unknown App".into(),
            repository: Some(repo.into()),
            version: Some("1.0".into()),
            latest_version: Some("2.0".into()),
            state: if installed {
                Some("stopped".into())
            } else {
                None
            },
            available: Some(true),
            update_available: Some(false),
            system_managed: Some(false),
            ingress: Some(false),
            official_supervisor_owner: true,
            activation_authorized: false,
        };
        let inventory = RunningInventory {
            api_version: ApiVersion::V2,
            read_only: true,
            apps: if installed {
                vec![RunningApp {
                    slug: slug.into(),
                    name: "Unknown App".into(),
                    repository: Some(repo.into()),
                    state: Some("stopped".into()),
                    version: Some("1.0".into()),
                    update_available: Some(false),
                }]
            } else {
                vec![]
            },
        };
        (catalog, detail, inventory)
    }
    fn status(
        f: &(Catalog, AppDetail, RunningInventory),
        slug: &str,
        op: LifecycleAction,
    ) -> EvidenceStatus {
        let v = assess_lifecycle_evidence(&f.0, &f.1, &f.2, slug, op).unwrap();
        assert!(v.read_only);
        assert!(!v.activation_authorized);
        v.status
    }
    #[test]
    fn install_and_lifecycle_only_when_current_supervisor_sources_agree() {
        let slug = "future_vendor_9182";
        let f = fixture(slug, false);
        assert_eq!(
            status(&f, slug, LifecycleAction::Install),
            EvidenceStatus::EligibleForReview
        );
        for op in [
            LifecycleAction::Start,
            LifecycleAction::Stop,
            LifecycleAction::Restart,
            LifecycleAction::Update,
            LifecycleAction::Backup,
            LifecycleAction::Restore,
        ] {
            assert_eq!(status(&f, slug, op), EvidenceStatus::NotInstalled);
        }
        let mut installed = fixture(slug, true);
        assert_eq!(
            status(&installed, slug, LifecycleAction::Install),
            EvidenceStatus::AlreadyInstalled
        );
        assert_eq!(
            status(&installed, slug, LifecycleAction::Start),
            EvidenceStatus::EligibleForReview
        );
        assert_eq!(
            status(&installed, slug, LifecycleAction::Stop),
            EvidenceStatus::AlreadyStopped
        );
        assert_eq!(
            status(&installed, slug, LifecycleAction::Restore),
            EvidenceStatus::VerifiedBackupRequired
        );
        installed.1.state = Some("started".into());
        installed.2.apps[0].state = Some("started".into());
        assert_eq!(
            status(&installed, slug, LifecycleAction::Restart),
            EvidenceStatus::EligibleForReview
        );
        assert_eq!(
            status(&installed, slug, LifecycleAction::Stop),
            EvidenceStatus::EligibleForReview
        );
        installed.0.apps[0].update_available = true;
        installed.1.update_available = Some(true);
        installed.2.apps[0].update_available = Some(true);
        let update = assess_lifecycle_evidence(
            &installed.0,
            &installed.1,
            &installed.2,
            slug,
            LifecycleAction::Update,
        )
        .unwrap();
        assert_eq!(update.status, EvidenceStatus::VerifiedBackupRequired);
        assert!(update.verified_backup_required);
        assert!(!update.platform_candidate && !update.activation_authorized);
    }
    #[test]
    fn tampering_unknown_states_system_apps_and_store_disagreement_fail_closed() {
        let slug = "new_vendor_2281";
        let mut f = fixture(slug, true);
        f.0.apps[0].installed = false;
        assert_eq!(
            status(&f, slug, LifecycleAction::Start),
            EvidenceStatus::Inconsistent
        );
        f.0.apps[0].installed = true;
        f.1.state = Some("started".into());
        assert_eq!(
            status(&f, slug, LifecycleAction::Start),
            EvidenceStatus::Inconsistent
        );
        f.1.state = None;
        f.2.apps[0].state = None;
        assert_eq!(
            status(&f, slug, LifecycleAction::Start),
            EvidenceStatus::UnknownState
        );
        f.1.system_managed = Some(true);
        assert_eq!(
            status(&f, slug, LifecycleAction::Backup),
            EvidenceStatus::SystemManaged
        );
        f.1.system_managed = Some(false);
        f.0.official_supervisor_owner = false;
        assert_eq!(
            assess_lifecycle_evidence(&f.0, &f.1, &f.2, slug, LifecycleAction::Install),
            Err(AvailabilityError::UntrustedSource)
        );
    }
    #[test]
    fn update_cannot_be_suggested_without_consistent_versions_and_backup_proof() {
        let slug = "another_vendor_814";
        let mut f = fixture(slug, true);
        f.0.apps[0].update_available = true;
        f.1.update_available = Some(true);
        f.2.apps[0].update_available = Some(true);
        assert_eq!(
            status(&f, slug, LifecycleAction::Update),
            EvidenceStatus::VerifiedBackupRequired
        );
        f.2.apps[0].version = Some("0.9".into());
        assert_eq!(
            status(&f, slug, LifecycleAction::Update),
            EvidenceStatus::UpdateNotConfirmed
        );
        f.2.apps[0].version = Some("1.0".into());
        f.1.latest_version = Some("1.0".into());
        assert_eq!(
            status(&f, slug, LifecycleAction::Update),
            EvidenceStatus::UpdateNotConfirmed
        );
        f.1.latest_version = Some("2.0".into());
        f.0.apps[0].available = false;
        assert_eq!(
            status(&f, slug, LifecycleAction::Update),
            EvidenceStatus::UpdateNotConfirmed
        );
    }
    #[test]
    fn unrelated_vendors_never_require_hardcoded_app_allowlists() {
        for n in 0..1017 {
            let slug = format!("anyfutureprovider_{n:04}");
            let f = fixture(&slug, false);
            assert_eq!(
                status(&f, &slug, LifecycleAction::Install),
                EvidenceStatus::EligibleForReview
            );
        }
    }
}
