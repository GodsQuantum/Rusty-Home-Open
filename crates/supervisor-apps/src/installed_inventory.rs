//! Read-only installed App inventory from official Supervisor /addons or /v2/apps.
//! Every vendor is accepted; no installed-App allowlist.
use crate::official_store::ApiVersion;
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct RunningApp {
    pub slug: String,
    pub name: String,
    pub repository: Option<String>,
    pub state: Option<String>,
    pub version: Option<String>,
    pub update_available: Option<bool>,
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct RunningInventory {
    pub api_version: ApiVersion,
    pub apps: Vec<RunningApp>,
    pub read_only: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InventoryError {
    TooLarge,
    Invalid,
    WrongApiVersion,
    Duplicate,
}
fn field(value: &Value, name: &str, max: usize) -> Result<Option<String>, InventoryError> {
    match value.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(text))
            if !text.is_empty() && text.len() <= max && !text.chars().any(char::is_control) =>
        {
            Ok(Some(text.clone()))
        }
        _ => Err(InventoryError::Invalid),
    }
}
pub fn parse_running_inventory(
    bytes: &[u8],
    api_version: ApiVersion,
) -> Result<RunningInventory, InventoryError> {
    if bytes.len() > 8 * 1024 * 1024 {
        return Err(InventoryError::TooLarge);
    }
    let root = serde_json::from_slice::<crate::NoDuplicates>(bytes)
        .map_err(|_| InventoryError::Invalid)?
        .0;
    if root.get("result").and_then(Value::as_str) != Some("ok") {
        return Err(InventoryError::Invalid);
    }
    let data = root
        .get("data")
        .and_then(Value::as_object)
        .ok_or(InventoryError::Invalid)?;
    let key = match api_version {
        ApiVersion::V1 => "addons",
        ApiVersion::V2 => "apps",
    };
    let other = if key == "addons" { "apps" } else { "addons" };
    if data.contains_key(other) {
        return Err(InventoryError::WrongApiVersion);
    }
    let items = data
        .get(key)
        .and_then(Value::as_array)
        .ok_or(InventoryError::WrongApiVersion)?;
    if items.len() > 50_000 {
        return Err(InventoryError::TooLarge);
    }
    let mut apps = Vec::with_capacity(items.len());
    let mut seen = HashSet::with_capacity(items.len());
    for item in items {
        let slug = field(item, "slug", 120)?.ok_or(InventoryError::Invalid)?;
        if !slug
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-'))
        {
            return Err(InventoryError::Invalid);
        }
        if !seen.insert(slug.to_ascii_lowercase()) {
            return Err(InventoryError::Duplicate);
        }
        let state = field(item, "state", 80)?;
        let update_available = match item.get("update_available") {
            None | Some(Value::Null) => None,
            Some(Value::Bool(v)) => Some(*v),
            _ => return Err(InventoryError::Invalid),
        };
        apps.push(RunningApp {
            slug,
            name: field(item, "name", 256)?.ok_or(InventoryError::Invalid)?,
            repository: field(item, "repository", 120)?,
            state,
            version: field(item, "version", 120)?,
            update_available,
        });
    }
    apps.sort_unstable_by(|a, b| a.slug.cmp(&b.slug));
    Ok(RunningInventory {
        api_version,
        apps,
        read_only: true,
    })
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeEvent {
    Installed,
    Removed,
    StateChanged,
    VersionChanged,
    UpdateChanged,
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Change {
    pub slug: String,
    pub event: RuntimeEvent,
}
pub fn diff_running(old: &RunningInventory, new: &RunningInventory) -> Vec<Change> {
    let prev: BTreeMap<&str, &RunningApp> = old.apps.iter().map(|a| (a.slug.as_str(), a)).collect();
    let curr: BTreeMap<&str, &RunningApp> = new.apps.iter().map(|a| (a.slug.as_str(), a)).collect();
    let mut changes = Vec::new();
    for (slug, app) in &curr {
        match prev.get(slug) {
            None => changes.push(Change {
                slug: (*slug).into(),
                event: RuntimeEvent::Installed,
            }),
            Some(was) => {
                for (different, event) in [
                    (was.state != app.state, RuntimeEvent::StateChanged),
                    (was.version != app.version, RuntimeEvent::VersionChanged),
                    (
                        was.update_available != app.update_available,
                        RuntimeEvent::UpdateChanged,
                    ),
                ] {
                    if different {
                        changes.push(Change {
                            slug: (*slug).into(),
                            event,
                        });
                    }
                }
            }
        }
    }
    for slug in prev.keys() {
        if !curr.contains_key(slug) {
            changes.push(Change {
                slug: (*slug).into(),
                event: RuntimeEvent::Removed,
            });
        }
    }
    changes
}
