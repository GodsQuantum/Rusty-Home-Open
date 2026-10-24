#![cfg(feature = "durable-job-watch")]
use hars_supervisor_apps::{
    durable_job_watch::{DurableJobWatches, OfficialPollError},
    official_store::ApiVersion,
    official_transport::{LocalSupervisorClient, SupervisorToken, TransportError},
    operation_reconcile::{Operation, Outcome},
};
use serde_json::{Value, json};
use std::{
    fs,
    io::{Read, Write},
    net::{SocketAddr, TcpListener},
    os::unix::fs::PermissionsExt,
    thread,
};

const JOB: &str = "0123456789abcdef0123456789abcdef";
const OTHER: &str = "ffffffffffffffffffffffffffffffff";
const APP: &str = "any_future_vendor_2027";

fn private_dir() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
    dir
}
fn response(status: &str, body: Value) -> String {
    let data = body.to_string();
    format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{data}",
        data.len()
    )
}
fn job(done: Option<bool>, uuid: &str) -> String {
    response(
        "200 OK",
        json!({"result":"ok","data":{
            "uuid":uuid,"progress":if done == Some(true) {100.0} else {60.0},
            "done":done,"errors":[],"child_jobs":[],
            "stage":"PRIVATE_DATA_NEVER_EXPORTED"
        }}),
    )
}
fn inventory(version: ApiVersion, state: &str) -> String {
    let key = match version {
        ApiVersion::V1 => "addons",
        ApiVersion::V2 => "apps",
    };
    response(
        "200 OK",
        json!({"result":"ok","data":{key:[{
            "slug":APP, "name":"Future vendor", "repository":"custom_2027",
            "state":state, "version":"1.0.0", "update_available":false,
            "options":{"password":"NOT_VISIBLE"}
        }]}}),
    )
}
fn serve(expect: Vec<(String, String)>) -> (SocketAddr, thread::JoinHandle<()>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let addr = listener.local_addr().unwrap();
    let worker = thread::spawn(move || {
        for (path, reply) in expect {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(3)))
                .unwrap();
            let mut bytes = Vec::new();
            let mut chunk = [0; 1024];
            while !bytes.windows(4).any(|w| w == b"\r\n\r\n") {
                let count = stream.read(&mut chunk).unwrap();
                assert!(count > 0 && bytes.len() < 16_384);
                bytes.extend_from_slice(&chunk[..count]);
            }
            let raw = String::from_utf8(bytes).unwrap();
            assert!(raw.starts_with(&format!("GET {path} HTTP/1.1\r\n")));
            assert!(raw.contains("Authorization: Bearer fixture_token\r\n"));
            assert!(!raw.contains("POST "));
            stream.write_all(reply.as_bytes()).unwrap();
        }
    });
    (addr, worker)
}
fn client(addr: SocketAddr) -> LocalSupervisorClient {
    LocalSupervisorClient::new(addr, SupervisorToken::new("fixture_token".into()).unwrap()).unwrap()
}
#[test]
fn authenticated_official_poll_survives_restart_and_requires_postcondition() {
    let dir = private_dir();
    let db = dir.path().join("jobs.redb");
    {
        let mut ledger = DurableJobWatches::open(&db).unwrap();
        ledger
            .register(JOB, APP, Operation::Start, None, "core_account", 100)
            .unwrap();
        let (addr, worker) = serve(vec![(format!("/jobs/{JOB}"), job(Some(false), JOB))]);
        let row = ledger
            .poll_local_supervisor(JOB, ApiVersion::V1, &client(addr), 101)
            .unwrap()
            .unwrap();
        assert_eq!(row.outcome, Some(Outcome::Running));
        assert!(!row.postcondition_observed);
        worker.join().unwrap();
    }
    {
        let mut ledger = DurableJobWatches::open(&db).unwrap();
        let (addr, worker) = serve(vec![
            (format!("/v2/jobs/{JOB}"), job(Some(true), JOB)),
            ("/v2/apps".into(), inventory(ApiVersion::V2, "stopped")),
        ]);
        let row = ledger
            .poll_local_supervisor(JOB, ApiVersion::V2, &client(addr), 102)
            .unwrap()
            .unwrap();
        assert_eq!(row.outcome, Some(Outcome::AwaitingState));
        worker.join().unwrap();
        let (addr, worker) = serve(vec![
            (format!("/jobs/{JOB}"), job(Some(true), JOB)),
            ("/addons".into(), inventory(ApiVersion::V1, "started")),
        ]);
        let row = ledger
            .poll_local_supervisor(JOB, ApiVersion::V1, &client(addr), 103)
            .unwrap()
            .unwrap();
        assert_eq!(row.outcome, Some(Outcome::Confirmed));
        assert!(row.postcondition_observed);
        worker.join().unwrap();
    }
    // A terminal result is restored without network polling or regressing.
    let mut ledger = DurableJobWatches::open(&db).unwrap();
    let disconnected = client("127.0.0.1:1".parse().unwrap());
    let row = ledger
        .poll_local_supervisor(JOB, ApiVersion::V1, &disconnected, 104)
        .unwrap()
        .unwrap();
    assert_eq!(row.outcome, Some(Outcome::Confirmed));
}

#[test]
fn bad_job_data_or_unauthorized_inventory_preserves_durable_state() {
    let dir = private_dir();
    let mut ledger = DurableJobWatches::open(&dir.path().join("jobs.redb")).unwrap();
    ledger
        .register(JOB, APP, Operation::Install, None, "core_account", 100)
        .unwrap();
    let (addr, worker) = serve(vec![(format!("/jobs/{JOB}"), job(Some(true), OTHER))]);
    assert!(matches!(
        ledger.poll_local_supervisor(JOB, ApiVersion::V1, &client(addr), 101),
        Err(OfficialPollError::Transport(TransportError::InvalidJob))
    ));
    worker.join().unwrap();
    assert_eq!(ledger.get(JOB).unwrap().unwrap().outcome, None);

    let (addr, worker) = serve(vec![
        (format!("/jobs/{JOB}"), job(Some(true), JOB)),
        (
            "/addons".into(),
            response("401 Unauthorized", json!({"result":"error"})),
        ),
    ]);
    assert!(matches!(
        ledger.poll_local_supervisor(JOB, ApiVersion::V1, &client(addr), 102),
        Err(OfficialPollError::Transport(TransportError::Unauthorized))
    ));
    worker.join().unwrap();
    assert_eq!(ledger.get(JOB).unwrap().unwrap().outcome, None);
}

#[test]
fn unknown_job_never_initiates_network_and_bad_time_fails_before_poll() {
    let dir = private_dir();
    let mut ledger = DurableJobWatches::open(&dir.path().join("jobs.redb")).unwrap();
    let disconnected = client("127.0.0.1:1".parse().unwrap());
    assert!(
        ledger
            .poll_local_supervisor(JOB, ApiVersion::V1, &disconnected, 100)
            .unwrap()
            .is_none()
    );
    ledger
        .register(JOB, APP, Operation::Start, None, "core_account", 100)
        .unwrap();
    assert!(matches!(
        ledger.poll_local_supervisor(JOB, ApiVersion::V1, &disconnected, 99),
        Err(OfficialPollError::Storage(_))
    ));
    assert_eq!(ledger.get(JOB).unwrap().unwrap().outcome, None);
}
