//! P8ei isolated executable: real MQTT via official App image plugin.
//! NOT a production app, daemon, Supervisor identity provider or MQTT ACL.
use hars_mqtt_supervisor_auth_lab::{AuthDecision, LabAuthApiPermission, MqttSupervisorAuthLab};
use serde_json::Value;
use std::{
    env, fs,
    io::{Read, Write},
    net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream},
    path::Path,
    thread,
    time::Duration,
};

const SUP: u16 = 18082;
const GATE: u16 = 18083;
const MQTT: u16 = 18883;
const TOKEN: &str = "P8EI_SYNTHETIC_SUPERVISOR_API_TEST_TOKEN";
const USER: &str = "p8ei-test-user";
const PASS: &str = "p8ei-SYNTHETIC-test-only-password-2026";
const IO_TIMEOUT: Duration = Duration::from_secs(3);
fn addr(port: u16) -> SocketAddr {
    SocketAddr::from((Ipv4Addr::LOCALHOST, port))
}
fn read_http(stream: &mut TcpStream) -> Result<(String, String, Vec<u8>), &'static str> {
    stream
        .set_read_timeout(Some(IO_TIMEOUT))
        .map_err(|_| "timeout")?;
    let mut raw = Vec::new();
    loop {
        let mut chunk = [0u8; 1024];
        let n = stream.read(&mut chunk).map_err(|_| "read")?;
        if n == 0 {
            return Err("eof");
        }
        raw.extend_from_slice(&chunk[..n]);
        if raw.len() > 4096 {
            return Err("too_large");
        }
        if let Some(cut) = raw.windows(4).position(|x| x == b"\r\n\r\n") {
            let header = std::str::from_utf8(&raw[..cut]).map_err(|_| "utf8")?;
            let mut lines = header.split("\r\n");
            let request = lines.next().ok_or("missing_method")?;
            let words: Vec<_> = request.split_whitespace().collect();
            if words.len() != 3 || !["HTTP/1.1", "HTTP/1.0"].contains(&words[2]) {
                return Err("method");
            }
            let mut length = None;
            for line in lines {
                let (k, v) = line.split_once(':').ok_or("header")?;
                if k.eq_ignore_ascii_case("transfer-encoding") {
                    return Err("chunked");
                }
                if k.eq_ignore_ascii_case("content-length") {
                    if length.is_some() {
                        return Err("duplicate_length");
                    }
                    length = Some(v.trim().parse::<usize>().map_err(|_| "length")?);
                }
            }
            let n = length.ok_or("missing_length")?;
            if n > 2048 {
                return Err("size");
            }
            if raw.len() >= cut + 4 + n {
                if raw.len() != cut + 4 + n {
                    return Err("extra");
                }
                return Ok((
                    words[0].to_string(),
                    words[1].to_string(),
                    raw[cut + 4..].to_vec(),
                ));
            }
        }
    }
}
fn http_reply(s: &mut TcpStream, status: u16, body: &str) {
    let label = match status {
        200 => "OK",
        401 => "Unauthorized",
        503 => "Service Unavailable",
        _ => "Forbidden",
    };
    let h = format!(
        "HTTP/1.1 {status} {label}\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = s.write_all(h.as_bytes());
    let _ = s.write_all(body.as_bytes());
}
fn spawn_fake_supervisor(flag: String) -> std::io::Result<()> {
    let listener = TcpListener::bind(addr(SUP))?;
    thread::spawn(move || {
        for accepted in listener.incoming() {
            let Ok(mut s) = accepted else { continue };
            let flag = flag.clone();
            thread::spawn(move || {
                let Ok((method, path, body)) = read_http(&mut s) else {
                    http_reply(&mut s, 403, r#"{"result":"error"}"#);
                    return;
                };
                if method != "POST" || path != "/auth" {
                    http_reply(&mut s, 403, r#"{"result":"error"}"#);
                    return;
                }
                let flag = fs::read_to_string(flag).unwrap_or_default();
                if flag.trim() == "outage" {
                    http_reply(&mut s, 503, r#"{"result":"error"}"#);
                    return;
                }
                let Ok(value) = serde_json::from_slice::<Value>(&body) else {
                    http_reply(&mut s, 403, r#"{"result":"error"}"#);
                    return;
                };
                let match_user = value.as_object().is_some_and(|o| {
                    o.len() == 2
                        && o.get("username").and_then(Value::as_str) == Some(USER)
                        && o.get("password").and_then(Value::as_str) == Some(PASS)
                });
                if flag.trim() == "permit" && match_user {
                    http_reply(&mut s, 200, r#"{"result":"ok"}"#)
                } else {
                    http_reply(&mut s, 401, r#"{"result":"error"}"#)
                }
            });
        }
    });
    Ok(())
}
fn serve(flag: String) -> Result<(), Box<dyn std::error::Error>> {
    if !Path::new(&flag).is_file() {
        return Err("synthetic fixture flag is missing".into());
    }
    spawn_fake_supervisor(flag)?;
    let listener = TcpListener::bind(addr(GATE))?;
    for incoming in listener.incoming() {
        let mut s = incoming?;
        thread::spawn(move || {
            let Ok((method, path, body)) = read_http(&mut s) else {
                http_reply(&mut s, 403, r#"{"allowed":false}"#);
                return;
            };
            let Ok(bridge) = MqttSupervisorAuthLab::new(
                addr(SUP),
                TOKEN.to_string(),
                LabAuthApiPermission::FixtureGranted,
            ) else {
                http_reply(&mut s, 403, r#"{"allowed":false}"#);
                return;
            };
            match bridge.decide_go_auth(&method, &path, &body) {
                AuthDecision::Allow => http_reply(&mut s, 200, r#"{"allowed":true}"#),
                AuthDecision::Deny => http_reply(&mut s, 401, r#"{"allowed":false}"#),
            }
        });
    }
    Ok(())
}
fn mqtt_string(out: &mut Vec<u8>, text: &str) {
    let len = u16::try_from(text.len()).unwrap();
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(text.as_bytes());
}
fn connect_probe(kind: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let anon = kind == "anonymous";
    let mut packet = vec![
        0,
        4,
        b'M',
        b'Q',
        b'T',
        b'T',
        4,
        if anon { 2 } else { 0xc2 },
        0,
        10,
    ];
    mqtt_string(&mut packet, "p8ei-disposable-client");
    if !anon {
        mqtt_string(
            &mut packet,
            if kind == "reserved" {
                "homeassistant"
            } else {
                USER
            },
        );
        mqtt_string(
            &mut packet,
            if kind == "wrong" {
                "P8EI_FAKE_BAD_PASSWORD"
            } else {
                PASS
            },
        );
    }
    if packet.len() > 127 {
        return Err("bad probe packet length".into());
    }
    let mut s = TcpStream::connect_timeout(&addr(MQTT), IO_TIMEOUT)?;
    s.set_read_timeout(Some(IO_TIMEOUT))?;
    s.set_write_timeout(Some(IO_TIMEOUT))?;
    s.write_all(&[0x10, packet.len() as u8])?;
    s.write_all(&packet)?;
    let mut response = [0u8; 4];
    s.read_exact(&mut response)?;
    if response[..3] != [0x20, 0x02, 0x00] || ![0, 5].contains(&response[3]) {
        return Err(format!("unexpected real MQTT CONNACK code: {response:?}").into());
    }
    let _ = s.write_all(&[0xe0, 0]);
    Ok(response[3] == 0)
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<String> = env::args().collect();
    match a.get(1).map(String::as_str) {
        Some("server") => serve(a.get(2).ok_or("missing control flag")?.clone()),
        Some("ready") => {
            for p in [SUP, GATE] {
                TcpStream::connect_timeout(&addr(p), IO_TIMEOUT)?;
            }
            println!("P8EI_TWO_FAKE_SUPERVISOR_RUST_GATEWAY_BOUND_TO_CONTAINER_LOOPBACK_PASS");
            Ok(())
        }
        Some("probe") => {
            let kind = a.get(2).ok_or("scenario")?;
            if !["valid", "wrong", "anonymous", "reserved"].contains(&kind.as_str()) {
                return Err("unrecognized probe".into());
            }
            let observed = connect_probe(kind)?;
            if observed != (kind == "valid") {
                return Err(format!("actual MQTT CONNACK mismatch for {kind}: {observed}").into());
            }
            println!("P8EI_OFFICIAL_MOSQUITTO_GO_AUTH_MQTT_{kind}_CONNACK_PASS");
            Ok(())
        }
        Some("probe-denied-valid") => {
            if connect_probe("valid")? {
                return Err("revoked or unavailable login was accepted".into());
            }
            println!("P8EI_OFFICIAL_MOSQUITTO_GO_AUTH_REVOKED_OR_UNAVAILABLE_DENIED_PASS");
            Ok(())
        }
        _ => Err("P8ei isolated server/ready/probe/probe-denied-valid required".into()),
    }
}
