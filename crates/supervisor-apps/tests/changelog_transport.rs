use hars_supervisor_apps::changelog::{changelog_path, parse_changelog};
use hars_supervisor_apps::official_store::ApiVersion;
use hars_supervisor_apps::official_transport::{
    LocalSupervisorClient, SupervisorToken, TransportError,
};
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpListener},
    thread,
};
fn server(response: String) -> (SocketAddr, thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let worker = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = Vec::new();
        let mut buf = [0; 1024];
        while !request.windows(4).any(|w| w == b"\r\n\r\n") {
            let count = stream.read(&mut buf).unwrap();
            assert!(count > 0 && request.len() < 8192);
            request.extend_from_slice(&buf[..count]);
        }
        stream.write_all(response.as_bytes()).unwrap();
        String::from_utf8(request).unwrap()
    });
    (addr, worker)
}
fn response(status: &str, mime: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {status}\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}
fn client(addr: SocketAddr) -> LocalSupervisorClient {
    LocalSupervisorClient::new(addr, SupervisorToken::new("fixture_only".into()).unwrap()).unwrap()
}
#[test]
fn official_v1_and_v2_changelog_read_only_authenticated_tcp() {
    for (version, expected) in [
        (ApiVersion::V1, "/addons/unknown_vendor_app/changelog"),
        (
            ApiVersion::V2,
            "/v2/store/apps/unknown_vendor_app/changelog",
        ),
    ] {
        let (addr, worker) = server(response(
            "200 OK",
            "text/plain; charset=utf-8",
            "## New release\nFixes and improvements.",
        ));
        let result = client(addr)
            .fetch_app_changelog(version, "unknown_vendor_app")
            .unwrap();
        assert_eq!(result.slug, "unknown_vendor_app");
        assert!(result.read_only);
        assert_eq!(result.text, "## New release\nFixes and improvements.");
        let request = worker.join().unwrap();
        assert!(request.starts_with(&format!("GET {expected} HTTP/1.1\r\n")));
    }
}
#[test]
fn changelog_rejects_traversal_controls_oversize_and_invalid_utf8() {
    for slug in ["", "../a", "%2e%2e", "a/b", "a?token=1", "a#b"] {
        assert!(changelog_path(ApiVersion::V1, slug).is_err(), "{slug}");
    }
    assert!(parse_changelog(&[0xff], "ok").is_err());
    assert!(parse_changelog(b"abc\0def", "ok").is_err());
    assert!(parse_changelog(&vec![b'a'; 65537], "ok").is_err());
}
#[test]
fn changelog_refuses_unauthorized_redirect_wrong_type_and_missing_length() {
    for (status, mime) in [
        ("403 Forbidden", "text/plain"),
        ("302 Found", "text/plain"),
        ("200 OK", "text/html"),
    ] {
        let (addr, worker) = server(response(status, mime, "unsafe"));
        assert!(
            client(addr)
                .fetch_app_changelog(ApiVersion::V1, "arbitrary")
                .is_err()
        );
        worker.join().unwrap();
    }
    let (addr, worker) = server("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\nabc".into());
    assert!(matches!(
        client(addr).fetch_app_changelog(ApiVersion::V1, "arbitrary"),
        Err(TransportError::BadResponse)
    ));
    worker.join().unwrap();
}
