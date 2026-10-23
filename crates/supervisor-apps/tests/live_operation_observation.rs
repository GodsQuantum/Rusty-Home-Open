use hars_supervisor_apps::{
    official_store::ApiVersion,
    official_transport::{LocalSupervisorClient, SupervisorToken, TransportError},
    operation_reconcile::{Operation, Outcome, Watch},
};
use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpListener},
    thread,
};
const JOB: &str = "0123456789abcdef0123456789abcdef";
const SLUG: &str = "new_vendor_future_42";
fn response(status: &str, body: Value) -> String {
    let body = body.to_string();
    format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}
fn job(done: Option<bool>, errors: Value, id: &str) -> String {
    response(
        "200 OK",
        json!({"result":"ok","data":{
            "uuid":id,"progress":100.0,"done":done,"errors":errors,
            "child_jobs":[],"stage":"official supervisor"
        }}),
    )
}
fn inventory(version: ApiVersion, state: &str, installed_version: &str) -> String {
    let key = match version {
        ApiVersion::V1 => "addons",
        ApiVersion::V2 => "apps",
    };
    let mut data = serde_json::Map::new();
    data.insert(
        key.into(),
        json!([{
            "slug":SLUG,"name":"Unknown Future Vendor","repository":"any_vendor",
            "state":state,"version":installed_version,"update_available":false,
            "options":{"password":"SECRET_MUST_NOT_LEAK"}
        }]),
    );
    response("200 OK", json!({"result":"ok","data":data}))
}
fn serve(responses: Vec<(String, String)>) -> (SocketAddr, thread::JoinHandle<()>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let addr = listener.local_addr().unwrap();
    let worker = thread::spawn(move || {
        for (path, response) in responses {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(3)))
                .unwrap();
            let mut bytes = Vec::new();
            let mut buf = [0u8; 1024];
            while !bytes.windows(4).any(|w| w == b"\r\n\r\n") {
                let n = stream.read(&mut buf).unwrap();
                assert!(n > 0 && bytes.len() < 16384);
                bytes.extend_from_slice(&buf[..n]);
            }
            let request = String::from_utf8(bytes).unwrap();
            assert!(request.starts_with(&format!("GET {path} HTTP/1.1\r\n")));
            assert!(request.contains("Authorization: Bearer synthetic_fixture\r\n"));
            stream.write_all(response.as_bytes()).unwrap();
        }
    });
    (addr, worker)
}
fn client(addr: SocketAddr) -> LocalSupervisorClient {
    LocalSupervisorClient::new(
        addr,
        SupervisorToken::new("synthetic_fixture".into()).unwrap(),
    )
    .unwrap()
}
fn watch(action: Operation, prior: Option<&str>) -> Watch {
    Watch::new(SLUG, JOB, action, prior).unwrap()
}
#[test]
fn successful_job_requires_separate_observed_postcondition_v1_and_v2() {
    for (v, prefix, apps, action, prior, state, version, outcome) in [
        (
            ApiVersion::V1,
            "",
            "/addons",
            Operation::Start,
            None,
            "started",
            "1",
            Outcome::Confirmed,
        ),
        (
            ApiVersion::V2,
            "/v2",
            "/v2/apps",
            Operation::Update,
            Some("1"),
            "started",
            "2",
            Outcome::Confirmed,
        ),
        (
            ApiVersion::V1,
            "",
            "/addons",
            Operation::Start,
            None,
            "stopped",
            "1",
            Outcome::AwaitingState,
        ),
    ] {
        let (addr, worker) = serve(vec![
            (
                format!("{prefix}/jobs/{JOB}"),
                job(Some(true), json!([]), JOB),
            ),
            (apps.into(), inventory(v, state, version)),
        ]);
        let observed = client(addr)
            .observe_operation(v, &watch(action, prior))
            .unwrap();
        assert_eq!(observed.outcome, outcome);
        assert_eq!(observed.completion_verified, outcome == Outcome::Confirmed);
        worker.join().unwrap();
    }
}
#[test]
fn nonterminal_or_failed_job_skips_inventory_read() {
    for (done, errors, expected) in [
        (None, json!([]), Outcome::Queued),
        (Some(false), json!([]), Outcome::Running),
        (Some(true), json!(["failure"]), Outcome::Failed),
    ] {
        let (addr, worker) = serve(vec![(format!("/jobs/{JOB}"), job(done, errors, JOB))]);
        let observed = client(addr)
            .observe_operation(ApiVersion::V1, &watch(Operation::Start, None))
            .unwrap();
        assert_eq!(observed.outcome, expected);
        assert!(!observed.completion_verified);
        worker.join().unwrap();
    }
}
#[test]
fn unauthorized_inventory_and_wrong_job_identity_fail_closed() {
    let (addr, worker) = serve(vec![
        (format!("/jobs/{JOB}"), job(Some(true), json!([]), JOB)),
        (
            "/addons".into(),
            response("401 Unauthorized", json!({"result":"error"})),
        ),
    ]);
    assert!(matches!(
        client(addr).observe_operation(ApiVersion::V1, &watch(Operation::Start, None)),
        Err(TransportError::Unauthorized)
    ));
    worker.join().unwrap();

    let (addr, worker) = serve(vec![(
        format!("/jobs/{JOB}"),
        job(Some(true), json!([]), "ffffffffffffffffffffffffffffffff"),
    )]);
    assert!(matches!(
        client(addr).observe_operation(ApiVersion::V1, &watch(Operation::Start, None)),
        Err(TransportError::InvalidJob)
    ));
    worker.join().unwrap();
}
