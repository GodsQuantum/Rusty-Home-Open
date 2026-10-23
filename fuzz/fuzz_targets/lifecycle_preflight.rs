#![no_main]
// Fuzz the actual Rusty Home seven-operation App preflight against
// conflicting Supervisor Store / Detail / Installed inventory observations.
// No HTTP, Supervisor credentials, real user data, writes, or containers.
use hars_supervisor_apps::{
    app_detail::AppDetail,
    installed_inventory::{RunningApp, RunningInventory},
    official_store::{ApiVersion, App, Catalog, Repo},
    operation_preflight::{assess_lifecycle_evidence, EvidenceStatus, LifecycleAction},
};
use libfuzzer_sys::fuzz_target;

fn opt_bool(input: u8) -> Option<bool> {
    match input % 3 {
        0 => None,
        1 => Some(false),
        _ => Some(true),
    }
}
fn state(input: u8) -> Option<String> {
    match input % 6 {
        0 => None,
        1 => Some("started".into()),
        2 => Some("stopped".into()),
        3 => Some("error".into()),
        4 => Some("unknown".into()),
        _ => Some("starting".into()),
    }
}
fn version(input: u8) -> Option<String> {
    match input % 5 {
        0 => None,
        1 => Some("1.0".into()),
        2 => Some("2.0".into()),
        3 => Some("".into()),
        _ => Some("9.9.9-dev".into()),
    }
}

fuzz_target!(|raw: &[u8]| {
    if raw.len() > 4096 {
        return;
    }
    // No blacklisted vendor list: fuzz arbitrary but safe, bounded slugs.
    let slug_suffix = raw.first().copied().unwrap_or(0);
    let slug = format!("unfamiliar_vendor_{slug_suffix:03}");
    let repository = "unknown_repositorystable".to_string();
    let byte = |i: usize| raw.get(i).copied().unwrap_or(0);

    let installed = byte(1) & 1 != 0;
    let runtime_present = byte(2) & 1 != 0;
    let update_available = byte(3) & 1 != 0;
    let state_inventory = state(byte(4));
    let state_detail = state(byte(5));
    let current_version = version(byte(6));
    let target_version = version(byte(7));
    let running_version = version(byte(8));
    let api = if byte(9) & 1 == 0 {
        ApiVersion::V1
    } else {
        ApiVersion::V2
    };
    let other_api = if byte(10) & 1 == 0 {
        api
    } else if api == ApiVersion::V1 {
        ApiVersion::V2
    } else {
        ApiVersion::V1
    };
    let mut catalog = Catalog {
        api_version: api,
        official_supervisor_owner: byte(11) & 1 != 0,
        activation_authorized: byte(12) & 1 != 0,
        repositories: vec![Repo {
            slug: repository.clone(),
            name: "Unfamiliar source".into(),
        }],
        apps: vec![App {
            slug: slug.clone(),
            repository: repository.clone(),
            name: "Generic App".into(),
            available: byte(13) & 1 != 0,
            installed,
            update_available,
        }],
    };
    let mut detail = AppDetail {
        slug: if byte(14) & 1 != 0 {
            "other_vendor".into()
        } else {
            slug.clone()
        },
        name: "Generic App".into(),
        repository: if byte(15) & 1 == 0 {
            Some(repository.clone())
        } else {
            Some("wrong_repo".into())
        },
        version: current_version,
        latest_version: target_version,
        state: state_detail,
        available: opt_bool(byte(16)),
        update_available: opt_bool(byte(17)),
        system_managed: opt_bool(byte(18)),
        ingress: Some(false),
        official_supervisor_owner: byte(19) & 1 != 0,
        activation_authorized: byte(20) & 1 != 0,
    };
    let inventory = RunningInventory {
        api_version: other_api,
        read_only: byte(21) & 1 != 0,
        apps: if runtime_present {
            vec![RunningApp {
                slug: slug.clone(),
                name: "Generic App".into(),
                repository: Some(repository.clone()),
                state: state_inventory,
                version: running_version,
                update_available: opt_bool(byte(22)),
            }]
        } else {
            vec![]
        },
    };
    if byte(23) & 1 != 0 {
        catalog.apps.push(catalog.apps[0].clone());
    }
    if byte(24) & 1 != 0 {
        catalog.repositories.clear();
    }
    if byte(25) & 1 != 0 {
        detail.system_managed = Some(true);
    }
    for action in [
        LifecycleAction::Install,
        LifecycleAction::Start,
        LifecycleAction::Stop,
        LifecycleAction::Restart,
        LifecycleAction::Update,
        LifecycleAction::Backup,
        LifecycleAction::Restore,
    ] {
        if let Ok(plan) = assess_lifecycle_evidence(&catalog, &detail, &inventory, &slug, action) {
            assert!(plan.read_only);
            assert!(!plan.activation_authorized);
            assert_eq!(plan.slug, slug);
            assert_eq!(plan.action, action);
            assert_eq!(
                plan.platform_candidate,
                plan.status == EvidenceStatus::EligibleForReview
            );
            assert_eq!(
                plan.verified_backup_required,
                matches!(action, LifecycleAction::Update | LifecycleAction::Restore)
            );
            if matches!(action, LifecycleAction::Update | LifecycleAction::Restore) {
                assert!(!plan.platform_candidate);
            }
            if detail.system_managed == Some(true) && plan.status != EvidenceStatus::Inconsistent {
                assert_eq!(plan.status, EvidenceStatus::SystemManaged);
            }
        }
    }
});
