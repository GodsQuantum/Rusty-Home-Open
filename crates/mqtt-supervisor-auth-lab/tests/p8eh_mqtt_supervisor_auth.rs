use hars_mqtt_supervisor_auth_lab::{AuthDecision, LabAuthApiPermission, MqttSupervisorAuthLab};
use serde_json::json;
use std::{
    io::{Read, Write},
    net::{Ipv4Addr, SocketAddr, TcpListener},
    thread,
    time::Duration,
};

const FAKE_TOKEN: &str = "P8EH_FICTIONAL_SUPERVISOR_AUTH_API_TOKEN";
const FAKE_PASSWORD: &str = "P8EH_FICTIONAL_MQTT_CLIENT_PASSWORD";

fn msg(user: &str, pass: &str) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "username": user,
        "password": pass,
        "clientid": "p8eh-disposable-fixture-client"
    }))
    .unwrap()
}
fn response(status: &str, body: &str, more_headers: &str) -> Vec<u8> {
    format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\n{more_headers}Connection: close\r\n\r\n{body}",
        body.len()
    )
    .into_bytes()
}
fn fake_supervisor(reply: Vec<u8>, delay: Duration) -> (SocketAddr, thread::JoinHandle<Vec<u8>>) {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let address = listener.local_addr().unwrap();
    let join = thread::spawn(move || {
        let (mut conn, _) = listener.accept().unwrap();
        conn.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        let mut bytes = Vec::new();
        loop {
            let mut chunk = [0u8; 1024];
            let n = conn.read(&mut chunk).unwrap_or(0);
            if n == 0 {
                break;
            }
            bytes.extend_from_slice(&chunk[..n]);
            if bytes.len() > 8192 {
                break;
            }
            if let Some(cut) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                let header = String::from_utf8_lossy(&bytes[..cut]);
                if let Some(line) = header
                    .lines()
                    .find(|l| l.to_ascii_lowercase().starts_with("content-length:"))
                {
                    let length: usize = line.split_once(':').unwrap().1.trim().parse().unwrap();
                    if bytes.len() >= cut + 4 + length {
                        break;
                    }
                }
            }
        }
        thread::sleep(delay);
        let _ = conn.write_all(&reply);
        bytes
    });
    (address, join)
}
fn instance(port: SocketAddr) -> MqttSupervisorAuthLab {
    MqttSupervisorAuthLab::new(
        port,
        FAKE_TOKEN.into(),
        LabAuthApiPermission::FixtureGranted,
    )
    .unwrap()
}

#[test]
fn genuine_loopback_http_post_auth_accepts_fixture_valid_and_rejects_invalid() {
    for (status, body, expected) in [
        (
            "200 OK",
            r#"{"result":"ok","data":{}}"#,
            AuthDecision::Allow,
        ),
        (
            "401 Unauthorized",
            r#"{"result":"error"}"#,
            AuthDecision::Deny,
        ),
    ] {
        let (address, join) = fake_supervisor(response(status, body, ""), Duration::ZERO);
        let client = instance(address);
        let result = client.decide_go_auth(
            "POST",
            "/authentication",
            &msg("fixture-user", FAKE_PASSWORD),
        );
        assert_eq!(result, expected);
        let request = String::from_utf8(join.join().unwrap()).unwrap();
        assert!(request.starts_with("POST /auth HTTP/1.1\r\n"));
        assert!(request.contains(&format!("X-Supervisor-Token: {FAKE_TOKEN}\r\n")));
        assert!(!request.contains("Authorization:"));
        assert!(!request.contains("Bearer "));
        let data = request.split("\r\n\r\n").nth(1).unwrap();
        let json: serde_json::Value = serde_json::from_str(data).unwrap();
        assert_eq!(json["username"], "fixture-user");
        assert_eq!(json["password"], FAKE_PASSWORD);
        assert!(
            json.get("clientid").is_none(),
            "Supervisor /auth should receive only official username/password"
        );
        assert!(!format!("{client:?}").contains(FAKE_TOKEN));
        assert!(!format!("{client:?}").contains(FAKE_PASSWORD));
    }
    println!("P8EH_REAL_LOOPBACK_SUPERVISOR_POST_AUTH_ALLOW_DENY_TOKEN_HEADER_REDACTED_PASS");
}

#[test]
fn supervisor_offline_and_permission_denied_never_authorize() {
    let dead = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let addr = dead.local_addr().unwrap();
    drop(dead);
    let client = instance(addr);
    assert_eq!(
        client.decide_go_auth("POST", "/authentication", &msg("user", FAKE_PASSWORD)),
        AuthDecision::Deny
    );
    let disabled =
        MqttSupervisorAuthLab::new(addr, FAKE_TOKEN.into(), LabAuthApiPermission::NotGranted)
            .unwrap();
    assert_eq!(
        disabled.decide_go_auth("POST", "/authentication", &msg("user", FAKE_PASSWORD)),
        AuthDecision::Deny
    );
    for bad in ["", "key\r\nX-Forged: 1", "key\n", " ".repeat(5000).as_str()] {
        assert!(
            MqttSupervisorAuthLab::new(addr, bad.into(), LabAuthApiPermission::FixtureGranted)
                .is_err()
        );
    }
    let external = SocketAddr::from(([192, 0, 2, 10], 80));
    assert!(
        MqttSupervisorAuthLab::new(
            external,
            "fake".into(),
            LabAuthApiPermission::FixtureGranted
        )
        .is_err()
    );
    println!("P8EH_SUPERVISOR_UNREACHABLE_NO_PERMISSION_INVALID_TOKEN_EXTERNAL_EGRESS_DENIED_PASS");
}

#[test]
fn unimplemented_superuser_acl_and_malformed_go_auth_never_hit_identity_backend() {
    let dead = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let address = dead.local_addr().unwrap();
    drop(dead);
    let client = instance(address);
    let valid = msg("fixture-user", FAKE_PASSWORD);
    for (method, path) in [
        ("GET", "/authentication"),
        ("POST", "/auth"),
        ("POST", "/acl"),
        ("POST", "/superuser"),
        ("POST", "/authentication/"),
        ("POST", "/authentication?admin=true"),
        ("DELETE", "/authentication"),
    ] {
        assert_eq!(
            client.decide_go_auth(method, path, &valid),
            AuthDecision::Deny
        );
    }
    for raw in [
        b"{}".as_slice(),
        b"not-json".as_slice(),
        br#"{"username":"x","password":"z","clientid":"i","admin":true}"#.as_slice(),
        br#"{"username":"homeassistant","password":"a","clientid":"i"}"#.as_slice(),
        br#"{"username":"addons","password":"a","clientid":"i"}"#.as_slice(),
        br#"{"username":"x","password":"","clientid":"i"}"#.as_slice(),
        br#"{"username":"x","password":"z","clientid":""}"#.as_slice(),
        br#"{"username":"x","username":"y","password":"z","clientid":"i"}"#.as_slice(),
    ] {
        assert_eq!(
            client.decide_go_auth("POST", "/authentication", raw),
            AuthDecision::Deny
        );
    }
    assert_eq!(
        client.decide_go_auth("POST", "/authentication", &vec![b'x'; 1025]),
        AuthDecision::Deny
    );
    println!("P8EH_MQTT_SUPERUSER_ACL_RESERVED_DUPLICATE_JSON_AND_HTTP_PATH_DENY_PASS");
}

#[test]
fn malformed_forged_redirect_and_oversized_success_responses_fail_closed() {
    let cases: Vec<Vec<u8>> = vec![
        response("302 Found", "", "Location: http://example.invalid/\r\n"),
        response("403 Forbidden", r#"{"result":"ok"}"#, ""),
        response("500 Internal Server Error", r#"{"result":"ok"}"#, ""),
        response("200 OK", "{}", ""),
        response("200 OK", r#"{"result":"error"}"#, ""),
        response("200 OK", "not-json", ""),
        response(
            "200 OK",
            r#"{"result":"ok"}"#,
            "Transfer-Encoding: chunked\r\n",
        ),
        response("200 OK", r#"{"result":"ok"}"#, "Content-Length: 15\r\n"),
        b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n{\"result\":\"ok\"}".to_vec(),
        b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\n{\"result\":\"ok\"}".to_vec(),
        response("200 OK", &"x".repeat(3500), ""),
    ];
    for reply in cases {
        let (addr, join) = fake_supervisor(reply, Duration::ZERO);
        assert_eq!(
            instance(addr).decide_go_auth("POST", "/authentication", &msg("user", FAKE_PASSWORD)),
            AuthDecision::Deny
        );
        join.join().unwrap();
    }
    println!(
        "P8EH_FORGED_SUCCESS_REDIRECT_403_500_DUPLICATE_LENGTH_TRUNCATED_OVERSIZED_REFUSED_PASS"
    );
}

#[test]
fn slow_supervisor_fails_bounded_and_does_not_cache_good_identity() {
    let (addr, join) = fake_supervisor(
        response("200 OK", r#"{"result":"ok"}"#, ""),
        Duration::from_millis(900),
    );
    let client = instance(addr);
    let start = std::time::Instant::now();
    assert_eq!(
        client.decide_go_auth("POST", "/authentication", &msg("user", FAKE_PASSWORD)),
        AuthDecision::Deny
    );
    assert!(start.elapsed() < Duration::from_millis(850));
    join.join().unwrap();
    println!("P8EH_SUPERVISOR_TIMEOUT_FAIL_CLOSED_NO_CLIENT_SIDE_CREDENTIAL_CACHE_PASS");
}
