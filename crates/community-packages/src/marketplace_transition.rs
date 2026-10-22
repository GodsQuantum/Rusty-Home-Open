//! Safe, offline, *read-only* transition analysis for HACS -> Core Marketplace.
//! This is NOT a second HACS/Marketplace installer or a state migrator.
//! Real state changes and all Critical policy belong to official HA Core.
use crate::Kind;
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogueState {
    Normal,
    Removed,
    Critical,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyEntry {
    pub repository: String,
    pub kind: Kind,
    pub custom: bool,
    pub catalogue: CatalogueState,
    pub update_entity_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedCandidate {
    pub repository: String,
    pub kind: Kind,
    pub update_entity_id: String,
    pub requires_github_connection: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionPlan {
    /// Inert candidates only: no files copied, no repo downloaded.
    pub managed: Vec<ManagedCandidate>,
    /// Old AppDaemon/Python scripts and Removed repos remain intact, unmanaged.
    pub unmanaged_preserved: Vec<LegacyEntry>,
    /// Core Marketplace must decide its own Critical handling; Rust does not delete.
    pub critical_hold: Vec<LegacyEntry>,
    /// Logical contracts, NOT URLs/HTTP handlers implemented in this crate.
    pub legacy_hacs_navigation: bool,
    pub legacy_hacsfiles_resolution: bool,
    pub activation_authorized: bool,
    pub applies_changes: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionError {
    TooManyEntries,
    UnsafeRepository,
    UnsafeEntityId,
    MissingUpdateEntity,
    DuplicateRepository,
    DuplicateUpdateEntity,
    UnsupportedAppKind,
}

const MAX_ENTRIES: usize = 20_000;

fn clean_segment(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 120
        && s != "."
        && s != ".."
        && !s.starts_with('.')
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}
fn valid_repository(name: &str) -> bool {
    let mut segments = name.split('/');
    matches!((segments.next(), segments.next(), segments.next()),
        (Some(owner), Some(repo), None) if clean_segment(owner) && clean_segment(repo))
}
fn valid_update_entity(id: &str) -> bool {
    let Some(suffix) = id.strip_prefix("update.") else {
        return false;
    };
    !suffix.is_empty()
        && suffix.len() <= 200
        && !suffix.starts_with('.')
        && !suffix.ends_with('.')
        && suffix
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
}

/// No filesystem, network, token, or mutation privileges. Official Core 2026.11
/// migrates its own HACS storage and resolves the old routes.
pub fn plan_transition(entries: &[LegacyEntry]) -> Result<TransitionPlan, TransitionError> {
    if entries.len() > MAX_ENTRIES {
        return Err(TransitionError::TooManyEntries);
    }
    let mut repositories = HashSet::with_capacity(entries.len());
    let mut update_entities = HashSet::with_capacity(entries.len());
    let mut plan = TransitionPlan {
        managed: Vec::new(),
        unmanaged_preserved: Vec::new(),
        critical_hold: Vec::new(),
        legacy_hacs_navigation: true,
        legacy_hacsfiles_resolution: true,
        activation_authorized: false,
        applies_changes: false,
    };
    for entry in entries {
        if !valid_repository(&entry.repository) {
            return Err(TransitionError::UnsafeRepository);
        }
        if !repositories.insert(entry.repository.to_ascii_lowercase()) {
            return Err(TransitionError::DuplicateRepository);
        }
        if entry.kind == Kind::SupervisorApp {
            return Err(TransitionError::UnsupportedAppKind);
        }
        if let Some(update) = &entry.update_entity_id {
            if !valid_update_entity(update) {
                return Err(TransitionError::UnsafeEntityId);
            }
            if !update_entities.insert(update.to_ascii_lowercase()) {
                return Err(TransitionError::DuplicateUpdateEntity);
            }
        }
        if entry.catalogue == CatalogueState::Critical {
            plan.critical_hold.push(entry.clone());
            continue;
        }
        if entry.catalogue == CatalogueState::Removed
            || matches!(entry.kind, Kind::AppDaemon | Kind::PythonScript)
        {
            plan.unmanaged_preserved.push(entry.clone());
            continue;
        }
        let update = entry
            .update_entity_id
            .as_ref()
            .ok_or(TransitionError::MissingUpdateEntity)?;
        plan.managed.push(ManagedCandidate {
            repository: entry.repository.clone(),
            kind: entry.kind,
            update_entity_id: update.clone(),
            requires_github_connection: entry.custom,
        });
    }
    Ok(plan)
}
