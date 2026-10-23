//! Explicit, read-only and locally scoped transport for official Supervisor.
//! This does NOT install Apps or connect to a real HAOS Supervisor by itself.
use crate::app_detail::{AppDetail, detail_path, parse_app_detail};
use crate::installed_inventory::{RunningInventory, parse_running_inventory};
use crate::official_store::{ApiVersion, Catalog, parse_store_response};
use std::{
    fmt,
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    time::Duration,
};

const MAX_BODY: usize = 8 * 1024 * 1024;
const MAX_HEADERS: usize = 16 * 1024;
const IO_TIMEOUT: Duration = Duration::from_secs(3);

/// Token never derives Display, Serialize or its original Debug value.
pub struct SupervisorToken(String);
impl fmt::Debug for SupervisorToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SupervisorToken([REDACTED])")
    }
}
impl SupervisorToken {
    pub fn new(value: String) -> Result<Self, TransportError> {
        if value.is_empty() || value.len() > 4096 || !value.bytes().all(|c| (33..=126).contains(&c))
        {
            return Err(TransportError::InvalidToken);
        }
        Ok(Self(value))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportError {
    UnsupportedHost,
    InvalidToken,
    Io,
    Unauthorized,
    UnexpectedStatus,
    BadResponse,
    TooLarge,
    UnexpectedApiVersion,
    InvalidStore,
    InvalidAppSlug,
    InvalidAppInfo,
    InvalidRuntimeInventory,
    InvalidJob,
    #[cfg(feature = "managed-apps-preview")]
    ManagedPreviewDisabled,
    #[cfg(feature = "lab-app-actions")]
    LabMutationDisabled,
}

/// Correlate an authenticated Supervisor job with separately observed App state.
/// The Watch must be created from a trusted operation receipt: the job
/// endpoint alone does not bind a job UUID to an App slug.
fn observe_operation_with<FJ, FI>(
    version: ApiVersion,
    watch: &crate::operation_reconcile::Watch,
    fetch_job: FJ,
    fetch_inventory: FI,
) -> Result<crate::operation_reconcile::Observation, TransportError>
where
    FJ: FnOnce(ApiVersion, &str) -> Result<crate::job_status::JobStatus, TransportError>,
    FI: FnOnce(ApiVersion) -> Result<RunningInventory, TransportError>,
{
    use crate::job_status::State;
    use crate::operation_reconcile::reconcile;
    let job = fetch_job(version, watch.job_id())?;
    if job.state != State::Succeeded {
        return Ok(reconcile(watch, Some(&job), None));
    }
    // No fallback to cached, stale, empty or unauthenticated inventory.
    let inventory = fetch_inventory(version)?;
    Ok(reconcile(watch, Some(&job), Some(&inventory)))
}

/// Only loopback connections. Future HAOS deployment requires an independently
/// authenticated, authorized and encrypted transport to its runtime agent.
pub struct LocalSupervisorClient {
    target: SocketAddr,
    token: SupervisorToken,
}
impl LocalSupervisorClient {
    /// Read-only official job + postcondition observation; never mutates Apps.
    /// Trusted user identity, operation receipt and polling are caller duties.
    pub fn observe_operation(
        &self,
        version: ApiVersion,
        watch: &crate::operation_reconcile::Watch,
    ) -> Result<crate::operation_reconcile::Observation, TransportError> {
        observe_operation_with(
            version,
            watch,
            |v, id| self.fetch_job(v, id),
            |v| self.fetch_installed(v),
        )
    }

    /// Installed Apps, never confused with the Store catalog.
    pub fn fetch_installed(&self, version: ApiVersion) -> Result<RunningInventory, TransportError> {
        let path = match version {
            ApiVersion::V1 => "/addons",
            ApiVersion::V2 => "/v2/apps",
        };
        let body = get_json(self.target, &self.token, path)?;
        parse_running_inventory(&body, version).map_err(|_| TransportError::InvalidRuntimeInventory)
    }

    pub fn new(target: SocketAddr, token: SupervisorToken) -> Result<Self, TransportError> {
        if !target.ip().is_loopback() || target.port() == 0 {
            return Err(TransportError::UnsupportedHost);
        }
        Ok(Self { target, token })
    }

    /// Read one official Supervisor job with validated ID and no mutations.
    pub fn fetch_job(
        &self,
        version: ApiVersion,
        id: &str,
    ) -> Result<crate::job_status::JobStatus, TransportError> {
        let path =
            crate::job_status::job_path(version, id).map_err(|_| TransportError::InvalidJob)?;
        let body = get_json(self.target, &self.token, &path)?;
        crate::job_status::parse_job_response(&body, id).map_err(|_| TransportError::InvalidJob)
    }

    /// Authenticated, read-only details of an arbitrary installed App.
    /// Sensitive options and device addresses are dropped by the typed parser.
    pub fn fetch_app_detail(
        &self,
        version: ApiVersion,
        slug: &str,
    ) -> Result<AppDetail, TransportError> {
        let path = detail_path(version, slug).map_err(|_| TransportError::InvalidAppSlug)?;
        let body = get_json(self.target, &self.token, &path)?;
        parse_app_detail(&body, slug).map_err(|_| TransportError::InvalidAppInfo)
    }

    /// GET only; a V2 request is opt-in because official Supervisor V2 uses a
    /// development feature flag. No upgrade/downgrade fallback or redirect.
    pub fn fetch_store(&self, version: ApiVersion) -> Result<Catalog, TransportError> {
        let mut stream =
            TcpStream::connect_timeout(&self.target, IO_TIMEOUT).map_err(|_| TransportError::Io)?;
        stream
            .set_read_timeout(Some(IO_TIMEOUT))
            .map_err(|_| TransportError::Io)?;
        stream
            .set_write_timeout(Some(IO_TIMEOUT))
            .map_err(|_| TransportError::Io)?;
        let path = match version {
            ApiVersion::V1 => "/store",
            ApiVersion::V2 => "/v2/store",
        };
        // Path, verb, hostname and content type are constants, never caller controlled.
        let request = format!(
            "GET {path} HTTP/1.1\r\nHost: supervisor\r\nAuthorization: Bearer {}\r\nAccept: application/json\r\nConnection: close\r\n\r\n",
            self.token.0
        );
        stream
            .write_all(request.as_bytes())
            .map_err(|_| TransportError::Io)?;
        let body = read_http_json(&mut stream)?;
        let catalog = parse_store_response(&body).map_err(|_| TransportError::InvalidStore)?;
        if catalog.api_version != version {
            return Err(TransportError::UnexpectedApiVersion);
        }
        Ok(catalog)
    }
}

/// Client executes only inside a Supervisor-managed App network.
/// Internal DNS service is fixed to supervisor, never caller-controlled.
/// Explicit laboratory-only service mode: CLI probe exits, a supervised App stays alive.
/// The production daemon will use a real event loop instead of this probe's parking loop.
pub fn app_service_mode(value: Option<&str>) -> bool {
    value == Some("1")
}

pub struct AppSupervisorClient {
    token: SupervisorToken,
}
impl fmt::Debug for AppSupervisorClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AppSupervisorClient { service: supervisor, token: [REDACTED] }")
    }
}
impl AppSupervisorClient {
    /// Read-only official job + postcondition observation; never mutates Apps.
    /// Trusted user identity, operation receipt and polling are caller duties.
    pub fn observe_operation(
        &self,
        version: ApiVersion,
        watch: &crate::operation_reconcile::Watch,
    ) -> Result<crate::operation_reconcile::Observation, TransportError> {
        observe_operation_with(
            version,
            watch,
            |v, id| self.fetch_job(v, id),
            |v| self.fetch_installed(v),
        )
    }

    pub fn new(token: SupervisorToken) -> Self {
        Self { token }
    }

    pub fn from_environment() -> Result<Self, TransportError> {
        let raw = std::env::var("SUPERVISOR_TOKEN").map_err(|_| TransportError::InvalidToken)?;
        Ok(Self::new(SupervisorToken::new(raw)?))
    }

    pub fn service_name(&self) -> &'static str {
        "supervisor"
    }

    /// Read actual running Apps from the official Supervisor service.
    pub fn fetch_installed(&self, version: ApiVersion) -> Result<RunningInventory, TransportError> {
        use std::net::{IpAddr, ToSocketAddrs};
        let path = match version {
            ApiVersion::V1 => "/addons",
            ApiVersion::V2 => "/v2/apps",
        };
        let target = (self.service_name(), 80)
            .to_socket_addrs()
            .map_err(|_| TransportError::Io)?
            .find(|addr| matches!(addr.ip(), IpAddr::V4(ip) if ip.is_private()))
            .ok_or(TransportError::UnsupportedHost)?;
        let body = get_json(target, &self.token, path)?;
        parse_running_inventory(&body, version).map_err(|_| TransportError::InvalidRuntimeInventory)
    }

    /// Read job progress on the authenticated internal Supervisor service.
    pub fn fetch_job(
        &self,
        version: ApiVersion,
        id: &str,
    ) -> Result<crate::job_status::JobStatus, TransportError> {
        use std::net::{IpAddr, ToSocketAddrs};
        let path =
            crate::job_status::job_path(version, id).map_err(|_| TransportError::InvalidJob)?;
        let target = (self.service_name(), 80)
            .to_socket_addrs()
            .map_err(|_| TransportError::Io)?
            .find(|addr| matches!(addr.ip(),IpAddr::V4(ip) if ip.is_private()))
            .ok_or(TransportError::UnsupportedHost)?;
        let body = get_json(target, &self.token, &path)?;
        crate::job_status::parse_job_response(&body, id).map_err(|_| TransportError::InvalidJob)
    }

    /// GET only, against the same internal Supervisor service used for the Store.
    pub fn fetch_app_detail(
        &self,
        version: ApiVersion,
        slug: &str,
    ) -> Result<AppDetail, TransportError> {
        use std::net::{IpAddr, ToSocketAddrs};
        let path = detail_path(version, slug).map_err(|_| TransportError::InvalidAppSlug)?;
        let target = (self.service_name(), 80)
            .to_socket_addrs()
            .map_err(|_| TransportError::Io)?
            .find(|addr| matches!(addr.ip(),IpAddr::V4(ip) if ip.is_private()))
            .ok_or(TransportError::UnsupportedHost)?;
        let body = get_json(target, &self.token, &path)?;
        parse_app_detail(&body, slug).map_err(|_| TransportError::InvalidAppInfo)
    }

    /// Only Docker-internal Supervisor DNS; no arbitrary URI/address accepted.
    pub fn fetch_store(&self, version: ApiVersion) -> Result<Catalog, TransportError> {
        use std::net::{IpAddr, ToSocketAddrs};
        let targets = (self.service_name(), 80)
            .to_socket_addrs()
            .map_err(|_| TransportError::Io)?;
        let target = targets
            .into_iter()
            .find(|addr| match addr.ip() {
                IpAddr::V4(ip) => ip.is_private(),
                IpAddr::V6(_) => false,
            })
            .ok_or(TransportError::UnsupportedHost)?;
        let mut stream =
            TcpStream::connect_timeout(&target, IO_TIMEOUT).map_err(|_| TransportError::Io)?;
        stream
            .set_read_timeout(Some(IO_TIMEOUT))
            .map_err(|_| TransportError::Io)?;
        stream
            .set_write_timeout(Some(IO_TIMEOUT))
            .map_err(|_| TransportError::Io)?;
        let path = match version {
            ApiVersion::V1 => "/store",
            ApiVersion::V2 => "/v2/store",
        };
        let request = format!(
            "GET {path} HTTP/1.1\r\nHost: supervisor\r\nAuthorization: Bearer {}\r\nAccept: application/json\r\nConnection: close\r\n\r\n",
            self.token.0
        );
        stream
            .write_all(request.as_bytes())
            .map_err(|_| TransportError::Io)?;
        let body = read_http_json(&mut stream)?;
        let catalog = parse_store_response(&body).map_err(|_| TransportError::InvalidStore)?;
        if catalog.api_version != version {
            return Err(TransportError::UnexpectedApiVersion);
        }
        Ok(catalog)
    }
}

fn get_json(
    target: SocketAddr,
    token: &SupervisorToken,
    path: &str,
) -> Result<Vec<u8>, TransportError> {
    let mut stream =
        TcpStream::connect_timeout(&target, IO_TIMEOUT).map_err(|_| TransportError::Io)?;
    stream
        .set_read_timeout(Some(IO_TIMEOUT))
        .map_err(|_| TransportError::Io)?;
    stream
        .set_write_timeout(Some(IO_TIMEOUT))
        .map_err(|_| TransportError::Io)?;
    let request = format!(
        "GET {path} HTTP/1.1\r\nHost: supervisor\r\nAuthorization: Bearer {}\r\nAccept: application/json\r\nConnection: close\r\n\r\n",
        token.0
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|_| TransportError::Io)?;
    read_http_json(&mut stream)
}

fn read_http_json(stream: &mut TcpStream) -> Result<Vec<u8>, TransportError> {
    let mut data = Vec::new();
    let mut chunk = [0_u8; 8192];
    let header_end;
    loop {
        if let Some(pos) = data.windows(4).position(|w| w == b"\r\n\r\n") {
            header_end = pos + 4;
            if header_end > MAX_HEADERS {
                return Err(TransportError::TooLarge);
            }
            break;
        }
        if data.len() > MAX_HEADERS {
            return Err(TransportError::TooLarge);
        }
        let count = stream.read(&mut chunk).map_err(|_| TransportError::Io)?;
        if count == 0 {
            return Err(TransportError::BadResponse);
        }
        data.extend_from_slice(&chunk[..count]);
    }
    let head = std::str::from_utf8(&data[..header_end]).map_err(|_| TransportError::BadResponse)?;
    let mut lines = head.split("\r\n");
    let status = lines.next().ok_or(TransportError::BadResponse)?;
    let code = status
        .strip_prefix("HTTP/1.1 ")
        .and_then(|s| s.split(' ').next())
        .and_then(|s| s.parse::<u16>().ok())
        .ok_or(TransportError::BadResponse)?;
    if code == 401 || code == 403 {
        return Err(TransportError::Unauthorized);
    }
    if code != 200 {
        return Err(TransportError::UnexpectedStatus);
    }
    let mut content_length = None;
    let mut has_transfer_encoding = false;
    for line in lines {
        if line.is_empty() {
            continue;
        }
        let (name, value) = line.split_once(':').ok_or(TransportError::BadResponse)?;
        let name = name.trim();
        let value = value.trim();
        if name.eq_ignore_ascii_case("content-length") {
            if content_length.is_some() {
                return Err(TransportError::BadResponse);
            }
            let parsed = value
                .parse::<usize>()
                .map_err(|_| TransportError::BadResponse)?;
            if parsed > MAX_BODY {
                return Err(TransportError::TooLarge);
            }
            content_length = Some(parsed);
        } else if name.eq_ignore_ascii_case("transfer-encoding") {
            has_transfer_encoding = true;
        } else if name.eq_ignore_ascii_case("content-type")
            && !value.to_ascii_lowercase().starts_with("application/json")
        {
            return Err(TransportError::BadResponse);
        }
    }
    if has_transfer_encoding {
        // Deliberately refuse HTTP chunked until a fully tested parser exists.
        return Err(TransportError::BadResponse);
    }
    let Some(expected) = content_length else {
        // Current isolated contract requires explicit length; no EOF framing.
        return Err(TransportError::BadResponse);
    };
    let payload = &mut data[header_end..].to_vec();
    if payload.len() > expected {
        return Err(TransportError::BadResponse);
    }
    while payload.len() < expected {
        let count = stream.read(&mut chunk).map_err(|_| TransportError::Io)?;
        if count == 0 || payload.len() + count > expected {
            return Err(TransportError::BadResponse);
        }
        payload.extend_from_slice(&chunk[..count]);
    }
    Ok(std::mem::take(payload))
}

/// P8g LAB-ONLY code is excluded from default Rust package builds.
/// It can ONLY stop/start core_ssh fixture in disposable QEMU.
#[cfg(feature = "lab-app-actions")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabAction {
    Stop,
    Start,
}

#[cfg(feature = "lab-app-actions")]
pub fn lab_action_path(action: LabAction) -> &'static str {
    match action {
        LabAction::Stop => "/addons/core_ssh/stop",
        LabAction::Start => "/addons/core_ssh/start",
    }
}
#[cfg(feature = "lab-app-actions")]
pub fn lab_mutation_enabled(value: Option<&str>) -> bool {
    value == Some("1")
}
/// Docker stop/start are awaited by Supervisor. GET's 3-second IO timeout
/// is unsuitable: leave the connection timeout at 3s, bound action read to 90s.
#[cfg(feature = "lab-app-actions")]
pub fn lab_action_io_timeout() -> Duration {
    Duration::from_secs(90)
}
#[cfg(feature = "lab-app-actions")]
impl AppSupervisorClient {
    /// Feature-gated disposable-QEMU fixture mutation ONLY; not end-user RBAC.
    pub fn lab_fixture_transition(&self, action: LabAction) -> Result<(), TransportError> {
        use std::net::{IpAddr, ToSocketAddrs};
        if !lab_mutation_enabled(std::env::var("HARS_P8G_QEMU_ONLY").ok().as_deref()) {
            return Err(TransportError::LabMutationDisabled);
        }
        let target = (self.service_name(), 80)
            .to_socket_addrs()
            .map_err(|_| TransportError::Io)?
            .find(|a| matches!(a.ip(), IpAddr::V4(ip) if ip.is_private()))
            .ok_or(TransportError::UnsupportedHost)?;
        let mut stream =
            TcpStream::connect_timeout(&target, IO_TIMEOUT).map_err(|_| TransportError::Io)?;
        stream
            .set_read_timeout(Some(lab_action_io_timeout()))
            .map_err(|_| TransportError::Io)?;
        stream
            .set_write_timeout(Some(IO_TIMEOUT))
            .map_err(|_| TransportError::Io)?;
        let request = format!(
            "POST {} HTTP/1.1\r\nHost: supervisor\r\nAuthorization: Bearer {}\r\nContent-Length: 0\r\nAccept: application/json\r\nConnection: close\r\n\r\n",
            lab_action_path(action),
            self.token.0
        );
        stream
            .write_all(request.as_bytes())
            .map_err(|_| TransportError::Io)?;
        let body = read_http_json(&mut stream)?;
        let response = serde_json::from_slice::<crate::NoDuplicates>(&body)
            .map_err(|_| TransportError::BadResponse)?
            .0;
        if response.get("result").and_then(serde_json::Value::as_str) != Some("ok") {
            return Err(TransportError::BadResponse);
        }
        Ok(())
    }
}

// Read-only Supervisor install preflight. Genuine Supervisor remains the
// authority for current OS/architecture/version eligibility; never trust the
// local catalogue or a user-supplied compatible=true flag.
#[cfg(feature = "managed-apps-preview")]
fn parse_install_availability_response(raw: &[u8]) -> Result<(), TransportError> {
    let response = serde_json::from_slice::<crate::NoDuplicates>(raw)
        .map_err(|_| TransportError::BadResponse)?
        .0;
    if response.get("result").and_then(serde_json::Value::as_str) != Some("ok") {
        return Err(TransportError::BadResponse);
    }
    let data = response.get("data").ok_or(TransportError::BadResponse)?;
    let obj = data.as_object().ok_or(TransportError::BadResponse)?;
    if let Some(flag) = obj.get("available")
        && flag.as_bool() != Some(true)
    {
        return Err(TransportError::BadResponse);
    }
    Ok(())
}

#[cfg(feature = "managed-apps-preview")]
impl AppSupervisorClient {
    fn require_install_available(&self, app: &str) -> Result<(), TransportError> {
        use std::net::{IpAddr, ToSocketAddrs};
        // AppSlug was validated by the typed PreparedRequest constructor.
        let target = (self.service_name(), 80)
            .to_socket_addrs()
            .map_err(|_| TransportError::Io)?
            .find(|a| matches!(a.ip(), IpAddr::V4(ip) if ip.is_private()))
            .ok_or(TransportError::UnsupportedHost)?;
        let mut stream =
            TcpStream::connect_timeout(&target, IO_TIMEOUT).map_err(|_| TransportError::Io)?;
        stream
            .set_read_timeout(Some(IO_TIMEOUT))
            .map_err(|_| TransportError::Io)?;
        stream
            .set_write_timeout(Some(IO_TIMEOUT))
            .map_err(|_| TransportError::Io)?;
        let path = format!("/store/addons/{app}/availability");
        let wire = format!(
            "GET {path} HTTP/1.1\r\nHost: supervisor\r\nAuthorization: Bearer {}\r\nAccept: application/json\r\nConnection: close\r\n\r\n",
            self.token.0
        );
        stream
            .write_all(wire.as_bytes())
            .map_err(|_| TransportError::Io)?;
        let raw = read_http_json(&mut stream)?;
        parse_install_availability_response(&raw)
    }
}

/// Verify official V1 backup metadata has the exact requested App.
/// This does not prove that tar contents are restorable or equivalent.
#[cfg(feature = "managed-apps-preview")]
fn parse_backup_preflight_response(
    raw: &[u8],
    backup: &str,
    app: &str,
) -> Result<(), TransportError> {
    let reply = serde_json::from_slice::<crate::NoDuplicates>(raw)
        .map_err(|_| TransportError::BadResponse)?
        .0;
    if reply.get("result").and_then(serde_json::Value::as_str) != Some("ok") {
        return Err(TransportError::BadResponse);
    }
    let data = reply
        .get("data")
        .and_then(serde_json::Value::as_object)
        .ok_or(TransportError::BadResponse)?;
    if data.get("slug").and_then(serde_json::Value::as_str) != Some(backup)
        || data.get("protected").and_then(serde_json::Value::as_bool) != Some(false)
    {
        return Err(TransportError::BadResponse);
    }
    let entries = data
        .get("addons")
        .and_then(serde_json::Value::as_array)
        .ok_or(TransportError::BadResponse)?;
    let mut matches = 0;
    for item in entries {
        if item.get("slug").and_then(serde_json::Value::as_str) == Some(app) {
            matches += 1;
            let version = item.get("version").and_then(serde_json::Value::as_str);
            if !version.is_some_and(|s| {
                !s.is_empty() && s.len() <= 128 && !s.chars().any(char::is_control)
            }) {
                return Err(TransportError::BadResponse);
            }
        }
    }
    if matches != 1 {
        return Err(TransportError::BadResponse);
    }
    Ok(())
}

#[cfg(feature = "managed-apps-preview")]
impl AppSupervisorClient {
    fn require_backup_contains_app(&self, backup: &str, app: &str) -> Result<(), TransportError> {
        use std::net::{IpAddr, ToSocketAddrs};
        let target = (self.service_name(), 80)
            .to_socket_addrs()
            .map_err(|_| TransportError::Io)?
            .find(|a| matches!(a.ip(), IpAddr::V4(ip) if ip.is_private()))
            .ok_or(TransportError::UnsupportedHost)?;
        let mut stream =
            TcpStream::connect_timeout(&target, IO_TIMEOUT).map_err(|_| TransportError::Io)?;
        stream
            .set_read_timeout(Some(IO_TIMEOUT))
            .map_err(|_| TransportError::Io)?;
        stream
            .set_write_timeout(Some(IO_TIMEOUT))
            .map_err(|_| TransportError::Io)?;
        let path = format!("/backups/{backup}/info");
        let wire = format!(
            "GET {path} HTTP/1.1\r\nHost: supervisor\r\nAuthorization: Bearer {}\r\nAccept: application/json\r\nConnection: close\r\n\r\n",
            self.token.0
        );
        stream
            .write_all(wire.as_bytes())
            .map_err(|_| TransportError::Io)?;
        let raw = read_http_json(&mut stream)?;
        parse_backup_preflight_response(&raw, backup, app)
    }
}

#[cfg(feature = "managed-apps-preview")]
#[derive(Debug, Clone, PartialEq, Eq)]
struct UpdatePlan {
    previous: String,
    expected: String,
}

#[cfg(feature = "managed-apps-preview")]
fn parse_installed_app_info(
    raw: &[u8],
    app: &str,
) -> Result<(String, String, bool), TransportError> {
    let response = serde_json::from_slice::<crate::NoDuplicates>(raw)
        .map_err(|_| TransportError::BadResponse)?
        .0;
    if response.get("result").and_then(serde_json::Value::as_str) != Some("ok") {
        return Err(TransportError::BadResponse);
    }
    let data = response
        .get("data")
        .and_then(serde_json::Value::as_object)
        .ok_or(TransportError::BadResponse)?;
    // Official GET /addons/<app>/info already rejects uninstalled Apps before
    // serializing its info_data, which has NO "installed" field (Supervisor
    // upstream api/apps.py). If an installed flag appears, never trust false
    // or a nonboolean value. Only this fixed installed-App endpoint is used.
    if data.get("slug").and_then(serde_json::Value::as_str) != Some(app) {
        return Err(TransportError::BadResponse);
    }
    if let Some(installed) = data.get("installed")
        && installed.as_bool() != Some(true)
    {
        return Err(TransportError::BadResponse);
    }
    let version = |k: &str| -> Result<String, TransportError> {
        data.get(k)
            .and_then(serde_json::Value::as_str)
            .filter(|v| !v.is_empty() && v.len() <= 128 && !v.chars().any(char::is_control))
            .map(str::to_owned)
            .ok_or(TransportError::BadResponse)
    };
    let available = data
        .get("update_available")
        .and_then(serde_json::Value::as_bool)
        .ok_or(TransportError::BadResponse)?;
    Ok((version("version")?, version("version_latest")?, available))
}
#[cfg(feature = "managed-apps-preview")]
fn parse_update_preflight_response(raw: &[u8], app: &str) -> Result<UpdatePlan, TransportError> {
    let (previous, expected, available) = parse_installed_app_info(raw, app)?;
    if !available || previous == expected {
        return Err(TransportError::BadResponse);
    }
    Ok(UpdatePlan { previous, expected })
}
#[cfg(feature = "managed-apps-preview")]
fn verify_update_postflight(
    raw: &[u8],
    app: &str,
    plan: &UpdatePlan,
) -> Result<(), TransportError> {
    let (version, _, available) = parse_installed_app_info(raw, app)?;
    if available || version != plan.expected || version == plan.previous {
        return Err(TransportError::BadResponse);
    }
    Ok(())
}
#[cfg(feature = "managed-apps-preview")]
impl AppSupervisorClient {
    fn fetch_installed_app_info(&self, app: &str) -> Result<Vec<u8>, TransportError> {
        use std::net::{IpAddr, ToSocketAddrs};
        let target = (self.service_name(), 80)
            .to_socket_addrs()
            .map_err(|_| TransportError::Io)?
            .find(|addr| matches!(addr.ip(), IpAddr::V4(ip) if ip.is_private()))
            .ok_or(TransportError::UnsupportedHost)?;
        let mut stream =
            TcpStream::connect_timeout(&target, IO_TIMEOUT).map_err(|_| TransportError::Io)?;
        stream
            .set_read_timeout(Some(IO_TIMEOUT))
            .map_err(|_| TransportError::Io)?;
        stream
            .set_write_timeout(Some(IO_TIMEOUT))
            .map_err(|_| TransportError::Io)?;
        let wire = format!(
            "GET /addons/{app}/info HTTP/1.1\r\nHost: supervisor\r\nAuthorization: Bearer {}\r\nAccept: application/json\r\nConnection: close\r\n\r\n",
            self.token.0
        );
        stream
            .write_all(wire.as_bytes())
            .map_err(|_| TransportError::Io)?;
        read_http_json(&mut stream)
    }
    fn require_update_available(&self, app: &str) -> Result<UpdatePlan, TransportError> {
        parse_update_preflight_response(&self.fetch_installed_app_info(app)?, app)
    }
    pub fn preview_probe_update_eligibility(
        &self,
        app: &crate::managed_apps::AppSlug,
    ) -> Result<(), TransportError> {
        if std::env::var("HARS_P8H_QEMU_ONLY").ok().as_deref() != Some("1") {
            return Err(TransportError::ManagedPreviewDisabled);
        }
        self.require_update_available(app.as_str()).map(|_| ())
    }
}

// IMPORTANT: This PREVIEW transport is *compiled out by default* and it is
// NOT an end-user authorization implementation. It can only run in the
// explicit disposable QEMU test App, and requires external approval too.
#[cfg(feature = "managed-apps-preview")]
impl AppSupervisorClient {
    /// LAB ONLY: the sole write path requires a consumed, audit-backed permit.
    /// There is no boolean authorization bypass here. P8i still needs trusted
    /// authentication, revocation and a production-grade audit backend.
    pub fn preview_execute(
        &self,
        permit: crate::audit_lab::AuditedPermit,
    ) -> Result<serde_json::Value, TransportError> {
        use std::net::{IpAddr, ToSocketAddrs};
        if std::env::var("HARS_P8H_QEMU_ONLY").ok().as_deref() != Some("1") {
            return Err(TransportError::ManagedPreviewDisabled);
        }
        let request = permit.request();
        if request.body().len() > 64 * 1024 {
            return Err(TransportError::TooLarge);
        }
        // Fail closed BEFORE any mutating POST on incompatibility, upstream
        // denial or a malformed availability response.
        if request.action() == crate::managed_apps::Action::Install {
            self.require_install_available(request.target())?;
        }
        if let Some(backup) = request.restore_backup_slug() {
            self.require_backup_contains_app(backup, request.target())?;
        }
        // Supervisor only updates to latest, regardless of requested version.
        // Refuse no-op and never disable its pre-update backup.
        let update_plan = if request.action() == crate::managed_apps::Action::Update {
            let body = serde_json::from_slice::<crate::NoDuplicates>(request.body())
                .map_err(|_| TransportError::BadResponse)?
                .0;
            if body.get("backup").and_then(serde_json::Value::as_bool) != Some(true) {
                return Err(TransportError::BadResponse);
            }
            Some(self.require_update_available(request.target())?)
        } else {
            None
        };
        let target = (self.service_name(), 80)
            .to_socket_addrs()
            .map_err(|_| TransportError::Io)?
            .find(|a| matches!(a.ip(), IpAddr::V4(ip) if ip.is_private()))
            .ok_or(TransportError::UnsupportedHost)?;
        let mut stream =
            TcpStream::connect_timeout(&target, IO_TIMEOUT).map_err(|_| TransportError::Io)?;
        stream
            .set_read_timeout(Some(Duration::from_secs(120)))
            .map_err(|_| TransportError::Io)?;
        stream
            .set_write_timeout(Some(IO_TIMEOUT))
            .map_err(|_| TransportError::Io)?;
        // Both verb and path originate exclusively from validated static
        // operation definitions; operator input never forms arbitrary URLs.
        let body = request.body();
        let wire_head = format!(
            "{} {} HTTP/1.1\r\nHost: supervisor\r\nAuthorization: Bearer {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nAccept: application/json\r\nConnection: close\r\n\r\n",
            request.method(),
            request.path(),
            self.token.0,
            body.len()
        );
        stream
            .write_all(wire_head.as_bytes())
            .map_err(|_| TransportError::Io)?;
        stream.write_all(body).map_err(|_| TransportError::Io)?;
        let response = read_http_json(&mut stream)?;
        let parsed = serde_json::from_slice::<crate::NoDuplicates>(&response)
            .map_err(|_| TransportError::BadResponse)?
            .0;
        if parsed.get("result").and_then(serde_json::Value::as_str) != Some("ok") {
            return Err(TransportError::BadResponse);
        }
        if let Some(plan) = update_plan {
            let updated = self.fetch_installed_app_info(request.target())?;
            verify_update_postflight(&updated, request.target(), &plan)?;
        }
        // The response can contain secrets/options. Never print or log it.
        Ok(parsed
            .get("data")
            .cloned()
            .unwrap_or(serde_json::Value::Null))
    }
}

#[cfg(all(test, feature = "managed-apps-preview"))]
mod p8j_update_preflight_tests {
    use super::*;
    const READY: &[u8] = br#"{"result":"ok","data":{"slug":"core_ssh","installed":true,"version":"9.0.0","version_latest":"9.1.0","update_available":true}}"#;
    #[test]
    fn requires_real_installed_version_difference_and_official_update_flag() {
        let plan = parse_update_preflight_response(READY, "core_ssh").unwrap();
        assert_eq!(plan.previous, "9.0.0");
        assert_eq!(plan.expected, "9.1.0");
        // Match genuine official Supervisor GET /addons/<app>/info:
        // installed is validated by the endpoint, not returned as a field.
        let upstream = br#"{"result":"ok","data":{"slug":"core_ssh","version":"9.0.0","version_latest":"9.1.0","update_available":true}}"#;
        assert_eq!(
            parse_update_preflight_response(upstream, "core_ssh").unwrap(),
            plan
        );
        let invalid = [
            br#"{"result":"ok","data":{"slug":"core_ssh","installed":true,"version":"9.0.0","version_latest":"9.0.0","update_available":true}}"#.as_slice(),
            br#"{"result":"ok","data":{"slug":"core_ssh","installed":true,"version":"9.0.0","version_latest":"9.1.0","update_available":false}}"#,
            br#"{"result":"ok","data":{"slug":"core_ssh","installed":false,"version":"9.0.0","version_latest":"9.1.0","update_available":true}}"#,
            br#"{"result":"ok","data":{"slug":"evil_app","installed":true,"version":"9.0.0","version_latest":"9.1.0","update_available":true}}"#,
            br#"{"result":"ok","data":{"slug":"core_ssh","installed":true,"version":"9.0.0","version_latest":"9.1.0","update_available":"true"}}"#,
            br#"{"result":"ok","data":{"slug":"core_ssh","installed":"true","version":"9.0.0","version_latest":"9.1.0","update_available":true}}"#,
            br#"{"result":"ok","data":{"slug":"core_ssh","installed":true,"version":"9.0.0","version_latest":"9.1.0"}}"#,
            br#"{"result":"ok","data":{"slug":"core_ssh","installed":true,"version":"","version_latest":"9.1.0","update_available":true}}"#,
            br#"{"result":"ok","data":{"slug":"core_ssh","installed":true,"version":"9.0.0","version_latest":"9.1.0","update_available":true,"update_available":false}}"#,
            br#"{"result":"error","data":{}}"#,
            b"garbage",
        ];
        for response in invalid {
            assert!(
                parse_update_preflight_response(response, "core_ssh").is_err(),
                "invalid update preflight was accepted"
            );
        }
    }
    #[test]
    fn postflight_requires_exact_version_and_no_updates_pending() {
        let plan = parse_update_preflight_response(READY, "core_ssh").unwrap();
        let valid=br#"{"result":"ok","data":{"slug":"core_ssh","installed":true,"version":"9.1.0","version_latest":"9.1.0","update_available":false}}"#;
        assert!(verify_update_postflight(valid, "core_ssh", &plan).is_ok());
        for response in [
            READY,
            br#"{"result":"ok","data":{"slug":"core_ssh","installed":true,"version":"9.2.0","version_latest":"9.2.0","update_available":false}}"#.as_slice(),
            br#"{"result":"ok","data":{"slug":"core_ssh","installed":true,"version":"9.1.0","version_latest":"9.1.0","update_available":true}}"#,
            br#"{"result":"ok","data":{"slug":"core_ssh","installed":false,"version":"9.1.0","version_latest":"9.1.0","update_available":false}}"#,
        ] {
            assert!(verify_update_postflight(response,"core_ssh",&plan).is_err());
        }
    }
}

#[cfg(all(test, feature = "managed-apps-preview"))]
mod p8j_backup_preflight_tests {
    use super::*;

    const VALID: &[u8] = br#"{"result":"ok","data":{"slug":"abc12345","protected":false,"type":"partial","addons":[{"slug":"core_ssh","name":"Terminal","version":"9.1.0","size":1.0}]}}"#;

    #[test]
    fn only_exact_unencrypted_backup_with_named_app_and_version_is_accepted() {
        assert!(parse_backup_preflight_response(VALID, "abc12345", "core_ssh").is_ok());
        assert!(parse_backup_preflight_response(VALID, "deadbeef", "core_ssh").is_err());
        assert!(parse_backup_preflight_response(VALID, "abc12345", "different").is_err());
        for raw in [
            br#"{"result":"error","data":{}}"#.as_slice(),
            br#"{"result":"ok","data":{"slug":"abc12345","protected":true,"addons":[{"slug":"core_ssh","version":"9"}]}}"#,
            br#"{"result":"ok","data":{"slug":"abc12345","protected":false,"addons":[]}}"#,
            br#"{"result":"ok","data":{"slug":"abc12345","protected":false,"addons":[{"slug":"core_ssh","version":""}]}}"#,
            br#"{"result":"ok","data":{"slug":"abc12345","protected":false,"addons":[{"slug":"core_ssh","version":"9"},{"slug":"core_ssh","version":"8"}]}}"#,
            br#"{"result":"ok","data":{"slug":"abc12345","protected":false,"addons":[{"slug":"core_ssh","version":"9"}],"addons":[]}}"#,
            br#"{"result":"ok","data":{"slug":"abc12345","addons":[{"slug":"core_ssh","version":"9"}]}}"#,
            b"bad json",
        ] {
            assert!(parse_backup_preflight_response(raw, "abc12345", "core_ssh").is_err());
        }
    }
}

#[cfg(all(test, feature = "managed-apps-preview"))]
mod p8j_availability_tests {
    use super::*;

    #[test]
    fn install_preflight_rejects_denials_missing_result_and_duplicate_fields() {
        assert!(parse_install_availability_response(br#"{"result":"ok","data":{}}"#).is_ok());
        assert!(
            parse_install_availability_response(br#"{"result":"ok","data":{"available":true}}"#)
                .is_ok()
        );
        for raw in [
            br#"{"result":"error","message":"unsupported architecture"}"#.as_slice(),
            br#"{"result":"ok","data":{"available":false}}"#,
            br#"{"result":"ok","result":"error"}"#,
            br#"{"data":{}}"#,
            br#"{"result":"ok","data":{"available":"true"}}"#,
            b"not-json",
        ] {
            assert!(parse_install_availability_response(raw).is_err());
        }
    }
}
