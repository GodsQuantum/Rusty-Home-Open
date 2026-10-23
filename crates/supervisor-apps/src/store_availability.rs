//! Read-only, fail-closed platform availability preflight for arbitrary
//! official Supervisor Store Apps. This is NOT install authorization.
use crate::app_detail::{AppDetail, valid_slug};
use crate::installed_inventory::RunningInventory;
use crate::official_store::Catalog;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AvailabilityStatus {
    Available,
    Unavailable,
    AlreadyInstalled,
    SystemManaged,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StoreAvailability {
    pub slug: String,
    pub repository: String,
    pub status: AvailabilityStatus,
    /// True only if both official live Store views explicitly report availability.
    pub platform_available: bool,
    pub installed: bool,
    pub read_only: bool,
    /// A Store preflight must NEVER grant an installation capability.
    pub activation_authorized: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AvailabilityError {
    InvalidSlug,
    NotInCatalog,
    InconsistentDetail,
    UntrustedSource,
}

/// Reconcile a freshly fetched official Store catalogue with its detail
/// endpoint. The caller must authenticate to the fixed internal Supervisor
/// and fetch both resources during this request. No snapshots or stale cache.
pub fn assess_store_availability(
    catalog: &Catalog,
    detail: &AppDetail,
    inventory: &RunningInventory,
    slug: &str,
) -> Result<StoreAvailability, AvailabilityError> {
    if !valid_slug(slug) {
        return Err(AvailabilityError::InvalidSlug);
    }
    if catalog.api_version != inventory.api_version {
        return Err(AvailabilityError::InconsistentDetail);
    }
    if !inventory.read_only
        || !catalog.official_supervisor_owner
        || catalog.activation_authorized
        || !detail.official_supervisor_owner
        || detail.activation_authorized
    {
        return Err(AvailabilityError::UntrustedSource);
    }
    let mut matches = catalog.apps.iter().filter(|app| app.slug == slug);
    let app = matches.next().ok_or(AvailabilityError::NotInCatalog)?;
    if matches.next().is_some() {
        return Err(AvailabilityError::InconsistentDetail);
    }
    if detail.slug != slug
        || detail.repository.as_deref() != Some(app.repository.as_str())
        || !catalog
            .repositories
            .iter()
            .any(|repo| repo.slug == app.repository)
    {
        return Err(AvailabilityError::InconsistentDetail);
    }
    let mut installed_matches = inventory.apps.iter().filter(|entry| entry.slug == slug);
    let installed_entry = installed_matches.next();
    if installed_matches.next().is_some()
        || installed_entry
            .and_then(|entry| entry.repository.as_deref())
            .is_some_and(|repository| repository != app.repository)
    {
        return Err(AvailabilityError::InconsistentDetail);
    }
    // Any official source seeing an installation must prevent a false
    // uninstalled/available report during a non-atomic Store refresh.
    let installed = app.installed || installed_entry.is_some();
    let status = if installed {
        AvailabilityStatus::AlreadyInstalled
    } else if detail.system_managed == Some(true) {
        AvailabilityStatus::SystemManaged
    } else if !app.available || detail.available == Some(false) {
        AvailabilityStatus::Unavailable
    } else if detail.available.is_none() {
        AvailabilityStatus::Unknown
    } else {
        AvailabilityStatus::Available
    };
    Ok(StoreAvailability {
        slug: app.slug.clone(),
        repository: app.repository.clone(),
        status,
        platform_available: status == AvailabilityStatus::Available,
        installed,
        read_only: true,
        activation_authorized: false,
    })
}
