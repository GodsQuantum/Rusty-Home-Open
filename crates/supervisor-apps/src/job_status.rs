//! Read-only status of any official Supervisor Job, not an App-specific fixture.
use crate::official_store::ApiVersion;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashSet;

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
/// The official Supervisor V1 and experimental V2 job-index routes.
pub fn jobs_info_path(version: ApiVersion) -> &'static str {
    match version {
        ApiVersion::V1 => "/jobs/info",
        ApiVersion::V2 => "/v2/jobs/info",
    }
}

/// A deliberately minimal view: never expose job names, references, stages,
/// errors, child metadata, conditions, or any vendor/private fields.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct JobSummary {
    pub id: String,
    pub progress: f64,
    pub state: State,
    pub error_count: usize,
    pub read_only: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct JobListing {
    pub jobs: Vec<JobSummary>,
    pub truncated: bool,
    pub read_only: bool,
}

const MAX_INDEX_JOBS: usize = 512;
const VISIBLE_JOBS: usize = 24;

/// Validate the ENTIRE official index before selecting its newest 24 jobs.
/// Duplicate keys/IDs, invalid UUIDs, malformed children and oversized
/// replies fail closed; no user-supplied endpoints or mutation permissions.
pub fn parse_jobs_info_response(bytes: &[u8]) -> Result<JobListing, JobError> {
    if bytes.len() > 1024 * 1024 {
        return Err(JobError::TooLarge);
    }
    let reply = serde_json::from_slice::<crate::NoDuplicates>(bytes)
        .map_err(|_| JobError::InvalidResponse)?
        .0;
    if reply.get("result").and_then(Value::as_str) != Some("ok") {
        return Err(JobError::InvalidResponse);
    }
    let jobs = reply
        .get("data")
        .and_then(|v| v.get("jobs"))
        .and_then(Value::as_array)
        .ok_or(JobError::InvalidResponse)?;
    if jobs.len() > MAX_INDEX_JOBS {
        return Err(JobError::TooLarge);
    }
    let mut seen = HashSet::with_capacity(jobs.len());
    let mut visible = Vec::with_capacity(jobs.len().min(VISIBLE_JOBS));
    for job in jobs {
        let id = job
            .get("uuid")
            .and_then(Value::as_str)
            .ok_or(JobError::InvalidResponse)?;
        job_path(ApiVersion::V1, id)?;
        if !seen.insert(id) {
            return Err(JobError::InvalidResponse);
        }
        let parsed = parse_job_data(job, id)?;
        if visible.len() < VISIBLE_JOBS {
            visible.push(JobSummary {
                id: parsed.id,
                progress: parsed.progress,
                state: parsed.state,
                error_count: parsed.error_count,
                read_only: true,
            });
        }
    }
    Ok(JobListing {
        jobs: visible,
        truncated: jobs.len() > VISIBLE_JOBS,
        read_only: true,
    })
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
    parse_job_data(job, expected)
}

fn parse_job_data(job: &Value, expected: &str) -> Result<JobStatus, JobError> {
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
