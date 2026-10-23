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

#[test]
fn live_loopback_supervisor_catalog_supports_arbitrary_new_apps_and_filtering() {
    use hars_supervisor_apps::catalog_filter::Filter;
    let apps: Vec<_> = (0..1211)
        .map(|i| {
            serde_json::json!({
                "slug":format!("unknown_app_{i:05}"),
                "repository":format!("vendor_{}",i%17),
                "name":format!("Unseen Future App {i}"),
                "available":true,
                "installed":i%2==0,
                "update_available":i%3==0
            })
        })
        .collect();
    let repos: Vec<_> = (0..17)
        .map(|i| serde_json::json!({"slug":format!("vendor_{i}"),"name":format!("Vendor {i}")}))
        .collect();
    let data = serde_json::json!({"result":"ok","data":{"apps":apps,"repositories":repos}});
    let (addr, worker) = serve(ok(&data.to_string()));
    let store = client(addr).fetch_store(ApiVersion::V2).unwrap();
    assert_eq!(store.apps.len(), 1211);
    let page = store
        .filter_apps(Filter {
            repository: Some("VENDOR_9"),
            installed: Some(true),
            updates_only: true,
            limit: 200,
            ..Default::default()
        })
        .unwrap();
    assert!(page.total > 0);
    assert!(
        page.apps
            .iter()
            .all(|app| app.repository == "vendor_9" && app.installed && app.update_available)
    );
    assert!(worker.join().unwrap().starts_with("GET /v2/store HTTP/1.1"));
}

#[test]
fn any_supervisor_app_detail_is_fetched_over_authenticated_loopback_without_secrets() {
    let response = serde_json::json!({"result":"ok","data":{
        "slug":"thirdparty_future123","name":"Future Device",
        "repository":"unlisted_repo","state":"started",
        "available":true,"installed":true,"update_available":false,
        "options":{"password":"NEVER_OUTPUT"},"schema":{"token":"SUPERSECRET"},
        "ip_address":"192.0.2.99","devices":["/dev/private"]
    }});
    for (version, path) in [
        (ApiVersion::V1, "/addons/thirdparty_future123/info"),
        (ApiVersion::V2, "/v2/apps/thirdparty_future123/info"),
    ] {
        let (addr, worker) = serve(ok(&response.to_string()));
        let info = client(addr)
            .fetch_app_detail(version, "thirdparty_future123")
            .unwrap();
        assert_eq!(info.slug, "thirdparty_future123");
        assert_eq!(info.repository.as_deref(), Some("unlisted_repo"));
        let output = serde_json::to_string(&info).unwrap();
        for secret in [
            "NEVER_OUTPUT",
            "SUPERSECRET",
            "ip_address",
            "devices",
            "options",
        ] {
            assert!(!output.contains(secret));
        }
        let request = worker.join().unwrap();
        assert!(request.starts_with(&format!("GET {path} HTTP/1.1\r\n")));
        assert!(request.contains("Authorization: Bearer fixture_value_123\r\n"));
    }
}
#[test]
fn rejects_bad_slugs_without_network_access_and_unauthorized_info() {
    let api = client("127.0.0.1:1".parse().unwrap());
    for slug in [
        "",
        "../secret",
        "bad\nInjected: x",
        "https://example.invalid",
    ] {
        assert_eq!(
            api.fetch_app_detail(ApiVersion::V1, slug),
            Err(TransportError::InvalidAppSlug)
        );
    }
    let body = r#"{"result":"ok","data":{"slug":"not_the_requested","name":"Wrong App"}}"#;
    let (addr, worker) = serve(ok(body));
    assert_eq!(
        client(addr).fetch_app_detail(ApiVersion::V2, "correct_app"),
        Err(TransportError::InvalidAppInfo)
    );
    worker.join().unwrap();
    let (addr, worker) = serve("HTTP/1.1 403 Forbidden\r\nContent-Length: 2\r\n\r\n{}".into());
    assert_eq!(
        client(addr).fetch_app_detail(ApiVersion::V1, "good_app"),
        Err(TransportError::Unauthorized)
    );
    worker.join().unwrap();
}

#[test]
fn installed_apps_v1_v2_use_authenticated_supervisor_routes() {
    for (version, path, key) in [
        (ApiVersion::V1, "/addons", "addons"),
        (ApiVersion::V2, "/v2/apps", "apps"),
    ] {
        let mut data = serde_json::Map::new();
        data.insert(
            key.to_owned(),
            serde_json::json!([{
                "slug":"future_unlisted","name":"Future Provider",
                "repository":"new_vendor","state":"started","version":"3.1",
                "update_available":true,"options":{"password":"PRIVATE"}
            }]),
        );
        let body = serde_json::json!({"result":"ok","data":data}).to_string();
        let (addr, worker) = serve(ok(&body));
        let result = client(addr).fetch_installed(version).unwrap();
        assert_eq!(result.apps.len(), 1);
        assert_eq!(result.apps[0].slug, "future_unlisted");
        assert!(!serde_json::to_string(&result).unwrap().contains("PRIVATE"));
        let request = worker.join().unwrap();
        assert!(request.starts_with(&format!("GET {path} HTTP/1.1\r\n")));
        assert!(request.contains("Authorization: Bearer fixture_value_123"));
    }
}
#[test]
fn installed_apps_http_outage_is_not_an_empty_catalog() {
    let (addr, worker) = serve("HTTP/1.1 403 Forbidden\r\nContent-Length: 2\r\n\r\n{}".to_owned());
    assert!(matches!(
        client(addr).fetch_installed(ApiVersion::V2),
        Err(TransportError::Unauthorized)
    ));
    worker.join().unwrap();
    let (addr, worker) = serve(ok(r#"{"result":"ok","data":{"addons":[]}}"#));
    assert!(matches!(
        client(addr).fetch_installed(ApiVersion::V2),
        Err(TransportError::InvalidRuntimeInventory)
    ));
    worker.join().unwrap();
}

#[test]
fn supervisor_jobs_readonly_v1_v2_authorized_http() {
    use hars_supervisor_apps::job_status::State;
    let id = "abcdef0123456789abcdef0123456789";
    let reply = serde_json::json!({"result":"ok","data":{
        "uuid":id,"progress":100.0,"done":true,"stage":null,
        "errors":[],"child_jobs":[],"extra":{"password":"SECRET"}
    }});
    for (version, path) in [
        (ApiVersion::V1, format!("/jobs/{id}")),
        (ApiVersion::V2, format!("/v2/jobs/{id}")),
    ] {
        let (addr, worker) = serve(ok(&reply.to_string()));
        let result = client(addr).fetch_job(version, id).unwrap();
        assert_eq!(result.state, State::Succeeded);
        assert!(!serde_json::to_string(&result).unwrap().contains("SECRET"));
        let request = worker.join().unwrap();
        assert!(request.starts_with(&format!("GET {path} HTTP/1.1\r\n")));
        assert!(request.contains("Authorization: Bearer fixture_value_123"));
    }
    let local = client("127.0.0.1:1".parse().unwrap());
    assert_eq!(
        local.fetch_job(ApiVersion::V1, "../../bad"),
        Err(TransportError::InvalidJob)
    );
}

#[test]
fn official_store_detail_fetches_uninstalled_unknown_app_and_redacts_secrets() {
    let payload = serde_json::json!({"result":"ok","data":{
        "slug":"unknown_new_2060","name":"Unknown New App",
        "repository":"future_vendor","available":true,
        "version_latest":"4.0","installed":false,
        "options":{"api_key":"SECRET_MUST_NOT_LEAK"},
        "host_network":true,"devices":["/dev/private"]
    }});
    let (addr, worker) = serve(ok(&payload.to_string()));
    let app = client(addr)
        .fetch_store_app_detail(ApiVersion::V1, "unknown_new_2060")
        .unwrap();
    assert_eq!(app.slug, "unknown_new_2060");
    assert_eq!(app.repository.as_deref(), Some("future_vendor"));
    assert!(!app.activation_authorized);
    let safe = serde_json::to_string(&app).unwrap();
    assert!(!safe.contains("SECRET_MUST_NOT_LEAK"));
    assert!(!safe.contains("devices"));
    let request = worker.join().unwrap();
    assert!(request.starts_with("GET /store/addons/unknown_new_2060 HTTP/1.1"));
    assert!(request.contains("Authorization: Bearer fixture_value_123"));
}
