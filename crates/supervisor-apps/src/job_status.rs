//! Read-only status of any official Supervisor Job, not an App-specific fixture.
use crate::official_store::ApiVersion;
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobError {
    UnsafeId,
    InvalidResponse,
    TooLarge,
    MismatchedId,
}
pub fn job_path(version: ApiVersion, id: &str) -> Result<String, JobError> {
    if id.len() != 32
        || !id
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err(JobError::UnsafeId);
    }
    Ok(match version {
        ApiVersion::V1 => format!("/jobs/{id}"),
        ApiVersion::V2 => format!("/v2/jobs/{id}"),
    })
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    Queued,
    Running,
    Succeeded,
    Failed,
}
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct JobStatus {
    pub id: String,
    pub progress: f64,
    pub state: State,
    pub error_count: usize,
    pub stage: Option<String>,
    pub read_only: bool,
}
fn count_errors(job: &Value, depth: usize) -> Result<usize, JobError> {
    if depth > 8 {
        return Err(JobError::TooLarge);
    }
    let errors = job
        .get("errors")
        .and_then(Value::as_array)
        .ok_or(JobError::InvalidResponse)?;
    let children = job
        .get("child_jobs")
        .and_then(Value::as_array)
        .ok_or(JobError::InvalidResponse)?;
    if errors.len() > 64 || children.len() > 256 {
        return Err(JobError::TooLarge);
    }
    let mut total = errors.len();
    for item in children {
        total += count_errors(item, depth + 1)?;
    }
    Ok(total)
}
pub fn parse_job_response(bytes: &[u8], expected: &str) -> Result<JobStatus, JobError> {
    job_path(ApiVersion::V1, expected)?;
    if bytes.len() > 1024 * 1024 {
        return Err(JobError::TooLarge);
    }
    let reply = serde_json::from_slice::<crate::NoDuplicates>(bytes)
        .map_err(|_| JobError::InvalidResponse)?
        .0;
    if reply.get("result").and_then(Value::as_str) != Some("ok") {
        return Err(JobError::InvalidResponse);
    }
    let job = reply.get("data").ok_or(JobError::InvalidResponse)?;
    if job.get("uuid").and_then(Value::as_str) != Some(expected) {
        return Err(JobError::MismatchedId);
    }
    let progress = job
        .get("progress")
        .and_then(Value::as_f64)
        .ok_or(JobError::InvalidResponse)?;
    if !(0.0..=100.0).contains(&progress) {
        return Err(JobError::InvalidResponse);
    }
    let done = match job.get("done") {
        Some(Value::Bool(b)) => Some(*b),
        Some(Value::Null) => None,
        _ => return Err(JobError::InvalidResponse),
    };
    let error_count = count_errors(job, 0)?;
    let state = match done {
        None => State::Queued,
        Some(false) => State::Running,
        Some(true) if error_count > 0 => State::Failed,
        Some(true) => State::Succeeded,
    };
    let stage = match job.get("stage") {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) if s.len() <= 120 && !s.chars().any(char::is_control) => {
            Some(s.clone())
        }
        _ => return Err(JobError::InvalidResponse),
    };
    Ok(JobStatus {
        id: expected.into(),
        progress,
        state,
        error_count,
        stage,
        read_only: true,
    })
}
