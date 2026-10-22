//! Preview-only generic official Supervisor App operation plans.
//! This module does NOT itself authenticate an end user or connect to hosts.
//! It is compiled OUT by default. The Supervisor retains all lifecycle logic.
//!
//! A real product must supply audited user RBAC, explicit approval, the
//! installed-App preflight, Job polling, backup verification and rollback.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManagerError {
    UnsafeAppSlug,
    UnsafeBackupSlug,
    NotAuthorized,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppSlug(String);
impl AppSlug {
    pub fn parse(raw: &str) -> Result<Self, ManagerError> {
        if raw.is_empty()
            || raw.len() > 80
            || !raw.starts_with(|c: char| c.is_ascii_lowercase() || c.is_ascii_digit())
            || !raw
                .bytes()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_')
            || raw == "local_hars_rust_probe"
        {
            return Err(ManagerError::UnsafeAppSlug);
        }
        Ok(Self(raw.to_owned()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupSlug(String);
impl BackupSlug {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn parse(raw: &str) -> Result<Self, ManagerError> {
        if raw.len() < 8
            || raw.len() > 64
            || !raw
                .bytes()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        {
            return Err(ManagerError::UnsafeBackupSlug);
        }
        Ok(Self(raw.to_owned()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operation {
    Install,
    Start,
    Stop,
    Restart,
    UpdateWithBackup,
    BackupPartial,
    RestorePartial { backup: BackupSlug },
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Install,
    Start,
    Stop,
    Restart,
    Update,
    Backup,
    Restore,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedRequest {
    path: String,
    body: Vec<u8>,
    human_target: String,
    action: Action,
    restore_backup: Option<BackupSlug>,
}

impl PreparedRequest {
    pub fn method(&self) -> &'static str {
        "POST"
    }
    pub fn action(&self) -> Action {
        self.action
    }
    /// Exact validated backup identifier, only for a targeted restore.
    pub fn restore_backup_slug(&self) -> Option<&str> {
        self.restore_backup.as_ref().map(BackupSlug::as_str)
    }
    pub fn path(&self) -> &str {
        &self.path
    }
    pub fn body(&self) -> &[u8] {
        &self.body
    }
    pub fn target(&self) -> &str {
        &self.human_target
    }
    pub fn requires_explicit_authorization(&self) -> bool {
        true
    }
    /// This is a pure validation predicate, NOT a user-auth implementation.
    /// The calling application must verify identity/role before passing true.
    pub fn assert_authorized(&self, verified_operator: bool) -> Result<(), ManagerError> {
        if verified_operator {
            Ok(())
        } else {
            Err(ManagerError::NotAuthorized)
        }
    }
}

pub fn prepare(app: &AppSlug, operation: Operation) -> Result<PreparedRequest, ManagerError> {
    let slug = app.as_str();
    let action = match &operation {
        Operation::Install => Action::Install,
        Operation::Start => Action::Start,
        Operation::Stop => Action::Stop,
        Operation::Restart => Action::Restart,
        Operation::UpdateWithBackup => Action::Update,
        Operation::BackupPartial => Action::Backup,
        Operation::RestorePartial { .. } => Action::Restore,
    };
    let restore_backup = match &operation {
        Operation::RestorePartial { backup } => Some(backup.clone()),
        _ => None,
    };
    let (path, body): (String, Vec<u8>) = match operation {
        Operation::Install => (format!("/store/addons/{slug}/install"), b"{}".to_vec()),
        Operation::Start => (format!("/addons/{slug}/start"), Vec::new()),
        Operation::Stop => (format!("/addons/{slug}/stop"), Vec::new()),
        Operation::Restart => (format!("/addons/{slug}/restart"), Vec::new()),
        // Supervisor's official store update automatically takes a backup.
        // We never expose a flag to suppress the backup.
        Operation::UpdateWithBackup => (
            format!("/store/addons/{slug}/update"),
            br#"{"backup":true}"#.to_vec(),
        ),
        Operation::BackupPartial => (
            "/backups/new/partial".to_string(),
            serde_json::json!({
                "addons": [slug],
                "homeassistant": false,
                "folders": [],
                "name": format!("HA Rust - {slug}"),
            })
            .to_string()
            .into_bytes(),
        ),
        Operation::RestorePartial { backup } => (
            format!("/backups/{}/restore/partial", backup.0),
            serde_json::json!({
                "addons": [slug],
                "homeassistant": false,
                "folders": [],
            })
            .to_string()
            .into_bytes(),
        ),
    };
    Ok(PreparedRequest {
        path,
        body,
        human_target: slug.to_string(),
        action,
        restore_backup,
    })
}
