use hars_supervisor_apps::job_status::{State, jobs_info_path, parse_jobs_info_response};
use hars_supervisor_apps::official_store::ApiVersion;
use hars_supervisor_apps::official_transport::{LocalSupervisorClient, SupervisorToken};
use serde_json::json;
use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
};

fn job(id: &str) -> serde_json::Value {
    json!({
        "uuid":id,"progress":25,"done":false,"stage":"token-secret-stage",
        "errors":[],"child_jobs":[],"extra":{"password":"never-expose"},
        "reference":"private-host-name","name":"private-operation"
    })
}
fn response(jobs: Vec<serde_json::Value>) -> Vec<u8> {
    json!({"result":"ok","data":{"jobs":jobs,"ignore_conditions":["private-condition"]}})
        .to_string()
        .into_bytes()
}
#[test]
fn official_v1_v2_paths_are_constant() {
    assert_eq!(jobs_info_path(ApiVersion::V1), "/jobs/info");
    assert_eq!(jobs_info_path(ApiVersion::V2), "/v2/jobs/info");
}
#[test]
fn discovery_sanitizes_every_private_job_field() {
    let a = "0123456789abcdef0123456789abcdef";
    let b = "abcdef0123456789abcdef0123456789";
    let result = parse_jobs_info_response(&response(vec![job(a), job(b)])).unwrap();
    assert_eq!(result.jobs.len(), 2);
    assert_eq!(result.jobs[0].id, a);
    assert_eq!(result.jobs[0].state, State::Running);
    assert_eq!(result.jobs[0].progress, 25.0);
    assert!(!result.truncated);
    let public = serde_json::to_string(&result).unwrap();
    for secret in [
        "token-secret-stage",
        "never-expose",
        "private-host-name",
        "private-condition",
        "private-operation",
        "password",
        "reference",
    ] {
        assert!(!public.contains(secret), "{secret} leaked");
    }
}
#[test]
fn rejects_duplicate_unsafe_and_inconsistent_jobs() {
    let a = "0123456789abcdef0123456789abcdef";
    for jobs in [
        vec![job(a), job(a)],
        vec![job("BAD")],
        vec![job("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA")],
        vec![json!({"uuid":a,"progress":120,"done":false,"errors":[],"child_jobs":[]})],
        vec![json!({"uuid":a,"progress":0,"done":"false","errors":[],"child_jobs":[]})],
        vec![json!({"uuid":a,"progress":0,"done":false,"errors":[]})],
    ] {
        assert!(parse_jobs_info_response(&response(jobs)).is_err());
    }
    for bad in [
        br#"{"result":"ok","data":{"jobs":[],"jobs":[]}}"#.as_slice(),
        br#"{"result":"error","data":{"jobs":[]}}"#,
        br#"{"result":"ok","data":{"jobs":"not-array"}}"#,
        br#"{"result":"ok","data":{}}"#,
        br#"{"result":"ok","data":{"jobs":[{"uuid":"0123456789abcdef0123456789abcdef","progress":0,"done":true,"errors":[],"child_jobs":[],"uuid":"0123456789abcdef0123456789abcdef"}]}}"#,
    ] {
        assert!(parse_jobs_info_response(bad).is_err());
    }
}
#[test]
fn caps_visible_jobs_but_validates_full_response() {
    let jobs = (0..30).map(|i| job(&format!("{i:032x}"))).collect();
    let parsed = parse_jobs_info_response(&response(jobs)).unwrap();
    assert_eq!(parsed.jobs.len(), 24);
    assert!(parsed.truncated);
    let huge = vec![b' '; 1024 * 1024 + 1];
    assert!(parse_jobs_info_response(&huge).is_err());
    let jobs = (0..513).map(|i| job(&format!("{i:032x}"))).collect();
    assert!(parse_jobs_info_response(&response(jobs)).is_err());
}
#[test]
fn official_http_get_list_is_authenticated_and_read_only() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let worker = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buf = [0u8; 2048];
        let size = stream.read(&mut buf).unwrap();
        let request = String::from_utf8_lossy(&buf[..size]).into_owned();
        let body = response(vec![job("0123456789abcdef0123456789abcdef")]);
        let head = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        stream.write_all(head.as_bytes()).unwrap();
        stream.write_all(&body).unwrap();
        request
    });
    let client = LocalSupervisorClient::new(
        addr,
        SupervisorToken::new("fixture-only-token".into()).unwrap(),
    )
    .unwrap();
    let result = client.fetch_jobs(ApiVersion::V1).unwrap();
    assert_eq!(result.jobs.len(), 1);
    let request = worker.join().unwrap();
    assert!(request.starts_with("GET /jobs/info HTTP/1.1\r\n"));
    assert!(request.contains("Authorization: Bearer fixture-only-token\r\n"));
    assert!(!request.contains("POST "));
}
