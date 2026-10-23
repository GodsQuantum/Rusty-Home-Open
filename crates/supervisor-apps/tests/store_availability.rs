use hars_supervisor_apps::app_detail::AppDetail;
use hars_supervisor_apps::installed_inventory::{RunningApp, RunningInventory};
use hars_supervisor_apps::official_store::{ApiVersion, App, Catalog, Repo};
use hars_supervisor_apps::store_availability::{
    AvailabilityError, AvailabilityStatus, assess_store_availability,
};

fn catalog() -> Catalog {
    Catalog {
        api_version: ApiVersion::V1,
        repositories: vec![Repo {
            slug: "future_vendor".into(),
            name: "Any vendor".into(),
        }],
        apps: vec![App {
            slug: "future_vendor_app".into(),
            repository: "future_vendor".into(),
            name: "Any App".into(),
            available: true,
            installed: false,
            update_available: false,
        }],
        official_supervisor_owner: true,
        activation_authorized: false,
    }
}

fn inventory() -> RunningInventory {
    RunningInventory {
        api_version: ApiVersion::V1,
        apps: vec![],
        read_only: true,
    }
}

fn detail() -> AppDetail {
    AppDetail {
        slug: "future_vendor_app".into(),
        name: "Any App".into(),
        repository: Some("future_vendor".into()),
        version: None,
        latest_version: Some("1.0.0".into()),
        state: None,
        available: Some(true),
        update_available: Some(false),
        system_managed: None,
        ingress: Some(true),
        official_supervisor_owner: true,
        activation_authorized: false,
    }
}

#[test]
fn available_from_both_live_sources_never_authorizes_a_write() {
    let result =
        assess_store_availability(&catalog(), &detail(), &inventory(), "future_vendor_app")
            .unwrap();
    assert_eq!(result.status, AvailabilityStatus::Available);
    assert!(result.platform_available);
    assert!(!result.installed);
    assert!(result.read_only);
    assert!(!result.activation_authorized);
    assert_eq!(result.slug, "future_vendor_app");
    assert_eq!(result.repository, "future_vendor");
}

#[test]
fn conflicting_or_unavailable_supervisor_metadata_fails_closed() {
    let mut c = catalog();
    let mut d = detail();
    c.apps[0].available = false;
    assert_eq!(
        assess_store_availability(&c, &d, &inventory(), "future_vendor_app")
            .unwrap()
            .status,
        AvailabilityStatus::Unavailable
    );
    c.apps[0].available = true;
    d.available = Some(false);
    assert_eq!(
        assess_store_availability(&c, &d, &inventory(), "future_vendor_app")
            .unwrap()
            .status,
        AvailabilityStatus::Unavailable
    );
    d.available = None;
    let result = assess_store_availability(&c, &d, &inventory(), "future_vendor_app").unwrap();
    assert_eq!(result.status, AvailabilityStatus::Unknown);
    assert!(!result.platform_available);
}

#[test]
fn installed_and_supervisor_managed_are_never_offered_for_installation() {
    let mut c = catalog();
    let mut d = detail();
    c.apps[0].installed = true;
    assert_eq!(
        assess_store_availability(&c, &d, &inventory(), "future_vendor_app")
            .unwrap()
            .status,
        AvailabilityStatus::AlreadyInstalled
    );
    c.apps[0].installed = false;
    d.system_managed = Some(true);
    let result = assess_store_availability(&c, &d, &inventory(), "future_vendor_app").unwrap();
    assert_eq!(result.status, AvailabilityStatus::SystemManaged);
    assert!(!result.platform_available);
}

#[test]
fn missing_slug_mismatched_repository_or_untrusted_owner_is_not_a_preflight() {
    let c = catalog();
    let mut d = detail();
    assert_eq!(
        assess_store_availability(&c, &d, &inventory(), "not_in_store"),
        Err(AvailabilityError::NotInCatalog)
    );
    d.repository = Some("another_vendor".into());
    assert_eq!(
        assess_store_availability(&c, &d, &inventory(), "future_vendor_app"),
        Err(AvailabilityError::InconsistentDetail)
    );
    d.repository = None;
    assert_eq!(
        assess_store_availability(&c, &d, &inventory(), "future_vendor_app"),
        Err(AvailabilityError::InconsistentDetail)
    );
    d = detail();
    d.slug = "other_app".into();
    assert_eq!(
        assess_store_availability(&c, &d, &inventory(), "future_vendor_app"),
        Err(AvailabilityError::InconsistentDetail)
    );
    d = detail();
    d.official_supervisor_owner = false;
    assert_eq!(
        assess_store_availability(&c, &d, &inventory(), "future_vendor_app"),
        Err(AvailabilityError::UntrustedSource)
    );
}

#[test]
fn result_contains_no_raw_supervisor_fields_or_secret_material() {
    let report =
        assess_store_availability(&catalog(), &detail(), &inventory(), "future_vendor_app")
            .unwrap();
    let json = serde_json::to_string(&report).unwrap();
    for forbidden in [
        "options",
        "token",
        "password",
        "host_network",
        "device",
        "SECRET",
    ] {
        assert!(!json.contains(forbidden), "unexpected field: {forbidden}");
    }
    assert!(json.contains("\"activation_authorized\":false"));
}

#[test]
fn installed_inventory_prevents_false_install_availability_after_catalog_race() {
    let mut inv = inventory();
    inv.apps.push(RunningApp {
        slug: "future_vendor_app".into(),
        name: "Any App".into(),
        repository: Some("future_vendor".into()),
        state: Some("started".into()),
        version: Some("1.0.0".into()),
        update_available: Some(false),
    });
    let report =
        assess_store_availability(&catalog(), &detail(), &inv, "future_vendor_app").unwrap();
    assert_eq!(report.status, AvailabilityStatus::AlreadyInstalled);
    assert!(report.installed);
    assert!(!report.platform_available);
}

#[test]
fn wrong_inventory_version_repository_or_trust_fails_closed() {
    let c = catalog();
    let d = detail();
    let mut inv = inventory();
    inv.api_version = ApiVersion::V2;
    assert_eq!(
        assess_store_availability(&c, &d, &inv, "future_vendor_app"),
        Err(AvailabilityError::InconsistentDetail)
    );
    inv.api_version = ApiVersion::V1;
    inv.read_only = false;
    assert_eq!(
        assess_store_availability(&c, &d, &inv, "future_vendor_app"),
        Err(AvailabilityError::UntrustedSource)
    );
    inv.read_only = true;
    inv.apps.push(RunningApp {
        slug: "future_vendor_app".into(),
        name: "Any App".into(),
        repository: Some("another_vendor".into()),
        state: None,
        version: None,
        update_available: None,
    });
    assert_eq!(
        assess_store_availability(&c, &d, &inv, "future_vendor_app"),
        Err(AvailabilityError::InconsistentDetail)
    );
}
