//! Reconcile *actual* Supervisor job completion with observed App state.
//! This does not submit operations or grant permissions. Any valid App vendor
//! is supported. Neither an HTTP 200 nor 100% job progress proves success.
use crate::installed_inventory::{RunningApp, RunningInventory};
use crate::job_status::{JobStatus, State, job_path};
use crate::official_store::ApiVersion;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    Install,
    Start,
    Stop,
    Restart,
    Update,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Watch {
    slug: String,
    job_id: String,
    operation: Operation,
    old_version: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchError {
    InvalidApp,
    InvalidJob,
    MissingPriorVersion,
}

impl Watch {
    pub fn new(
        slug: &str,
        job_id: &str,
        operation: Operation,
        old_version: Option<&str>,
    ) -> Result<Self, WatchError> {
        if slug.is_empty()
            || slug.len() > 120
            || !slug
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-'))
        {
            return Err(WatchError::InvalidApp);
        }
        if job_path(ApiVersion::V1, job_id).is_err() {
            return Err(WatchError::InvalidJob);
        }
        if operation == Operation::Update
            && !old_version.is_some_and(|v| {
                !v.is_empty() && v.len() <= 128 && !v.chars().any(char::is_control)
            })
        {
            return Err(WatchError::MissingPriorVersion);
        }
        Ok(Self {
            slug: slug.into(),
            job_id: job_id.into(),
            operation,
            old_version: old_version.map(str::to_owned),
        })
    }
    pub fn app_slug(&self) -> &str {
        &self.slug
    }
    pub fn job_id(&self) -> &str {
        &self.job_id
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    NoEvidence,
    Queued,
    Running,
    Failed,
    AwaitingState,
    Confirmed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Observation {
    pub slug: String,
    pub operation: Operation,
    pub outcome: Outcome,
    /// Only a correlated official Supervisor job + postcondition may confirm.
    pub completion_verified: bool,
}

fn matches_postcondition(w: &Watch, app: Option<&RunningApp>) -> bool {
    match w.operation {
        Operation::Install => app.is_some(),
        Operation::Start | Operation::Restart => {
            app.is_some_and(|a| a.state.as_deref() == Some("started"))
        }
        Operation::Stop => app.is_some_and(|a| a.state.as_deref() == Some("stopped")),
        Operation::Update => app.is_some_and(|a| {
            a.version
                .as_deref()
                .is_some_and(|v| Some(v) != w.old_version.as_deref())
                && a.update_available == Some(false)
        }),
    }
}

/// The caller supplies authenticated, complete responses. A failed read
/// must be passed as None, NOT an empty inventory. No confidence is invented.
pub fn reconcile(
    w: &Watch,
    job: Option<&JobStatus>,
    inventory: Option<&RunningInventory>,
) -> Observation {
    let status = match job {
        None => Outcome::NoEvidence,
        Some(j) if j.id != w.job_id => Outcome::NoEvidence,
        Some(j) => match j.state {
            State::Queued => Outcome::Queued,
            State::Running => Outcome::Running,
            State::Failed => Outcome::Failed,
            State::Succeeded => match inventory {
                None => Outcome::AwaitingState,
                Some(snapshot) => {
                    let entry = snapshot.apps.iter().find(|a| a.slug == w.slug);
                    if matches_postcondition(w, entry) {
                        Outcome::Confirmed
                    } else {
                        Outcome::AwaitingState
                    }
                }
            },
        },
    };
    Observation {
        slug: w.slug.clone(),
        operation: w.operation,
        outcome: status,
        completion_verified: status == Outcome::Confirmed,
    }
}
