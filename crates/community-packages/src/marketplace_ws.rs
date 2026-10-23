//! Read-only adapter for the official Core Marketplace WebSocket API.
//! HA 2026.10 uses legacy HACS; HA 2026.11+ provides Core Marketplace.
//! This is not an installer or an authentication bypass.
use serde_json::{Value, json};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Integration,
    Plugin,
    Theme,
    Template,
}
impl Category {
    fn as_str(self) -> &'static str {
        match self {
            Self::Integration => "integration",
            Self::Plugin => "plugin",
            Self::Theme => "theme",
            Self::Template => "template",
        }
    }
    fn index(self) -> usize {
        match self {
            Self::Integration => 0,
            Self::Plugin => 1,
            Self::Theme => 2,
            Self::Template => 3,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadCommand {
    Info,
    List { categories: Vec<Category> },
    RepositoryInfo { repository_id: String },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadError {
    UnsupportedVersion,
    InvalidRequestId,
    InvalidRepositoryId,
    DuplicateCategory,
    InvalidResponse,
    ServerRejected,
}
/// Release gating does not substitute for the actual Core handshake.
pub fn core_marketplace_supported(version: &str) -> bool {
    let mut parts = version.split('.');
    let (Some(y), Some(m), Some(p), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return false;
    };
    if y.len() != 4
        || m.len() != 2
        || p.is_empty()
        || !y.bytes().all(|x| x.is_ascii_digit())
        || !m.bytes().all(|x| x.is_ascii_digit())
        || !p.as_bytes()[0].is_ascii_digit()
        || !p.bytes().all(|x| x.is_ascii_alphanumeric())
    {
        return false;
    }
    let (Ok(y), Ok(m)) = (y.parse::<u32>(), m.parse::<u32>()) else {
        return false;
    };
    (1..=12).contains(&m) && (y, m) >= (2026, 11)
}
fn safe_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
}
/// Construct only three read operations documented in upstream Core dev.
/// Core will separately require an authenticated admin for all three.
/// Request ids must be nonzero and unique on the actual WS connection.
pub fn read_request(version: &str, id: u64, op: ReadCommand) -> Result<Value, ReadError> {
    if !core_marketplace_supported(version) {
        return Err(ReadError::UnsupportedVersion);
    }
    if id == 0 {
        return Err(ReadError::InvalidRequestId);
    }
    match op {
        ReadCommand::Info => Ok(json!({"id":id,"type":"marketplace/info"})),
        ReadCommand::List { categories } => {
            let mut seen = [false; 4];
            for category in &categories {
                let i = category.index();
                if seen[i] {
                    return Err(ReadError::DuplicateCategory);
                }
                seen[i] = true;
            }
            if categories.is_empty() {
                Ok(json!({"id":id,"type":"marketplace/repositories/list"}))
            } else {
                Ok(
                    json!({"id":id,"type":"marketplace/repositories/list","categories":categories.iter().map(|x|x.as_str()).collect::<Vec<_>>()}),
                )
            }
        }
        ReadCommand::RepositoryInfo { repository_id } => {
            if !safe_id(&repository_id) {
                return Err(ReadError::InvalidRepositoryId);
            }
            Ok(json!({"id":id,"type":"marketplace/repository/info","repository_id":repository_id}))
        }
    }
}
/// A different id, denied Core permission, event or malformed reply is
/// never interpreted as an authorized state transition.
pub fn parse_read_result(id: u64, response: &Value) -> Result<&Value, ReadError> {
    if id == 0
        || response.get("id").and_then(Value::as_u64) != Some(id)
        || response.get("type").and_then(Value::as_str) != Some("result")
    {
        return Err(ReadError::InvalidResponse);
    }
    if response.get("success").and_then(Value::as_bool) != Some(true) {
        return Err(ReadError::ServerRejected);
    }
    response.get("result").ok_or(ReadError::InvalidResponse)
}
