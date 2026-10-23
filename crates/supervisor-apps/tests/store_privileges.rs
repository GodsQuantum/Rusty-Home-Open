//! Real loopback request contract for universal third-party security reviews.
use hars_supervisor_apps::{
    official_store::ApiVersion,
    official_transport::{LocalSupervisorClient, SupervisorToken, TransportError},
};
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpListener},
    thread,
};
fn response(slug: &str) -> String {
    format!(
        r#"{{"result":"ok","data":{{"slug":"{slug}","rating":2,"full_access":true,"docker_api":true,"hassio_role":"admin","apparmor":"disable","options":{{"password":"INTERNAL_SECRET_DO_NOT_PRINT"}}}}}}"#
    )
}
fn serve(body: String, status: &str) -> (SocketAddr, thread::JoinHandle<String>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let addr = listener.local_addr().unwrap();
    let status = status.to_owned();
    let worker = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut received = Vec::new();
        let mut buf = [0u8; 512];
        while !received.windows(4).any(|x| x == b"\r\n\r\n") {
            let n = stream.read(&mut buf).unwrap();
            assert!(n > 0 && received.len() < 8192);
            received.extend_from_slice(&buf[..n]);
        }
        stream.write_all(format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).unwrap();
        String::from_utf8(received).unwrap()
    });
    (addr, worker)
}
fn client(addr: SocketAddr) -> LocalSupervisorClient {
    LocalSupervisorClient::new(
        addr,
        SupervisorToken::new("synthetic-privileged-token".into()).unwrap(),
    )
    .unwrap()
}
#[test]
fn unknown_vendor_v1_v2_auth_and_secret_stripping_are_genuine_http_gets() {
    let slug = "future_vendor_923";
    for (api, path) in [
        (ApiVersion::V1, "/store/addons/future_vendor_923"),
        (ApiVersion::V2, "/v2/store/apps/future_vendor_923"),
    ] {
        let (addr, worker) = serve(response(slug), "200 OK");
        let found = client(addr).fetch_store_privileges(api, slug).unwrap();
        assert_eq!(found.slug, slug);
        assert!(found.read_only && !found.activation_authorized);
        assert!(found.granted.contains(&"docker_api".to_owned()));
        let public = serde_json::to_string(&found).unwrap();
        assert!(!public.contains("INTERNAL_SECRET_DO_NOT_PRINT"));
        let request = worker.join().unwrap();
        assert!(request.starts_with(&format!("GET {path} HTTP/1.1\r\n")));
        assert!(request.contains("Authorization: Bearer synthetic-privileged-token\r\n"));
        assert!(!request.contains("POST "));
    }
}
#[test]
fn wrong_vendor_auth_and_malformed_responses_deny_review() {
    let disconnected = client("127.0.0.1:1".parse().unwrap());
    assert_eq!(
        disconnected.fetch_store_privileges(ApiVersion::V1, "../vendor"),
        Err(TransportError::InvalidAppSlug)
    );
    for (status, payload, expected) in [
        (
            "401 Unauthorized",
            response("vendor"),
            TransportError::Unauthorized,
        ),
        (
            "302 Found",
            response("vendor"),
            TransportError::UnexpectedStatus,
        ),
        ("200 OK", response("other"), TransportError::InvalidAppInfo),
        (
            "200 OK",
            r#"{"result":"ok","data":{"slug":"vendor","docker_api":false,"docker_api":true}}"#
                .into(),
            TransportError::InvalidAppInfo,
        ),
    ] {
        let (addr, worker) = serve(payload, status);
        assert_eq!(
            client(addr).fetch_store_privileges(ApiVersion::V1, "vendor"),
            Err(expected)
        );
        worker.join().unwrap();
    }
}
