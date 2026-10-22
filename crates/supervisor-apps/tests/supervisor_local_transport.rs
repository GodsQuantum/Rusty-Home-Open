use hars_supervisor_apps::official_store::ApiVersion;
use hars_supervisor_apps::official_transport::{
    LocalSupervisorClient, SupervisorToken, TransportError,
};
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpListener},
    thread,
};
fn payload(field: &str) -> String {
    format!(
        r#"{{"result":"ok","data":{{"{field}":[{{"slug":"future_a","repository":"other","name":"An unknown app","available":true,"installed":false,"update_available":false}}],"repositories":[{{"slug":"other","name":"Other repository"}}]}}}}"#
    )
}
fn serve(response: String) -> (SocketAddr, thread::JoinHandle<String>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let addr = listener.local_addr().unwrap();
    let worker = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut bytes = Vec::new();
        let mut buf = [0u8; 512];
        while !bytes.windows(4).any(|w| w == b"\r\n\r\n") {
            let n = stream.read(&mut buf).unwrap();
            assert!(n > 0 && bytes.len() < 16384);
            bytes.extend_from_slice(&buf[..n]);
        }
        stream.write_all(response.as_bytes()).unwrap();
        String::from_utf8(bytes).unwrap()
    });
    (addr, worker)
}
fn ok(content: &str) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{content}",
        content.len()
    )
}
fn client(addr: SocketAddr) -> LocalSupervisorClient {
    LocalSupervisorClient::new(
        addr,
        SupervisorToken::new("fixture_value_123".to_owned()).unwrap(),
    )
    .unwrap()
}
#[test]
fn read_only_v1_store_authenticates_over_actual_loopback() {
    let (addr, worker) = serve(ok(&payload("addons")));
    let result = client(addr).fetch_store(ApiVersion::V1).unwrap();
    assert_eq!(result.apps[0].slug, "future_a");
    assert!(!result.activation_authorized);
    let request = worker.join().unwrap();
    assert!(request.starts_with("GET /store HTTP/1.1\r\n"));
    assert!(request.contains("Authorization: Bearer fixture_value_123\r\n"));
}
#[test]
fn explicitly_versioned_v2_store() {
    let (addr, worker) = serve(ok(&payload("apps")));
    let result = client(addr).fetch_store(ApiVersion::V2).unwrap();
    assert_eq!(result.api_version, ApiVersion::V2);
    assert!(
        worker
            .join()
            .unwrap()
            .starts_with("GET /v2/store HTTP/1.1\r\n")
    );
}
#[test]
fn refuses_non_loopback_and_invalid_credentials() {
    assert!(matches!(
        LocalSupervisorClient::new(
            "192.0.2.15:8123".parse().unwrap(),
            SupervisorToken::new("a".to_owned()).unwrap()
        ),
        Err(TransportError::UnsupportedHost)
    ));
    for value in ["", "not safe", "x\nSecond: line"] {
        assert!(SupervisorToken::new(value.to_owned()).is_err());
    }
    assert!(
        !format!(
            "{:?}",
            SupervisorToken::new("credential_placeholder".to_owned()).unwrap()
        )
        .contains("placeholder")
    );
}
#[test]
fn rejects_status_and_schema_mismatches() {
    for (status, expect) in [
        ("401 Unauthorized", TransportError::Unauthorized),
        ("302 Found", TransportError::UnexpectedStatus),
    ] {
        let body = payload("addons");
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let (addr, worker) = serve(response);
        assert!(matches!(client(addr).fetch_store(ApiVersion::V1),Err(e) if e==expect));
        worker.join().unwrap();
    }
    let (addr, worker) = serve(ok(&payload("apps")));
    assert!(matches!(
        client(addr).fetch_store(ApiVersion::V1),
        Err(TransportError::UnexpectedApiVersion)
    ));
    worker.join().unwrap();
}
#[test]
fn refuses_invalid_framing_and_oversized_responses() {
    for response in [
        "HTTP/1.1 200 OK\r\nContent-Length: 8388609\r\n\r\n".to_string(),
        "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n".to_string(),
        "HTTP/1.1 200 OK\r\nContent-Length: 8\r\nContent-Length: 8\r\n\r\n12345678".to_string(),
        "HTTP/1.1 200 OK\r\nContent-Length: 99\r\nConnection: close\r\n\r\n{}".to_string(),
    ] {
        let (addr, worker) = serve(response);
        assert!(client(addr).fetch_store(ApiVersion::V1).is_err());
        worker.join().unwrap();
    }
}
