//! Genuine loopback HTTP for universal Supervisor Store docs, no App mutation.
use hars_supervisor_apps::{
    changelog::{parse_changelog, store_documentation_path},
    official_store::ApiVersion,
    official_transport::{LocalSupervisorClient, SupervisorToken, TransportError},
};
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpListener},
    thread,
};

fn serve(body: String, status: &str, mime: &str) -> (SocketAddr, thread::JoinHandle<String>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let addr = listener.local_addr().unwrap();
    let status = status.to_owned();
    let mime = mime.to_owned();
    let worker = thread::spawn(move || {
        let (mut conn, _) = listener.accept().unwrap();
        let mut request = Vec::new();
        let mut buf = [0u8; 512];
        while !request.windows(4).any(|s| s == b"\r\n\r\n") {
            let n = conn.read(&mut buf).unwrap();
            assert!(n > 0 && request.len() < 16384);
            request.extend_from_slice(&buf[..n]);
        }
        conn.write_all(format!("HTTP/1.1 {status}\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).unwrap();
        String::from_utf8(request).unwrap()
    });
    (addr, worker)
}
fn client(addr: SocketAddr) -> LocalSupervisorClient {
    LocalSupervisorClient::new(
        addr,
        SupervisorToken::new("synthetic-only-bearer".into()).unwrap(),
    )
    .unwrap()
}
#[test]
fn third_party_store_documentation_is_real_bounded_get_on_v1_and_v2() {
    for (version, path) in [
        (
            ApiVersion::V1,
            "/store/addons/future_unknown_137/documentation",
        ),
        (
            ApiVersion::V2,
            "/v2/store/apps/future_unknown_137/documentation",
        ),
    ] {
        let docs = "# Universal New Vendor\n<img src=x onerror=alert(1)>\n**README**";
        let (addr, worker) = serve(docs.into(), "200 OK", "text/plain; charset=utf-8");
        let parsed = client(addr)
            .fetch_store_documentation(version, "future_unknown_137")
            .unwrap();
        assert!(parsed.read_only);
        assert_eq!(parsed.slug, "future_unknown_137");
        assert_eq!(parsed.text, docs);
        let req = worker.join().unwrap();
        assert!(req.starts_with(&format!("GET {path} HTTP/1.1\r\n")));
        assert!(req.contains("Authorization: Bearer synthetic-only-bearer\r\n"));
        assert!(!req.contains("POST "));
        assert!(!req.contains("DOCS.md"));
    }
}
#[test]
fn store_documentation_rejects_mutable_urls_mismatched_media_and_invalid_payloads() {
    for bad in [
        "",
        "../secret",
        "foo/bar",
        "foo%2fsecret",
        "foo?token=1",
        "abc\nHost: wrong",
    ] {
        assert!(store_documentation_path(ApiVersion::V1, bad).is_err());
        assert!(store_documentation_path(ApiVersion::V2, bad).is_err());
    }
    assert!(parse_changelog(&vec![b'x'; 65537], "future_unknown_137").is_err());
    assert!(parse_changelog(&[0xff], "future_unknown_137").is_err());
    assert!(parse_changelog(b"text\0unsafe", "future_unknown_137").is_err());
    let disconnected = client("127.0.0.1:1".parse().unwrap());
    assert_eq!(
        disconnected.fetch_store_documentation(ApiVersion::V1, "../secret"),
        Err(TransportError::InvalidAppSlug)
    );
    for (status, mime, expected) in [
        (
            "401 Unauthorized",
            "text/plain",
            TransportError::Unauthorized,
        ),
        ("403 Forbidden", "text/plain", TransportError::Unauthorized),
        ("302 Found", "text/plain", TransportError::UnexpectedStatus),
        ("200 OK", "text/html", TransportError::UnexpectedStatus),
    ] {
        let (addr, worker) = serve("Hello".into(), status, mime);
        let result = client(addr).fetch_store_documentation(ApiVersion::V1, "future_unknown_137");
        if mime == "text/html" {
            assert!(result.is_err());
        } else {
            assert_eq!(result, Err(expected));
        }
        worker.join().unwrap();
    }
}
