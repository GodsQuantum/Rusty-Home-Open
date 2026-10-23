//! Real loopback GET, exact V1/V2 Supervisor App schema with secret-only fixture.
use hars_supervisor_apps::{
    official_store::ApiVersion,
    official_transport::{LocalSupervisorClient, SupervisorToken, TransportError},
};
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpListener},
    thread,
};
fn payload(slug: &str) -> String {
    format!(
        r#"{{"result":"ok","data":{{"slug":"{slug}","schema":{{"password":"password","port":"int","api_key":"str?"}},"options":{{"password":"DO_NOT_EXPOSE","port":9001,"api_key":"PRIVATE_TOKEN"}},"ingress_url":"/private"}}}}"#
    )
}
fn serve(status: &'static str, body: String) -> (SocketAddr, thread::JoinHandle<String>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let addr = listener.local_addr().unwrap();
    let worker = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buf = [0u8; 512];
        let mut request = Vec::new();
        while !request.windows(4).any(|x| x == b"\r\n\r\n") {
            let n = stream.read(&mut buf).unwrap();
            assert!(n > 0 && request.len() < 16384);
            request.extend_from_slice(&buf[..n]);
        }
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(response.as_bytes()).unwrap();
        String::from_utf8(request).unwrap()
    });
    (addr, worker)
}
fn client(addr: SocketAddr) -> LocalSupervisorClient {
    LocalSupervisorClient::new(
        addr,
        SupervisorToken::new("synthetic-test-token".to_owned()).unwrap(),
    )
    .unwrap()
}
#[test]
fn generic_real_v1_and_opt_in_v2_gets_are_value_free() {
    let slug = "outside_vendor_724";
    for (v, path) in [
        (ApiVersion::V1, "/addons/outside_vendor_724/info"),
        (ApiVersion::V2, "/v2/apps/outside_vendor_724/info"),
    ] {
        let (addr, worker) = serve("200 OK", payload(slug));
        let schema = client(addr).fetch_app_config_schema(v, slug).unwrap();
        assert_eq!(schema.fields.len(), 3);
        assert!(schema.configuration_supported && schema.read_only);
        assert!(!schema.values_included && !schema.activation_authorized);
        let public = serde_json::to_string(&schema).unwrap();
        for forbidden in [
            "DO_NOT_EXPOSE",
            "PRIVATE_TOKEN",
            "/private",
            "options",
            "9001",
        ] {
            assert!(!public.contains(forbidden));
        }
        let req = worker.join().unwrap();
        assert!(req.starts_with(&format!("GET {path} HTTP/1.1\r\n")));
        assert!(req.contains("Authorization: Bearer synthetic-test-token\r\n"));
        assert!(!req.contains("POST "));
    }
}
#[test]
fn inaccessible_changed_or_ambiguous_results_fail_closed() {
    let unavailable = client("127.0.0.1:1".parse().unwrap());
    assert_eq!(
        unavailable.fetch_app_config_schema(ApiVersion::V1, "../unsafe"),
        Err(TransportError::InvalidAppSlug)
    );
    for (status, payload, expected) in [
        (
            "401 Unauthorized",
            payload("vendor2"),
            TransportError::Unauthorized,
        ),
        (
            "302 Found",
            payload("vendor2"),
            TransportError::UnexpectedStatus,
        ),
        ("200 OK", payload("vendor3"), TransportError::InvalidAppInfo),
        (
            "200 OK",
            r#"{"result":"ok","data":{"slug":"vendor2","schema":{},"schema":{}}}"#.into(),
            TransportError::InvalidAppInfo,
        ),
    ] {
        let (addr, worker) = serve(status, payload);
        assert_eq!(
            client(addr).fetch_app_config_schema(ApiVersion::V1, "vendor2"),
            Err(expected)
        );
        worker.join().unwrap();
    }
}
