//! Experimental standalone Rust MQTT HTTP -> Supervisor /auth lab boundary.
//! Entirely lab-only: not a daemon, not a broker auth plugin, not real HAOS
//! privileges; verifies real loopback TCP protocol against a disposable mock.
//! No HTTP listener, no production token, no ACL/administrator authorization.

use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    fmt,
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    time::Duration,
};

const TIMEOUT: Duration = Duration::from_millis(650);
const MAX_WIRE: usize = 4096;
const MAX_INCOMING: usize = 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthDecision {
    Allow,
    Deny,
}

/// Explicit fixture flag, NEVER an actual proof of Supervisor auth_api grant.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LabAuthApiPermission {
    NotGranted,
    FixtureGranted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthTransportError {
    InvalidConfiguration,
    ConnectionFailure,
    InvalidHttpReply,
    UnexpectedStatus,
}

/// Derived deserializer rejects duplicate keys and unknown go-auth fields.
/// Do not derive Debug/Serialize to avoid leaking transient credentials.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GoAuthCredentials {
    username: String,
    password: String,
    clientid: String,
}

pub struct MqttSupervisorAuthLab {
    target: SocketAddr,
    token: String,
    permission: LabAuthApiPermission,
}
impl fmt::Debug for MqttSupervisorAuthLab {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("MqttSupervisorAuthLab([REDACTED])")
    }
}

impl MqttSupervisorAuthLab {
    pub fn new(
        target: SocketAddr,
        token: String,
        permission: LabAuthApiPermission,
    ) -> Result<Self, AuthTransportError> {
        if !target.ip().is_loopback()
            || target.port() == 0
            || token.is_empty()
            || token.len() > 4096
            || !token.bytes().all(|b| (33..=126).contains(&b))
        {
            return Err(AuthTransportError::InvalidConfiguration);
        }
        Ok(Self {
            target,
            token,
            permission,
        })
    }

    /// In-process go-auth HTTP contract, no listener or network exposure.
    /// All ACL/superuser/malformed/unauthorized requests deny with no fallback.
    pub fn decide_go_auth(&self, method: &str, path: &str, raw: &[u8]) -> AuthDecision {
        if self.permission != LabAuthApiPermission::FixtureGranted
            || method != "POST"
            || path != "/authentication"
            || raw.is_empty()
            || raw.len() > MAX_INCOMING
        {
            return AuthDecision::Deny;
        }
        let Ok(GoAuthCredentials {
            username,
            password,
            clientid,
        }) = serde_json::from_slice::<GoAuthCredentials>(raw)
        else {
            return AuthDecision::Deny;
        };
        if username.is_empty()
            || username.len() > 128
            || username.chars().any(char::is_control)
            || username.eq_ignore_ascii_case("addons")
            || username.eq_ignore_ascii_case("homeassistant")
            || password.is_empty()
            || password.len() > 512
            || password.chars().any(char::is_control)
            || clientid.is_empty()
            || clientid.len() > 128
            || clientid.chars().any(char::is_control)
        {
            return AuthDecision::Deny;
        }
        match self.call_supervisor(&username, &password) {
            Ok(true) => AuthDecision::Allow,
            _ => AuthDecision::Deny,
        }
    }

    fn call_supervisor(&self, username: &str, password: &str) -> Result<bool, AuthTransportError> {
        let body = serde_json::to_vec(&json!({"username": username, "password": password}))
            .map_err(|_| AuthTransportError::InvalidConfiguration)?;
        let mut stream = TcpStream::connect_timeout(&self.target, TIMEOUT)
            .map_err(|_| AuthTransportError::ConnectionFailure)?;
        stream
            .set_read_timeout(Some(TIMEOUT))
            .map_err(|_| AuthTransportError::ConnectionFailure)?;
        stream
            .set_write_timeout(Some(TIMEOUT))
            .map_err(|_| AuthTransportError::ConnectionFailure)?;
        let headers = format!(
            "POST /auth HTTP/1.1\r\nHost: supervisor\r\nX-Supervisor-Token: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            self.token,
            body.len()
        );
        stream
            .write_all(headers.as_bytes())
            .and_then(|_| stream.write_all(&body))
            .map_err(|_| AuthTransportError::ConnectionFailure)?;
        let mut response = Vec::new();
        stream
            .take((MAX_WIRE + 1) as u64)
            .read_to_end(&mut response)
            .map_err(|_| AuthTransportError::ConnectionFailure)?;
        if response.len() > MAX_WIRE {
            return Err(AuthTransportError::InvalidHttpReply);
        }
        parse_supervisor_reply(&response)
    }
}

fn parse_supervisor_reply(wire: &[u8]) -> Result<bool, AuthTransportError> {
    let cut = wire
        .windows(4)
        .position(|s| s == b"\r\n\r\n")
        .ok_or(AuthTransportError::InvalidHttpReply)?;
    let header =
        std::str::from_utf8(&wire[..cut]).map_err(|_| AuthTransportError::InvalidHttpReply)?;
    let mut lines = header.split("\r\n");
    let status = match lines.next() {
        Some("HTTP/1.1 200 OK" | "HTTP/1.0 200 OK") => 200,
        Some("HTTP/1.1 401 Unauthorized" | "HTTP/1.0 401 Unauthorized") => 401,
        _ => return Err(AuthTransportError::UnexpectedStatus),
    };
    let mut length = None;
    for line in lines {
        let (key, value) = line
            .split_once(':')
            .ok_or(AuthTransportError::InvalidHttpReply)?;
        if key.eq_ignore_ascii_case("transfer-encoding")
            || key.eq_ignore_ascii_case("content-encoding")
        {
            return Err(AuthTransportError::InvalidHttpReply);
        }
        if key.eq_ignore_ascii_case("content-length") {
            if length.is_some() {
                return Err(AuthTransportError::InvalidHttpReply);
            }
            length = Some(
                value
                    .trim()
                    .parse::<usize>()
                    .map_err(|_| AuthTransportError::InvalidHttpReply)?,
            );
        }
    }
    let body = &wire[cut + 4..];
    if length != Some(body.len()) || body.len() > 2048 {
        return Err(AuthTransportError::InvalidHttpReply);
    }
    if status == 401 {
        return Ok(false);
    }
    let value: Value =
        serde_json::from_slice(body).map_err(|_| AuthTransportError::InvalidHttpReply)?;
    if value
        .as_object()
        .is_none_or(|v| v.get("result").and_then(Value::as_str) != Some("ok"))
    {
        return Err(AuthTransportError::InvalidHttpReply);
    }
    Ok(true)
}
