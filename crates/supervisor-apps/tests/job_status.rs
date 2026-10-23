use hars_supervisor_apps::{
    job_status::{JobError, State, job_path, parse_job_response},
    official_store::ApiVersion,
};
use serde_json::json;
const ID: &str = "1234567890abcdef1234567890abcdef";
fn job(progress: u32, done: bool, errors: bool) -> Vec<u8> {
    let errors = if errors {
        json!([{"message":"PRIVATE"}])
    } else {
        json!([])
    };
    serde_json::to_vec(&json!({"result":"ok","data":{
        "uuid":ID,"progress":progress,"done":done,"errors":errors,
        "child_jobs":[],"stage":"download","extra":{"token":"HIDDEN"}
    }}))
    .unwrap()
}
#[test]
fn progress_does_not_fake_success_and_errors_are_redacted() {
    assert_eq!(
        parse_job_response(&job(100, false, false), ID)
            .unwrap()
            .state,
        State::Running
    );
    assert_eq!(
        parse_job_response(&job(100, true, false), ID)
            .unwrap()
            .state,
        State::Succeeded
    );
    let failed = parse_job_response(&job(100, true, true), ID).unwrap();
    assert_eq!(failed.state, State::Failed);
    assert!(!serde_json::to_string(&failed).unwrap().contains("PRIVATE"));
    assert!(!serde_json::to_string(&failed).unwrap().contains("HIDDEN"));
}
#[test]
fn all_job_ids_are_validated_and_version_paths_are_official() {
    assert_eq!(job_path(ApiVersion::V1, ID).unwrap(), format!("/jobs/{ID}"));
    assert_eq!(
        job_path(ApiVersion::V2, ID).unwrap(),
        format!("/v2/jobs/{ID}")
    );
    for bad in ["", "../../evil", "123", "0123456789ABCDEF0123456789ABCDEF"] {
        assert_eq!(job_path(ApiVersion::V2, bad), Err(JobError::UnsafeId));
    }
    assert_eq!(
        parse_job_response(&job(101, true, false), ID),
        Err(JobError::InvalidResponse)
    );
}
