//! Adversarial, loopback-only Supervisor resource telemetry contract.
use hars_supervisor_apps::app_stats::{parse_app_stats, stats_path};
use hars_supervisor_apps::official_store::ApiVersion;
use hars_supervisor_apps::official_transport::{
    LocalSupervisorClient, SupervisorToken, TransportError,
};
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpListener},
    thread,
};

fn fixture() -> &'static str {
    r#"{"result":"ok","data":{"cpu_percent":12.5,"memory_usage":4096,"memory_limit":8192,"memory_percent":50.0,"network_rx":88,"network_tx":99,"options":{"password":"DO_NOT_EXPOSE"},"token":"DO_NOT_EXPOSE"}}"#
}
fn serve(body: String, status: &str) -> (SocketAddr, thread::JoinHandle<String>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let addr = listener.local_addr().unwrap();
    let status = status.to_owned();
    let worker = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut req = Vec::new();
        let mut buf = [0u8; 512];
        while !req.windows(4).any(|w| w == b"\r\n\r\n") {
            let n = stream.read(&mut buf).unwrap();
            assert!(n > 0 && req.len() < 16384);
            req.extend_from_slice(&buf[..n]);
        }
        stream.write_all(format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        ).as_bytes()).unwrap();
        String::from_utf8(req).unwrap()
    });
    (addr, worker)
}
fn client(addr: SocketAddr) -> LocalSupervisorClient {
    LocalSupervisorClient::new(
        addr,
        SupervisorToken::new("synthetic-test-token".into()).unwrap(),
    )
    .unwrap()
}

#[test]
fn paths_are_generic_versioned_and_injection_safe() {
    assert_eq!(
        stats_path(ApiVersion::V1, "future_vendor_xyz").unwrap(),
        "/addons/future_vendor_xyz/stats"
    );
    assert_eq!(
        stats_path(ApiVersion::V2, "future_vendor_xyz").unwrap(),
        "/v2/apps/future_vendor_xyz/stats"
    );
    for slug in [
        "",
        "../other",
        "a/b",
        "x%2fsecret",
        "x?token=1",
        "a\nX: evil",
    ] {
        assert!(stats_path(ApiVersion::V1, slug).is_err());
        assert!(stats_path(ApiVersion::V2, slug).is_err());
    }
}

#[test]
fn typed_stats_drop_secrets_and_reject_untrusted_numbers() {
    let stats = parse_app_stats(fixture().as_bytes(), "future_vendor_xyz").unwrap();
    assert_eq!(stats.slug, "future_vendor_xyz");
    assert_eq!(stats.memory_usage, 4096);
    assert_eq!(stats.memory_limit, 8192);
    assert_eq!(stats.cpu_percent, Some(12.5));
    assert_eq!(stats.network_rx, Some(88));
    assert!(stats.read_only);
    let encoded = serde_json::to_string(&stats).unwrap();
    for forbidden in ["DO_NOT_EXPOSE", "password", "token", "options"] {
        assert!(!encoded.contains(forbidden));
    }
    for payload in [
        r#"{"result":"error","data":{"memory_usage":1,"memory_limit":2}}"#,
        r#"{"result":"ok","data":{"memory_usage":-1,"memory_limit":2}}"#,
        r#"{"result":"ok","data":{"memory_usage":1,"memory_usage":2,"memory_limit":3}}"#,
        r#"{"result":"ok","data":{"memory_usage":1.2,"memory_limit":2}}"#,
        r#"{"result":"ok","data":{"memory_usage":1,"memory_limit":2,"cpu_percent":-1}}"#,
        r#"{"result":"ok","data":{"memory_usage":1,"memory_limit":2,"network_tx":"not-a-number"}}"#,
    ] {
        assert!(
            parse_app_stats(payload.as_bytes(), "future_vendor_xyz").is_err(),
            "{payload}"
        );
    }
    assert!(parse_app_stats(fixture().as_bytes(), "../other").is_err());
}

#[test]
fn live_v1_and_v2_fetch_real_authenticated_get_without_mutations() {
    for (version, path) in [
        (ApiVersion::V1, "/addons/future_vendor_xyz/stats"),
        (ApiVersion::V2, "/v2/apps/future_vendor_xyz/stats"),
    ] {
        let (addr, worker) = serve(fixture().into(), "200 OK");
        let stats = client(addr)
            .fetch_app_stats(version, "future_vendor_xyz")
            .unwrap();
        assert_eq!(stats.memory_usage, 4096);
        let request = worker.join().unwrap();
        assert!(request.starts_with(&format!("GET {path} HTTP/1.1\r\n")));
        assert!(request.contains("Authorization: Bearer synthetic-test-token\r\n"));
        assert!(!request.contains("POST "));
    }
}

#[test]
fn live_stats_fail_closed_on_auth_http_and_bad_identifiers() {
    let disconnected = client("127.0.0.1:1".parse().unwrap());
    assert_eq!(
        disconnected.fetch_app_stats(ApiVersion::V1, "../secret"),
        Err(TransportError::InvalidAppSlug)
    );
    for (status, error) in [
        ("401 Unauthorized", TransportError::Unauthorized),
        ("403 Forbidden", TransportError::Unauthorized),
        ("302 Found", TransportError::UnexpectedStatus),
    ] {
        let (addr, worker) = serve(fixture().into(), status);
        assert_eq!(
            client(addr).fetch_app_stats(ApiVersion::V1, "vendor_app"),
            Err(error)
        );
        worker.join().unwrap();
    }
}
