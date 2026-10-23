//! Fixed-target, read-only Core WebSocket user list for Supervisor Ingress.
//! Only the separately gated probe may run in disposable QEMU.
//! Home Assistant Supervisor owns the credentials; never send token to a UI.
//! Do not interpret manager-role App token as an authenticated human account.
//! Only command issued: config/auth/list. No call_service or mutate commands.
use crate::core_user_snapshot_lab::{CoreUserError, CoreUserSnapshot};
use serde_json::{Value, json};
use std::io::{Read, Write};
use std::{
    env,
    net::{Ipv4Addr, SocketAddr, TcpStream, ToSocketAddrs},
    time::Duration,
};
use tungstenite::{Message, WebSocket, client::IntoClientRequest, client::client};
const TIMEOUT: Duration = Duration::from_secs(8);
const MAX_TEXT: usize = 524_288;
const COMMAND_ID: u64 = 17;
const OFFICIAL_SUPERVISOR_IP: Ipv4Addr = Ipv4Addr::new(172, 30, 32, 2);

fn is_official_supervisor(addr: SocketAddr) -> bool {
    matches!(addr, SocketAddr::V4(v4) if *v4.ip() == OFFICIAL_SUPERVISOR_IP && v4.port() == 80)
}

#[derive(Debug, PartialEq, Eq)]
pub enum CoreLiveError {
    LabDisabled,
    CredentialUnavailable,
    HostNotInternal,
    ConnectionFailed,
    Protocol,
    Unauthorized,
    CommandDenied,
}
pub enum CoreLiveResult {
    Users(CoreUserSnapshot),
    PermissionDenied,
}

fn read_text<S: Read + Write>(ws: &mut WebSocket<S>) -> Result<Value, CoreLiveError> {
    for _ in 0..4 {
        match ws.read().map_err(|_| CoreLiveError::Protocol)? {
            Message::Text(txt) => {
                if txt.len() > MAX_TEXT {
                    return Err(CoreLiveError::Protocol);
                }
                return serde_json::from_slice::<crate::NoDuplicates>(txt.as_bytes())
                    .map(|x| x.0)
                    .map_err(|_| CoreLiveError::Protocol);
            }
            Message::Ping(_) | Message::Pong(_) => continue,
            _ => return Err(CoreLiveError::Protocol),
        }
    }
    Err(CoreLiveError::Protocol)
}
fn run_protocol<S: Read + Write>(
    ws: &mut WebSocket<S>,
    token: &str,
    now: u64,
) -> Result<CoreLiveResult, CoreLiveError> {
    let hello = read_text(ws)?;
    if hello.get("type").and_then(Value::as_str) != Some("auth_required") {
        return Err(CoreLiveError::Protocol);
    }
    let auth = serde_json::to_string(&json!({"type":"auth","access_token":token}))
        .map_err(|_| CoreLiveError::Protocol)?;
    ws.send(Message::Text(auth.into()))
        .map_err(|_| CoreLiveError::ConnectionFailed)?;
    let reply = read_text(ws)?;
    match reply.get("type").and_then(Value::as_str) {
        Some("auth_invalid") => return Err(CoreLiveError::Unauthorized),
        Some("auth_ok") => {}
        _ => return Err(CoreLiveError::Protocol),
    }
    // Only this read-only Core command, never any user-supplied command type.
    ws.send(Message::Text(
        json!({"id":COMMAND_ID,"type":"config/auth/list"})
            .to_string()
            .into(),
    ))
    .map_err(|_| CoreLiveError::ConnectionFailed)?;
    let result = read_text(ws)?;
    if result.get("type").and_then(Value::as_str) != Some("result")
        || result.get("id").and_then(Value::as_u64) != Some(COMMAND_ID)
    {
        return Err(CoreLiveError::Protocol);
    }
    if result.get("success").and_then(Value::as_bool) == Some(false) {
        return Ok(CoreLiveResult::PermissionDenied);
    }
    if result.get("success").and_then(Value::as_bool) != Some(true) {
        return Err(CoreLiveError::Protocol);
    }
    let bytes = serde_json::to_vec(&result).map_err(|_| CoreLiveError::Protocol)?;
    let snapshot =
        CoreUserSnapshot::parse_fixture(&bytes, COMMAND_ID, now).map_err(|e| match e {
            CoreUserError::Mismatch => CoreLiveError::Protocol,
            _ => CoreLiveError::Protocol,
        })?;
    Ok(CoreLiveResult::Users(snapshot))
}
/// Read-only current Core account inventory, obtained via the fixed Supervisor
/// internal proxy. This is information only, NEVER an App write capability.
/// Call only after independently verifying the accepted TCP Ingress peer.
pub fn fetch_official_core_users_readonly(now: u64) -> Result<CoreLiveResult, CoreLiveError> {
    let token = env::var("SUPERVISOR_TOKEN")
        .ok()
        .filter(|x| !x.is_empty() && x.len() <= 4096 && x.bytes().all(|c| !c.is_ascii_control()))
        .ok_or(CoreLiveError::CredentialUnavailable)?;
    let addr = ("supervisor", 80)
        .to_socket_addrs()
        .map_err(|_| CoreLiveError::ConnectionFailed)?
        .find(|addr| is_official_supervisor(*addr))
        .ok_or(CoreLiveError::HostNotInternal)?;
    let stream =
        TcpStream::connect_timeout(&addr, TIMEOUT).map_err(|_| CoreLiveError::ConnectionFailed)?;
    stream
        .set_read_timeout(Some(TIMEOUT))
        .map_err(|_| CoreLiveError::ConnectionFailed)?;
    stream
        .set_write_timeout(Some(TIMEOUT))
        .map_err(|_| CoreLiveError::ConnectionFailed)?;
    let mut req = "ws://supervisor/core/websocket"
        .into_client_request()
        .map_err(|_| CoreLiveError::Protocol)?;
    // Internal App-only header, never logged. Core WS also requires auth frame.
    let value = format!("Bearer {token}")
        .parse()
        .map_err(|_| CoreLiveError::Protocol)?;
    req.headers_mut().insert("Authorization", value);
    let (mut ws, _) = client(req, stream).map_err(|_| CoreLiveError::ConnectionFailed)?;
    run_protocol(&mut ws, &token, now)
}

/// Preserve the explicit QEMU marker for the existing independent lab probe.
#[cfg(feature = "core-user-live-lab")]
pub fn fetch_official_core_users_qemu(now: u64) -> Result<CoreLiveResult, CoreLiveError> {
    if env::var("HARS_P8H_QEMU_ONLY").ok().as_deref() != Some("1") {
        return Err(CoreLiveError::LabDisabled);
    }
    fetch_official_core_users_readonly(now)
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::thread;
    fn mock(status: &str) -> Result<CoreLiveResult, CoreLiveError> {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let s = status.to_owned();
        let task = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            stream.set_read_timeout(Some(TIMEOUT)).unwrap();
            stream.set_write_timeout(Some(TIMEOUT)).unwrap();
            let mut ws = tungstenite::accept(stream).unwrap();
            ws.send(Message::Text(
                r#"{"type":"auth_required","ha_version":"2026.10.2"}"#.into(),
            ))
            .unwrap();
            let response: Value =
                serde_json::from_str(ws.read().unwrap().to_text().unwrap()).unwrap();
            assert_eq!(response["type"], "auth");
            assert_eq!(response["access_token"], "test-secret-do-not-log");
            if s == "invalid" {
                ws.send(Message::Text(
                    r#"{"type":"auth_invalid","message":"Invalid"}"#.into(),
                ))
                .unwrap();
                return;
            }
            ws.send(Message::Text(
                r#"{"type":"auth_ok","ha_version":"2026.10.2"}"#.into(),
            ))
            .unwrap();
            let cmd: Value = serde_json::from_str(ws.read().unwrap().to_text().unwrap()).unwrap();
            assert_eq!(cmd["type"], "config/auth/list");
            assert_eq!(cmd["id"], COMMAND_ID);
            let result = match s.as_str() {
                "ok" | "revoked" | "system" | "member" => json!({
                    "type":"result","id":17,"success":true,"result":[{
                    "id":"testuser","is_owner":s == "ok","is_active":s != "revoked",
                    "system_generated":s == "system",
                    "group_ids": if s == "ok" || s == "system" {
                        vec!["system-admin"]
                    } else {
                        vec!["system-users"]
                    }
                }]}),
                "denied" => {
                    json!({"type":"result","id":17,"success":false,"error":{"code":"unauthorized"}})
                }
                "wrong" => json!({"type":"result","id":19,"success":true,"result":[]}),
                _ => unreachable!(),
            };
            ws.send(Message::Text(result.to_string().into())).unwrap();
        });
        let stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
        stream.set_read_timeout(Some(TIMEOUT)).unwrap();
        stream.set_write_timeout(Some(TIMEOUT)).unwrap();
        let (mut ws, _) = client(format!("ws://127.0.0.1:{port}/core/websocket"), stream).unwrap();
        let response = run_protocol(&mut ws, "test-secret-do-not-log", 100);
        task.join().unwrap();
        response
    }
    #[test]
    fn core_user_transport_never_accepts_alternate_private_dns_or_port() {
        assert!(is_official_supervisor("172.30.32.2:80".parse().unwrap()));
        for addr in [
            "172.30.32.2:443",
            "172.30.32.3:80",
            "172.16.0.2:80",
            "10.0.0.2:80",
            "127.0.0.1:80",
            "[::1]:80",
            "[fd00::2]:80",
        ] {
            assert!(!is_official_supervisor(addr.parse().unwrap()), "{addr}");
        }
    }
    #[test]
    fn real_loopback_websocket_protocol_with_no_mutation_command() {
        assert!(matches!(mock("ok"), Ok(CoreLiveResult::Users(_))));
        assert!(matches!(
            mock("denied"),
            Ok(CoreLiveResult::PermissionDenied)
        ));
        assert!(matches!(mock("wrong"), Err(CoreLiveError::Protocol)));
    }
    #[test]
    fn authenticated_core_websocket_observes_current_revocation_and_role() {
        use crate::core_user_snapshot_lab::CoreAccountStatus;
        use crate::ingress_identity_lab::ingress_user_claim;
        use std::net::{Ipv4Addr, SocketAddrV4};
        let claim = ingress_user_claim(
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(172, 30, 32, 2), 3000)),
            &[("X-Remote-User-Id", "testuser")],
        )
        .unwrap();
        for (fixture, expected) in [
            ("ok", CoreAccountStatus::ActiveAdmin),
            ("member", CoreAccountStatus::ActiveNonAdmin),
            ("revoked", CoreAccountStatus::Denied),
            ("system", CoreAccountStatus::Denied),
        ] {
            let CoreLiveResult::Users(users) = mock(fixture).unwrap() else {
                panic!("expected current Core user list");
            };
            assert_eq!(users.status(&claim, 100).unwrap(), expected);
            assert!(users.status(&claim, 115).is_err(), "stale Core grant");
        }
    }
    #[test]
    fn rejects_auth_invalid_and_unmarked_environment() {
        assert!(matches!(mock("invalid"), Err(CoreLiveError::Unauthorized)));
        // The independent QEMU probe retains its separate explicit marker.
        #[cfg(feature = "core-user-live-lab")]
        if env::var("HARS_P8H_QEMU_ONLY").ok().as_deref() != Some("1") {
            assert!(matches!(
                fetch_official_core_users_qemu(100),
                Err(CoreLiveError::LabDisabled)
            ));
        }
    }
}
