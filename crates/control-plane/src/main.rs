//! Rusty Home P3b prototype: read-only, multi-source application inventory.
//! No Docker socket exposed, no write paths, no HAOS device control or automation.
mod attestation;
mod ingress;
use hars_supervisor_apps::app_detail::valid_slug;
use hars_supervisor_apps::installed_inventory::RunningInventory;
use hars_supervisor_apps::official_store::ApiVersion;
use hars_supervisor_apps::official_transport::AppSupervisorClient;
use serde_json::{json, Value};
use std::env;
use std::fs;
use std::io::{Read, Write};
use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream};
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
enum AdapterKind {
    DockerCli,
    DockerFixture(String),
    SupervisorSnapshot(String),
    SupervisorLive(ApiVersion),
}

#[derive(Clone, Debug)]
struct Adapter {
    host: String,
    kind: AdapterKind,
}

trait HostAdapter {
    fn host(&self) -> &str;
    fn kind(&self) -> &'static str;
    fn collect(&self) -> Result<Vec<Value>, &'static str>;
}

impl HostAdapter for Adapter {
    fn host(&self) -> &str {
        &self.host
    }

    fn kind(&self) -> &'static str {
        match self.kind {
            AdapterKind::DockerCli | AdapterKind::DockerFixture(_) => "docker",
            AdapterKind::SupervisorSnapshot(_) => "supervisor-snapshot",
            AdapterKind::SupervisorLive(_) => "supervisor-live",
        }
    }

    fn collect(&self) -> Result<Vec<Value>, &'static str> {
        match &self.kind {
            AdapterKind::DockerCli => {
                // Fixed command only: no shell, arbitrary commands or Docker mutations.
                let output = Command::new("docker")
                    .args(["ps", "--all", "--format", "{{json .}}"])
                    .output()
                    .map_err(|_| "docker CLI unavailable")?;
                if !output.status.success() {
                    return Err("Docker inventory unavailable");
                }
                let body = String::from_utf8(output.stdout).map_err(|_| "invalid Docker output")?;
                docker_rows(&body, &self.host)
            }
            AdapterKind::DockerFixture(path) => {
                let body = fs::read_to_string(path).map_err(|_| "Docker fixture unavailable")?;
                docker_rows(&body, &self.host)
            }
            AdapterKind::SupervisorSnapshot(path) => {
                let body =
                    fs::read_to_string(path).map_err(|_| "Supervisor snapshot unavailable")?;
                supervisor_rows(&body, &self.host)
            }
            AdapterKind::SupervisorLive(version) => {
                // Fixed internal Supervisor service; never caller-chosen host.
                let client = AppSupervisorClient::from_environment()
                    .map_err(|_| "Supervisor live authentication unavailable")?;
                let installed = client
                    .fetch_installed(*version)
                    .map_err(|_| "Supervisor installed-App API unavailable")?;
                Ok(supervisor_live_rows(&installed, &self.host))
            }
        }
    }
}

fn valid_host(host: &str) -> bool {
    !host.is_empty()
        && host.len() <= 40
        && host
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

fn bounded(s: &str) -> String {
    s.chars().take(180).collect()
}

fn field<'a>(v: &'a Value, key: &str) -> &'a str {
    v.get(key).and_then(Value::as_str).unwrap_or("")
}

fn docker_rows(body: &str, host: &str) -> Result<Vec<Value>, &'static str> {
    body.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let row: Value = serde_json::from_str(line).map_err(|_| "invalid Docker row")?;
            let name = field(&row, "Names");
            if name.is_empty() {
                return Err("container missing name");
            }
            let image = field(&row, "Image");
            Ok(json!({
                "id": format!("{host}:docker:{}", bounded(name)),
                "host": host,
                "display_name": bounded(name),
                "image": bounded(image),
                "state": bounded(field(&row, "State")),
                "version": Value::Null,
                "update_available": Value::Null,
                "app_type": "generic_container",
                "classification": "docker",
                "backend": "docker",
                "managed_by_bridge": false,
                "capabilities": ["view_status"]
            }))
        })
        .collect()
}

fn supervisor_rows(body: &str, host: &str) -> Result<Vec<Value>, &'static str> {
    let doc: Value = serde_json::from_str(body).map_err(|_| "invalid Supervisor JSON")?;
    // Supports a sanitized snapshot, real Supervisor API {data:{addons}},
    // and a direct Supervisor {addons} payload. Never emit options or secrets.
    let items = doc
        .get("apps")
        .or_else(|| doc.get("addons"))
        .or_else(|| doc.get("data").and_then(|data| data.get("addons")))
        .or_else(|| doc.get("data").and_then(|data| data.get("apps")))
        .and_then(Value::as_array)
        .ok_or("Supervisor apps array missing")?;
    items
        .iter()
        .map(|item| {
            let name = field(item, "name");
            let slug = field(item, "slug");
            if name.is_empty() || slug.is_empty() {
                return Err("invalid Supervisor app metadata");
            }
            Ok(json!({
                "id": format!("{host}:supervisor:{}", bounded(slug)),
                "host": host,
                "display_name": bounded(name),
                "image": Value::Null,
                "state": bounded(field(item, "state")),
                "version": bounded(field(item, "version")),
                "update_available": item.get("update_available").and_then(Value::as_bool),
                "app_type": "supervisor_app",
                "classification": "supervisor",
                "backend": "supervisor",
                "managed_by_bridge": false,
                "capabilities": ["view_status"]
            }))
        })
        .collect()
}

/// Typed official Supervisor inventory -> unprivileged frontend presentation.
/// Never expose credentials, options, network addresses or write capabilities.
fn supervisor_live_rows(inventory: &RunningInventory, host: &str) -> Vec<Value> {
    inventory
        .apps
        .iter()
        .map(|app| {
            json!({
                "id": format!("{host}:supervisor:{}", app.slug),
                "host": host,
                "slug": app.slug,
                "live": true,
                "display_name": bounded(&app.name),
                "image": Value::Null,
                "state": app.state.as_deref().unwrap_or("unknown"),
                "version": app.version,
                "update_available": app.update_available,
                "app_type": "supervisor_app",
                "classification": "supervisor",
                "backend": "supervisor",
                "managed_by_bridge": false,
                "capabilities": ["view_status"]
            })
        })
        .collect()
}

/// Route only ASCII Supervisor slugs. Encoded separators, traversal and
/// unbounded paths must never reach the privileged internal Supervisor API.
fn live_detail_slug(line: &str) -> Option<Result<&str, &'static str>> {
    let rest = line.strip_prefix("GET /api/apps/")?;
    let slug = rest
        .strip_suffix("/detail HTTP/1.1")
        .or_else(|| rest.strip_suffix("/detail HTTP/1.0"))?;
    Some(if valid_slug(slug) {
        Ok(slug)
    } else {
        Err("invalid app slug")
    })
}

/// Only official authenticated installed App schema, not configured values.
fn live_config_slug(line: &str) -> Option<Result<&str, &'static str>> {
    let rest = line.strip_prefix("GET /api/apps/")?;
    let slug = rest
        .strip_suffix("/config-schema HTTP/1.1")
        .or_else(|| rest.strip_suffix("/config-schema HTTP/1.0"))?;
    Some(if valid_slug(slug) {
        Ok(slug)
    } else {
        Err("invalid app slug")
    })
}

/// The server never returns configured secrets, even to a valid Ingress user.
/// Confirm exactly one live Supervisor and current installed inventory first.
fn live_config_response(adapters: &[Adapter], slug: &str) -> (&'static str, String) {
    if !valid_slug(slug) {
        return (
            "400 Bad Request",
            json!({"error":"invalid app slug","read_only":true}).to_string(),
        );
    }
    let mut versions = adapters.iter().filter_map(|a| match a.kind {
        AdapterKind::SupervisorLive(v) => Some(v),
        _ => None,
    });
    let Some(version) = versions.next() else {
        return (
            "404 Not Found",
            json!({"error":"live Supervisor unavailable","read_only":true}).to_string(),
        );
    };
    if versions.next().is_some() {
        return (
            "503 Service Unavailable",
            json!({"error":"ambiguous Supervisor source","read_only":true}).to_string(),
        );
    }
    let result = AppSupervisorClient::from_environment().and_then(|client| {
        let inventory = client.fetch_installed(version)?;
        if !inventory.read_only || inventory.api_version != version {
            return Err(
                hars_supervisor_apps::official_transport::TransportError::InvalidRuntimeInventory,
            );
        }
        let matches = inventory.apps.iter().filter(|x| x.slug == slug).count();
        if matches == 0 {
            return Ok(None);
        }
        if matches != 1 {
            return Err(
                hars_supervisor_apps::official_transport::TransportError::InvalidRuntimeInventory,
            );
        }
        client.fetch_app_config_schema(version, slug).map(Some)
    });
    match result {
        Ok(Some(schema)) => (
            "200 OK",
            json!({"read_only":true,"configuration":schema}).to_string(),
        ),
        Ok(None) => (
            "404 Not Found",
            json!({"error":"app not installed","read_only":true}).to_string(),
        ),
        Err(_) => (
            "503 Service Unavailable",
            json!({"error":"Supervisor App schema unavailable","read_only":true}).to_string(),
        ),
    }
}

/// App release notes are never taken from a snapshot or a user-selected URL.
fn live_changelog_slug(line: &str) -> Option<Result<&str, &'static str>> {
    let rest = line.strip_prefix("GET /api/apps/")?;
    let slug = rest
        .strip_suffix("/changelog HTTP/1.1")
        .or_else(|| rest.strip_suffix("/changelog HTTP/1.0"))?;
    Some(if valid_slug(slug) {
        Ok(slug)
    } else {
        Err("invalid app slug")
    })
}

/// Release notes are informational. Confirm the installed App from the
/// current authenticated Supervisor inventory before requesting them.
fn live_changelog_response(adapters: &[Adapter], slug: &str) -> (&'static str, String) {
    if !valid_slug(slug) {
        return (
            "400 Bad Request",
            json!({"error":"invalid app slug","read_only":true}).to_string(),
        );
    }
    let mut versions = adapters.iter().filter_map(|a| match a.kind {
        AdapterKind::SupervisorLive(v) => Some(v),
        _ => None,
    });
    let Some(version) = versions.next() else {
        return (
            "404 Not Found",
            json!({"error":"live Supervisor source not configured","read_only":true}).to_string(),
        );
    };
    if versions.next().is_some() {
        return (
            "503 Service Unavailable",
            json!({"error":"ambiguous live Supervisor source","read_only":true}).to_string(),
        );
    }
    let result = AppSupervisorClient::from_environment().and_then(|client| {
        let inventory = client.fetch_installed(version)?;
        let matches = inventory.apps.iter().filter(|app| app.slug == slug).count();
        if matches == 0 {
            return Ok(None);
        }
        if matches != 1 {
            return Err(
                hars_supervisor_apps::official_transport::TransportError::InvalidRuntimeInventory,
            );
        }
        client.fetch_app_changelog(version, slug).map(Some)
    });
    match result {
        Ok(Some(notes)) => (
            "200 OK",
            json!({"read_only":true,"changelog":notes}).to_string(),
        ),
        Ok(None) => (
            "404 Not Found",
            json!({"error":"app not installed","read_only":true}).to_string(),
        ),
        Err(_) => (
            "503 Service Unavailable",
            json!({"error":"Supervisor App changelog unavailable","read_only":true}).to_string(),
        ),
    }
}

/// Read-only numeric telemetry for installed Apps from official Supervisor.
fn live_stats_slug(line: &str) -> Option<Result<&str, &'static str>> {
    let rest = line.strip_prefix("GET /api/apps/")?;
    let slug = rest
        .strip_suffix("/stats HTTP/1.1")
        .or_else(|| rest.strip_suffix("/stats HTTP/1.0"))?;
    Some(if valid_slug(slug) {
        Ok(slug)
    } else {
        Err("invalid app slug")
    })
}

/// Read-only official Supervisor Job lookup. A job UUID is NOT an App
/// authorization receipt and must never authorize lifecycle mutations.
fn live_job_id(line: &str) -> Option<Result<&str, &'static str>> {
    let rest = line.strip_prefix("GET /api/jobs/")?;
    let id = rest
        .strip_suffix(" HTTP/1.1")
        .or_else(|| rest.strip_suffix(" HTTP/1.0"))?;
    Some(
        if hars_supervisor_apps::job_status::job_path(ApiVersion::V1, id).is_ok() {
            Ok(id)
        } else {
            Err("invalid Supervisor job id")
        },
    )
}

/// Never expose raw Supervisor job stages, errors, extra metadata or children:
/// they may contain credentials, paths, and details from other operations.
fn public_job_summary(job: &hars_supervisor_apps::job_status::JobStatus) -> Option<Value> {
    if !job.read_only
        || hars_supervisor_apps::job_status::job_path(ApiVersion::V1, &job.id).is_err()
        || !job.progress.is_finite()
        || !(0.0..=100.0).contains(&job.progress)
        || job.error_count > 4096
    {
        return None;
    }
    Some(json!({
        "id":job.id,
        "state":job.state,
        "progress":job.progress,
        "error_count":job.error_count,
        "read_only":true
    }))
}

/// Official Supervisor is the only data source; never use a local fixture,
/// cached job, user-supplied endpoint or ambiguous V1/V2 source.
fn live_job_response(
    adapters: &[Adapter],
    id: &str,
    mode: AccessMode,
    has_verified_identity: bool,
    is_admin: bool,
) -> (&'static str, String) {
    // A job UUID is not a bearer capability. Until durable user-bound
    // operation receipts exist, only the current verified Supervisor
    // administrator may inspect arbitrary jobs, including by direct URL.
    if mode != AccessMode::SupervisorIngress {
        return (
            "404 Not Found",
            json!({"read_only":true,"error":"Supervisor ingress required"}).to_string(),
        );
    }
    if !has_verified_identity || !is_admin {
        return (
            "403 Forbidden",
            json!({"read_only":true,"error":"Supervisor administrator required"}).to_string(),
        );
    }
    if hars_supervisor_apps::job_status::job_path(ApiVersion::V1, id).is_err() {
        return (
            "400 Bad Request",
            json!({"read_only":true,"error":"invalid job id"}).to_string(),
        );
    }
    let mut versions = adapters.iter().filter_map(|a| match a.kind {
        AdapterKind::SupervisorLive(v) => Some(v),
        _ => None,
    });
    let Some(version) = versions.next() else {
        return (
            "404 Not Found",
            json!({"read_only":true,"error":"live Supervisor source not configured"}).to_string(),
        );
    };
    if versions.next().is_some() {
        return (
            "503 Service Unavailable",
            json!({"read_only":true,"error":"ambiguous live Supervisor source"}).to_string(),
        );
    }
    let result =
        AppSupervisorClient::from_environment().and_then(|client| client.fetch_job(version, id));
    match result {
        Ok(job) => match public_job_summary(&job) {
            Some(summary) => (
                "200 OK",
                json!({"read_only":true,"job":summary}).to_string(),
            ),
            None => (
                "503 Service Unavailable",
                json!({"read_only":true,"error":"invalid Supervisor job"}).to_string(),
            ),
        },
        Err(_) => (
            "503 Service Unavailable",
            json!({"read_only":true,"error":"Supervisor job unavailable"}).to_string(),
        ),
    }
}

/// Enumerate real Supervisor jobs only for a verified Supervisor Ingress
/// administrator. This is a read-only discovery convenience, NOT a write
/// permit. No raw job names, references, stages, errors or metadata escape.
fn live_jobs_response(
    adapters: &[Adapter],
    mode: AccessMode,
    has_verified_identity: bool,
    is_admin: bool,
) -> (&'static str, String) {
    if mode != AccessMode::SupervisorIngress {
        return (
            "404 Not Found",
            json!({"read_only":true,"error":"Supervisor ingress required"}).to_string(),
        );
    }
    if !has_verified_identity || !is_admin {
        return (
            "403 Forbidden",
            json!({"read_only":true,"error":"Supervisor administrator required"}).to_string(),
        );
    }
    let mut versions = adapters.iter().filter_map(|a| match a.kind {
        AdapterKind::SupervisorLive(v) => Some(v),
        _ => None,
    });
    let Some(version) = versions.next() else {
        return (
            "404 Not Found",
            json!({"read_only":true,"error":"live Supervisor source not configured"}).to_string(),
        );
    };
    if versions.next().is_some() {
        return (
            "503 Service Unavailable",
            json!({"read_only":true,"error":"ambiguous live Supervisor source"}).to_string(),
        );
    }
    match AppSupervisorClient::from_environment().and_then(|client| client.fetch_jobs(version)) {
        Ok(list) => (
            "200 OK",
            json!({"read_only":true,"jobs":list.jobs,"truncated":list.truncated}).to_string(),
        ),
        Err(_) => (
            "503 Service Unavailable",
            json!({"read_only":true,"error":"Supervisor job index unavailable"}).to_string(),
        ),
    }
}

/// A live inventory must confirm installation before any privileged stats GET.
/// Snapshot sources never trigger network reads and cannot impersonate live.
fn live_stats_response(adapters: &[Adapter], slug: &str) -> (&'static str, String) {
    if !valid_slug(slug) {
        return (
            "400 Bad Request",
            json!({"error":"invalid app slug","read_only":true}).to_string(),
        );
    }
    let mut versions = adapters.iter().filter_map(|a| match a.kind {
        AdapterKind::SupervisorLive(v) => Some(v),
        _ => None,
    });
    let Some(version) = versions.next() else {
        return (
            "404 Not Found",
            json!({"error":"live Supervisor source not configured","read_only":true}).to_string(),
        );
    };
    if versions.next().is_some() {
        return (
            "503 Service Unavailable",
            json!({"error":"ambiguous live Supervisor source","read_only":true}).to_string(),
        );
    }
    let result = AppSupervisorClient::from_environment().and_then(|client| {
        let inventory = client.fetch_installed(version)?;
        let matches = inventory.apps.iter().filter(|app| app.slug == slug).count();
        if matches == 0 {
            return Ok(None);
        }
        if matches != 1 {
            return Err(
                hars_supervisor_apps::official_transport::TransportError::InvalidRuntimeInventory,
            );
        }
        client.fetch_app_stats(version, slug).map(Some)
    });
    match result {
        Ok(Some(stats)) => (
            "200 OK",
            json!({"read_only":true,"stats":stats}).to_string(),
        ),
        Ok(None) => (
            "404 Not Found",
            json!({"error":"app not installed","read_only":true}).to_string(),
        ),
        Err(_) => (
            "503 Service Unavailable",
            json!({"error":"Supervisor App stats unavailable","read_only":true}).to_string(),
        ),
    }
}

/// Details are available only for Apps independently confirmed to be installed
/// by the authenticated Supervisor, never via a user-supplied data source.
/// The official API response is parsed into an allowlisted/redacted type.
fn live_detail_response(adapters: &[Adapter], slug: &str) -> (&'static str, String) {
    let mut versions = adapters.iter().filter_map(|a| match a.kind {
        AdapterKind::SupervisorLive(v) => Some(v),
        _ => None,
    });
    let Some(version) = versions.next() else {
        return (
            "404 Not Found",
            json!({"error":"live Supervisor source not configured","read_only":true}).to_string(),
        );
    };
    if versions.next().is_some() {
        return (
            "503 Service Unavailable",
            json!({"error":"ambiguous live Supervisor source","read_only":true}).to_string(),
        );
    }
    let result = AppSupervisorClient::from_environment().and_then(|client| {
        let inventory = client.fetch_installed(version)?;
        if !inventory.apps.iter().any(|app| app.slug == slug) {
            return Ok(None);
        }
        client.fetch_app_detail(version, slug).map(Some)
    });
    match result {
        Ok(Some(app)) => ("200 OK", json!({"read_only":true,"app":app}).to_string()),
        Ok(None) => (
            "404 Not Found",
            json!({"error":"app not installed","read_only":true}).to_string(),
        ),
        Err(_) => (
            "503 Service Unavailable",
            json!({"error":"Supervisor App details unavailable","read_only":true}).to_string(),
        ),
    }
}

/// Read-only official Supervisor Store. The Marketplace for Core custom
/// integrations is a DIFFERENT service, owned by Home Assistant Core.
fn supervisor_store_response(adapters: &[Adapter]) -> (&'static str, String) {
    let mut versions = adapters.iter().filter_map(|a| match a.kind {
        AdapterKind::SupervisorLive(v) => Some(v),
        _ => None,
    });
    let Some(version) = versions.next() else {
        return (
            "404 Not Found",
            json!({"read_only":true,"error":"no live Supervisor Store source"}).to_string(),
        );
    };
    if versions.next().is_some() {
        return (
            "503 Service Unavailable",
            json!({"read_only":true,"error":"ambiguous Supervisor Store source"}).to_string(),
        );
    }
    let result =
        AppSupervisorClient::from_environment().and_then(|client| client.fetch_store(version));
    match result {
        Ok(catalog) => (
            "200 OK",
            json!({"read_only":true,"catalog":catalog}).to_string(),
        ),
        Err(_) => (
            "503 Service Unavailable",
            json!({"read_only":true,"error":"Supervisor Store unavailable"}).to_string(),
        ),
    }
}

/// Store details work for installed and uninstalled Apps alike.
fn store_detail_slug(line: &str) -> Option<Result<&str, &'static str>> {
    let rest = line.strip_prefix("GET /api/store/apps/")?;
    let slug = rest
        .strip_suffix("/detail HTTP/1.1")
        .or_else(|| rest.strip_suffix("/detail HTTP/1.0"))?;
    Some(if hars_supervisor_apps::app_detail::valid_slug(slug) {
        Ok(slug)
    } else {
        Err("invalid store app slug")
    })
}

/// Do not issue an App detail request unless the authenticated Store
/// enumerates that exact slug. Never fall back to an offline fixture.
fn checked_store_detail<F>(
    catalog: &hars_supervisor_apps::official_store::Catalog,
    slug: &str,
    fetch_detail: F,
) -> Result<
    Option<hars_supervisor_apps::app_detail::AppDetail>,
    hars_supervisor_apps::official_transport::TransportError,
>
where
    F: FnOnce() -> Result<
        hars_supervisor_apps::app_detail::AppDetail,
        hars_supervisor_apps::official_transport::TransportError,
    >,
{
    if !catalog.apps.iter().any(|app| app.slug == slug) {
        return Ok(None);
    }
    fetch_detail().map(Some)
}

fn supervisor_store_detail_response(adapters: &[Adapter], slug: &str) -> (&'static str, String) {
    if !hars_supervisor_apps::app_detail::valid_slug(slug) {
        return (
            "400 Bad Request",
            json!({"read_only":true,"error":"invalid store app slug"}).to_string(),
        );
    }
    let mut versions = adapters.iter().filter_map(|a| match a.kind {
        AdapterKind::SupervisorLive(v) => Some(v),
        _ => None,
    });
    let Some(version) = versions.next() else {
        return (
            "404 Not Found",
            json!({"read_only":true,"error":"no live Supervisor Store source"}).to_string(),
        );
    };
    if versions.next().is_some() {
        return (
            "503 Service Unavailable",
            json!({"read_only":true,"error":"ambiguous Supervisor Store source"}).to_string(),
        );
    }
    // Verify membership in the current authenticated Store before fetching.
    // No caller-selected host, offline snapshot fallback or raw options.
    let result = AppSupervisorClient::from_environment().and_then(|client| {
        let catalog = client.fetch_store(version)?;
        checked_store_detail(&catalog, slug, || {
            client.fetch_store_app_detail(version, slug)
        })
    });
    match result {
        Ok(Some(app)) => ("200 OK", json!({"read_only":true,"app":app}).to_string()),
        Ok(None) => (
            "404 Not Found",
            json!({"read_only":true,"error":"app not in current Store"}).to_string(),
        ),
        Err(_) => (
            "503 Service Unavailable",
            json!({"read_only":true,"error":"Supervisor Store App details unavailable"})
                .to_string(),
        ),
    }
}

/// Read-only evidence for all seven operations on any current Store App.
/// This route never accepts a verb other than GET and issues no Supervisor POST.
fn store_operations_slug(line: &str) -> Option<Result<&str, &'static str>> {
    let rest = line.strip_prefix("GET /api/store/apps/")?;
    let slug = rest
        .strip_suffix("/operations HTTP/1.1")
        .or_else(|| rest.strip_suffix("/operations HTTP/1.0"))?;
    Some(if valid_slug(slug) {
        Ok(slug)
    } else {
        Err("invalid store app slug")
    })
}

fn supervisor_store_operations_response(
    adapters: &[Adapter],
    slug: &str,
) -> (&'static str, String) {
    use hars_supervisor_apps::operation_preflight::{assess_lifecycle_evidence, LifecycleAction};
    if !valid_slug(slug) {
        return (
            "400 Bad Request",
            json!({"read_only":true,"activation_authorized":false,"error":"invalid store app slug"})
                .to_string(),
        );
    }
    let mut versions = adapters.iter().filter_map(|a| match a.kind {
        AdapterKind::SupervisorLive(v) => Some(v),
        _ => None,
    });
    let Some(version) = versions.next() else {
        return (
            "404 Not Found",
            json!({"read_only":true,"activation_authorized":false,"error":"no live Supervisor source"})
                .to_string(),
        );
    };
    if versions.next().is_some() {
        return (
            "503 Service Unavailable",
            json!({"read_only":true,"activation_authorized":false,"error":"ambiguous Supervisor source"})
                .to_string(),
        );
    }
    // Exactly three fresh official views, no cached snapshot or caller-supplied URL.
    let result = AppSupervisorClient::from_environment().and_then(|client| {
        let catalog = client.fetch_store(version)?;
        let inventory = client.fetch_installed(version)?;
        checked_store_detail(&catalog, slug, || {
            client.fetch_store_app_detail(version, slug)
        })
        .map(|detail| detail.map(|app| (catalog, inventory, app)))
    });
    match result {
        Ok(Some((catalog, inventory, detail))) => {
            let actions = [
                LifecycleAction::Install,
                LifecycleAction::Start,
                LifecycleAction::Stop,
                LifecycleAction::Restart,
                LifecycleAction::Update,
                LifecycleAction::Backup,
                LifecycleAction::Restore,
            ];
            let plans = actions
                .iter()
                .map(|action| {
                    assess_lifecycle_evidence(&catalog, &detail, &inventory, slug, *action)
                })
                .collect::<Result<Vec<_>, _>>();
            match plans {
                Ok(preflights) => (
                    "200 OK",
                    json!({"read_only":true,"activation_authorized":false,
                        "slug":slug,"preflights":preflights})
                    .to_string(),
                ),
                Err(_) => (
                    "503 Service Unavailable",
                    json!({"read_only":true,"activation_authorized":false,
                        "error":"inconsistent official Supervisor operation evidence"})
                    .to_string(),
                ),
            }
        }
        Ok(None) => (
            "404 Not Found",
            json!({"read_only":true,"activation_authorized":false,
                "error":"App absent from live official Store"})
            .to_string(),
        ),
        Err(_) => (
            "503 Service Unavailable",
            json!({"read_only":true,"activation_authorized":false,
                "error":"Supervisor operation evidence unavailable"})
            .to_string(),
        ),
    }
}

/// Read-only availability route for any current Store App, not a write permit.
fn store_availability_slug(line: &str) -> Option<Result<&str, &'static str>> {
    let rest = line.strip_prefix("GET /api/store/apps/")?;
    let slug = rest
        .strip_suffix("/availability HTTP/1.1")
        .or_else(|| rest.strip_suffix("/availability HTTP/1.0"))?;
    Some(if valid_slug(slug) {
        Ok(slug)
    } else {
        Err("invalid store app slug")
    })
}

fn supervisor_store_availability_response(
    adapters: &[Adapter],
    slug: &str,
) -> (&'static str, String) {
    use hars_supervisor_apps::store_availability::assess_store_availability;
    if !valid_slug(slug) {
        return (
            "400 Bad Request",
            json!({"read_only":true,"error":"invalid store app slug"}).to_string(),
        );
    }
    let mut versions = adapters.iter().filter_map(|a| match a.kind {
        AdapterKind::SupervisorLive(v) => Some(v),
        _ => None,
    });
    let Some(version) = versions.next() else {
        return (
            "404 Not Found",
            json!({"read_only":true,"error":"no live Supervisor Store source"}).to_string(),
        );
    };
    if versions.next().is_some() {
        return (
            "503 Service Unavailable",
            json!({"read_only":true,"error":"ambiguous Supervisor Store source"}).to_string(),
        );
    }
    // The official Store catalog and Store detail are both fetched anew
    // with the Supervisor-issued token; no synthetic fallback is permitted.
    let result = AppSupervisorClient::from_environment().and_then(|client| {
        let catalog = client.fetch_store(version)?;
        let inventory = client.fetch_installed(version)?;
        checked_store_detail(&catalog, slug, || {
            client.fetch_store_app_detail(version, slug)
        })
        .map(|detail| detail.map(|app| (catalog, inventory, app)))
    });
    match result {
        Ok(Some((catalog, inventory, detail))) => {
            match assess_store_availability(&catalog, &detail, &inventory, slug) {
                Ok(preflight) => (
                    "200 OK",
                    json!({"read_only":true,"preflight":preflight}).to_string(),
                ),
                Err(_) => (
                    "503 Service Unavailable",
                    json!({"read_only":true,"error":"inconsistent Supervisor Store availability"})
                        .to_string(),
                ),
            }
        }
        Ok(None) => (
            "404 Not Found",
            json!({"read_only":true,"error":"app not in current Store"}).to_string(),
        ),
        Err(_) => (
            "503 Service Unavailable",
            json!({"read_only":true,"error":"Supervisor Store availability unavailable"})
                .to_string(),
        ),
    }
}

fn manifest_sources(body: &str) -> Result<Vec<Adapter>, String> {
    let doc: Value = serde_json::from_str(body).map_err(|_| "invalid manifest JSON")?;
    let list = doc
        .get("sources")
        .and_then(Value::as_array)
        .ok_or("manifest missing sources")?;
    if list.is_empty() || list.len() > 16 {
        return Err("manifest requires 1-16 sources".into());
    }
    let mut adapters: Vec<Adapter> = Vec::new();
    for item in list {
        let host = field(item, "host");
        let path = field(item, "path");
        if !valid_host(host) || adapters.iter().any(|a| a.host == host) {
            return Err("invalid or duplicate host label".into());
        }
        // Read a constrained, local runtime snapshot only; never /config or arbitrary files.
        let mut components = Path::new(path).components();
        if !matches!(components.next(), Some(Component::Normal(part)) if part == "runtime")
            || !components.all(|c| matches!(c, Component::Normal(_)))
        {
            return Err("manifest source path must be inside runtime/".into());
        }
        let kind = match field(item, "kind") {
            "docker-fixture" => AdapterKind::DockerFixture(path.to_string()),
            "supervisor-snapshot" => AdapterKind::SupervisorSnapshot(path.to_string()),
            _ => return Err("unsupported manifest adapter".into()),
        };
        adapters.push(Adapter {
            host: host.to_string(),
            kind,
        });
    }
    Ok(adapters)
}

fn aggregate(adapters: &[Adapter]) -> Value {
    let mut sources = Vec::new();
    let mut apps = Vec::new();
    for a in adapters {
        match a.collect() {
            Ok(mut rows) => {
                let count = rows.len();
                apps.append(&mut rows);
                sources.push(json!({
                    "host": a.host(), "kind": a.kind(), "status": "ok", "count":count,
                    "live":matches!(a.kind, AdapterKind::DockerCli | AdapterKind::SupervisorLive(_)),
                }));
            }
            Err(err) => sources.push(json!({
                "host": a.host(), "kind": a.kind(), "status":"unavailable",
                "error":err, "count":0,"live":false,
            })),
        }
    }
    apps.sort_by(|a, b| {
        field(a, "host")
            .cmp(field(b, "host"))
            .then_with(|| field(a, "display_name").cmp(field(b, "display_name")))
    });
    let degraded = sources.iter().any(|s| s["status"] != "ok");
    json!({
        "read_only": true,
        "mode": "LAB",
        "degraded": degraded,
        "sources": sources,
        "apps": apps,
    })
}

fn reply(
    stream: &mut TcpStream,
    status: &str,
    body: &str,
    content_type: &str,
) -> std::io::Result<()> {
    let hdr = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nContent-Security-Policy: default-src 'none'; style-src 'unsafe-inline'; script-src 'unsafe-inline'; connect-src 'self'\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(hdr.as_bytes())?;
    stream.write_all(body.as_bytes())?;
    stream.flush()
}

struct SignedContext {
    manifest: String,
    path: String,
    trust_key: String,
    sources: Vec<String>,
}
impl SignedContext {
    fn verify(&self) -> Result<attestation::Verified, String> {
        attestation::verify(&self.manifest, &self.path, &self.trust_key, &self.sources)
            .map_err(str::to_owned)
    }
}

fn report(adapters: &[Adapter], verified: Option<&attestation::Verified>) -> Value {
    let mut data = aggregate(adapters);
    data["attestation"] = match verified {
        Some(a) => {
            json!({"status":"verified","issuer":a.issuer,"issued_at":a.issued_at,"expires_at":a.expires_at})
        }
        None => json!({"status":"unsigned","warning":"No signed source attestation requested"}),
    };
    data
}

/// Static, audited planning-only status, deliberately no production action.
fn readiness_report() -> Value {
    let gates: Value = serde_json::from_str(include_str!("../../../config/release-gates.json"))
        .expect("invalid release policy shipped with app");
    let gates_list = gates["required_gates"]
        .as_array()
        .expect("release-gates rows");
    json!({
        "mode":"LAB",
        "read_only":true,
        "deployment_authorized":false,
        "production_ready":false,
        "release_gates_total":gates_list.len(),
        "release_gates_blocked":gates_list.iter().filter(|g|g["status"]!="certified").count(),
        "note":"Status is a static project policy; no production configuration is read or changed"
    })
}

struct CoreSidecar {
    socket: PathBuf,
    capability: PathBuf,
}
impl CoreSidecar {
    fn snapshot_status(&self) -> Result<Value, ()> {
        let owner = fs::symlink_metadata(&self.capability)
            .map_err(|_| ())?
            .uid();
        let snap = hars_python_bridge::fetch_snapshot(&self.socket, &self.capability, owner)
            .map_err(|_| ())?;
        Ok(json!({
            "status":"verified_lab_local", "mode":"LAB",
            "read_only":true, "generation":snap.generation,
            "entities_count":snap.entities.len(),
            "source":"authenticated_local_python_sidecar",
            "contains_entity_values":false, "production_ready":false
        }))
    }
}

/// Fail closed for browser-originated requests to the unauthenticated local-only lab.
/// Loopback bind alone does NOT prevent DNS rebinding by a hostile webpage.
/// Not production authentication; remote access remains forbidden.
fn parse_local_http_request(raw: &[u8], local: SocketAddr) -> Result<&str, &'static str> {
    let data = std::str::from_utf8(raw).map_err(|_| "invalid HTTP encoding")?;
    let headers = data
        .strip_suffix("\r\n\r\n")
        .ok_or("incomplete HTTP headers")?;
    let mut lines = headers.split("\r\n");
    let request = lines.next().ok_or("missing HTTP request")?;
    let mut request_parts = request.split(' ');
    let method = request_parts.next().ok_or("missing method")?;
    let target = request_parts.next().ok_or("missing target")?;
    let version = request_parts.next().ok_or("missing HTTP version")?;
    if request_parts.next().is_some()
        || !method.bytes().all(|b| b.is_ascii_uppercase())
        || method.is_empty()
        || !target.starts_with('/')
        || target.starts_with("//")
        || target.bytes().any(|b| !b.is_ascii_graphic())
        || !matches!(version, "HTTP/1.1" | "HTTP/1.0")
    {
        return Err("invalid HTTP request line");
    }

    let expected_host = local.to_string();
    let alias_host = format!("localhost:{}", local.port());
    let mut host_seen = false;
    let mut origin_seen = false;
    let mut fetch_seen = false;
    for (count, header) in lines.enumerate() {
        if count >= 64 {
            return Err("too many HTTP headers");
        }
        let (key, raw_value) = header.split_once(':').ok_or("invalid HTTP header")?;
        if key.is_empty()
            || !key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
            || header
                .bytes()
                .any(|b| (b < 32 && b != b'\t') || b == 127 || b > 126)
        {
            return Err("invalid HTTP header bytes");
        }
        let value = raw_value.trim_matches([' ', '\t']);
        if key.eq_ignore_ascii_case("host") {
            if host_seen || !(value == expected_host || value == alias_host) {
                return Err("Host is not the local listener");
            }
            host_seen = true;
        } else if key.eq_ignore_ascii_case("origin") {
            if origin_seen
                || !(value == format!("http://{expected_host}")
                    || value == format!("http://{alias_host}"))
            {
                return Err("Origin is not the local listener");
            }
            origin_seen = true;
        } else if key.eq_ignore_ascii_case("sec-fetch-site") {
            if fetch_seen || !matches!(value, "same-origin" | "none") {
                return Err("cross-site browser access not allowed");
            }
            fetch_seen = true;
        } else if key.eq_ignore_ascii_case("transfer-encoding")
            || (key.eq_ignore_ascii_case("content-length") && value != "0")
        {
            return Err("HTTP request body not accepted");
        }
    }
    if !host_seen {
        return Err("Host header required");
    }
    Ok(request)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AccessMode {
    Loopback,
    SupervisorIngress,
}

/// Informational native Supervisor session status; never an execution
/// capability. The upstream Supervisor owns login, admin validation and
/// injection of the sanitized effective group header on each request.
/// No user id, group names or tokens are emitted to the browser.
fn supervisor_session_response(
    mode: AccessMode,
    has_verified_identity: bool,
    is_admin: bool,
) -> (&'static str, String) {
    if mode != AccessMode::SupervisorIngress {
        return (
            "404 Not Found",
            json!({"read_only":true,"error":"supervisor ingress required"}).to_string(),
        );
    }
    if !has_verified_identity {
        return (
            "403 Forbidden",
            json!({"read_only":true,"error":"verified Supervisor user required"}).to_string(),
        );
    }
    (
        "200 OK",
        json!({
            "authenticated":true,
            "administrator":is_admin,
            "authority":"supervisor_ingress",
            "read_only":true,
            "actions_authorized":false
        })
        .to_string(),
    )
}

// Bounded, independent request handling: 8 workers, at most 32 pending.
// No unbounded thread-per-connection allocation or blocking send in accept().
const LAB_HTTP_WORKERS: usize = 8;
const LAB_HTTP_QUEUE: usize = 32;

fn handle(
    mut conn: TcpStream,
    adapters: &[Adapter],
    signed: Option<&SignedContext>,
    core: Option<&CoreSidecar>,
    mode: AccessMode,
) {
    let _ = conn.set_write_timeout(Some(Duration::from_secs(2)));
    // Deadline covers the ENTIRE header, not each read separately. A client
    // trickling one byte every second cannot keep a worker forever.
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut buffer = [0_u8; 4096];
    let mut size = 0;
    // Read the complete bounded HTTP header, not an arbitrary TCP chunk.
    loop {
        let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
            return;
        };
        if remaining.is_zero() {
            return;
        }
        let _ = conn.set_read_timeout(Some(remaining));
        if size == buffer.len() {
            // Drain only a bounded extra header tail before 431. Closing a TCP
            // connection with unread bytes can reset it and hide our response.
            // Never accept oversized headers even if the tail terminates.
            let _ = conn.set_read_timeout(Some(Duration::from_millis(100)));
            let mut tail = Vec::from(&buffer[buffer.len() - 3..]);
            let mut scratch = [0_u8; 1024];
            while tail.len() < 8192 {
                let take = scratch.len().min(8192 - tail.len());
                match conn.read(&mut scratch[..take]) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => tail.extend_from_slice(&scratch[..n]),
                }
                if tail.windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
            }
            let _ = reply(
                &mut conn,
                "431 Request Header Fields Too Large",
                r#"{"error":"HTTP headers too large"}"#,
                "application/json; charset=utf-8",
            );
            return;
        }
        match conn.read(&mut buffer[size..]) {
            Ok(0) | Err(_) => return,
            Ok(n) => size += n,
        }
        if let Some(pos) = buffer[..size].windows(4).position(|w| w == b"\r\n\r\n") {
            // No HTTP pipelining or request body in this lab API.
            if pos + 4 != size {
                let _ = reply(
                    &mut conn,
                    "400 Bad Request",
                    r#"{"error":"HTTP request body or pipelining denied"}"#,
                    "application/json; charset=utf-8",
                );
                return;
            }
            break;
        }
    }
    let local = match conn.local_addr() {
        Ok(addr) => addr,
        Err(_) => return,
    };
    let (line, claimed_user, supervisor_admin) = match mode {
        AccessMode::Loopback => match parse_local_http_request(&buffer[..size], local) {
            Ok(line) => (line.to_owned(), None, false),
            Err(_) => {
                let _ = reply(
                    &mut conn,
                    "400 Bad Request",
                    r#"{"error":"invalid local HTTP request"}"#,
                    "application/json; charset=utf-8",
                );
                return;
            }
        },
        AccessMode::SupervisorIngress => {
            let peer = match conn.peer_addr() {
                Ok(a) => a,
                Err(_) => return,
            };
            match ingress::verified_route(&buffer[..size], peer) {
                Ok(identity) => (identity.route, Some(identity.user_id), identity.is_admin),
                Err(_) => {
                    let _ = reply(
                        &mut conn,
                        "403 Forbidden",
                        r#"{"error":"official Supervisor ingress identity required"}"#,
                        "application/json; charset=utf-8",
                    );
                    return;
                }
            }
        }
    };
    // Supervisor already verified the authenticated user's effective groups
    // for the ingress session; this is only an informational read-only flag.
    // NEVER treat this or any header alone as an App write permit.
    let session_identity_present = claimed_user.is_some();
    drop(claimed_user);
    if line == "GET / HTTP/1.1" || line == "GET / HTTP/1.0" {
        let _ = reply(
            &mut conn,
            "200 OK",
            include_str!("../../../web/index.html"),
            "text/html; charset=utf-8",
        );
        return;
    }
    // Re-check signature, file digests and expiry for every HTTP request.
    // A valid startup snapshot must not remain trusted indefinitely.
    let checked = match signed {
        Some(s) => match s.verify() {
            Ok(v) => Some(v),
            Err(_) => {
                let _ = reply(
                    &mut conn,
                    "503 Service Unavailable",
                    &json!({"error":"signed inventory invalid or expired", "read_only":true})
                        .to_string(),
                    "application/json; charset=utf-8",
                );
                return;
            }
        },
        None => None,
    };
    let verified = checked.as_ref();
    let (status, response) = if line == "GET /health HTTP/1.1" || line == "GET /health HTTP/1.0" {
        ("200 OK", json!({"status":"ok","mode":"LAB","read_only":true,"version":env!("CARGO_PKG_VERSION")}).to_string())
    } else if line == "GET /api/session HTTP/1.1" || line == "GET /api/session HTTP/1.0" {
        supervisor_session_response(mode, session_identity_present, supervisor_admin)
    } else if line == "GET /api/apps HTTP/1.1" || line == "GET /api/apps HTTP/1.0" {
        ("200 OK", report(adapters, verified).to_string())
    } else if line == "GET /api/store HTTP/1.1" || line == "GET /api/store HTTP/1.0" {
        supervisor_store_response(adapters)
    } else if let Some(slug) = store_operations_slug(&line) {
        match slug {
            Ok(slug) => supervisor_store_operations_response(adapters, slug),
            Err(_) => (
                "400 Bad Request",
                json!({"error":"invalid store app slug","read_only":true,"activation_authorized":false})
                    .to_string(),
            ),
        }
    } else if let Some(slug) = store_availability_slug(&line) {
        match slug {
            Ok(slug) => supervisor_store_availability_response(adapters, slug),
            Err(_) => (
                "400 Bad Request",
                json!({"error":"invalid store app slug","read_only":true}).to_string(),
            ),
        }
    } else if let Some(slug) = store_detail_slug(&line) {
        match slug {
            Ok(slug) => supervisor_store_detail_response(adapters, slug),
            Err(_) => (
                "400 Bad Request",
                json!({"error":"invalid store app slug","read_only":true}).to_string(),
            ),
        }
    } else if line == "GET /api/jobs HTTP/1.1" || line == "GET /api/jobs HTTP/1.0" {
        live_jobs_response(adapters, mode, session_identity_present, supervisor_admin)
    } else if let Some(id) = live_job_id(&line) {
        match id {
            Ok(id) => live_job_response(
                adapters,
                id,
                mode,
                session_identity_present,
                supervisor_admin,
            ),
            Err(_) => (
                "400 Bad Request",
                json!({"error":"invalid Supervisor job id","read_only":true}).to_string(),
            ),
        }
    } else if let Some(slug) = live_config_slug(&line) {
        match slug {
            Ok(slug) => live_config_response(adapters, slug),
            Err(_) => (
                "400 Bad Request",
                json!({"error":"invalid app slug","read_only":true}).to_string(),
            ),
        }
    } else if let Some(slug) = live_changelog_slug(&line) {
        match slug {
            Ok(slug) => live_changelog_response(adapters, slug),
            Err(_) => (
                "400 Bad Request",
                json!({"error":"invalid app slug","read_only":true}).to_string(),
            ),
        }
    } else if let Some(slug) = live_stats_slug(&line) {
        match slug {
            Ok(slug) => live_stats_response(adapters, slug),
            Err(_) => (
                "400 Bad Request",
                json!({"error":"invalid app slug","read_only":true}).to_string(),
            ),
        }
    } else if let Some(slug) = live_detail_slug(&line) {
        match slug {
            Ok(slug) => live_detail_response(adapters, slug),
            Err(_) => (
                "400 Bad Request",
                json!({"error":"invalid app slug","read_only":true}).to_string(),
            ),
        }
    } else if line == "GET /api/readiness HTTP/1.1" || line == "GET /api/readiness HTTP/1.0" {
        ("200 OK", readiness_report().to_string())
    } else if line == "GET /api/python-core HTTP/1.1" || line == "GET /api/python-core HTTP/1.0" {
        match core {
            Some(c) => match c.snapshot_status() {
                Ok(v) => ("200 OK", v.to_string()),
                Err(()) => (
                    "503 Service Unavailable",
                    json!({"status":"unavailable","read_only":true,
                        "error":"private Python sidecar unavailable"})
                    .to_string(),
                ),
            },
            None => (
                "404 Not Found",
                json!({"status":"not_configured","read_only":true}).to_string(),
            ),
        }
    } else if line == "GET /api/sources HTTP/1.1" || line == "GET /api/sources HTTP/1.0" {
        let a = report(adapters, verified);
        (
            "200 OK",
            json!({"read_only":true,"sources":a["sources"],"degraded":a["degraded"],"attestation":a["attestation"]}).to_string(),
        )
    } else {
        (
            "404 Not Found",
            json!({"error":"unknown route"}).to_string(),
        )
    };
    let _ = reply(
        &mut conn,
        status,
        &response,
        "application/json; charset=utf-8",
    );
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let mut adapters: Vec<Adapter> = Vec::new();
    let mut bind: Option<String> = None;
    let mut manifest_file: Option<String> = None;
    let mut attestation_file: Option<String> = None;
    let mut public_key_file: Option<String> = None;
    let mut python_socket: Option<String> = None;
    let mut python_capability: Option<String> = None;
    let mut ingress_mode = false;
    while let Some(arg) = args.next() {
        let kind = match arg.as_str() {
            "--attestation" if attestation_file.is_none() => {
                attestation_file = Some(args.next().ok_or("--attestation needs a file")?);
                None
            }
            "--trust-key" if public_key_file.is_none() => {
                public_key_file = Some(args.next().ok_or("--trust-key needs a file")?);
                None
            }
            "--python-socket" if python_socket.is_none() => {
                python_socket = Some(args.next().ok_or("--python-socket requires path")?);
                None
            }
            "--python-capability" if python_capability.is_none() => {
                python_capability = Some(args.next().ok_or("--python-capability requires file")?);
                None
            }
            "--docker" => Some(AdapterKind::DockerCli),
            "--ingress" if !ingress_mode => {
                ingress_mode = true;
                None
            }
            "--supervisor-live" => Some(AdapterKind::SupervisorLive(ApiVersion::V1)),
            "--supervisor-live-v2" => Some(AdapterKind::SupervisorLive(ApiVersion::V2)),
            "--manifest" if manifest_file.is_none() => {
                let path = args.next().ok_or("--manifest requires a path")?;
                manifest_file = Some(path.clone());
                let body = fs::read_to_string(path).map_err(|_| "manifest file unavailable")?;
                let entries = manifest_sources(&body)?;
                for entry in entries {
                    if adapters.len() >= 16 || adapters.iter().any(|a| a.host == entry.host) {
                        return Err("too many or duplicate sources".into());
                    }
                    adapters.push(entry);
                }
                None
            }
            "--fixture" => Some(AdapterKind::DockerFixture(
                args.next().ok_or("--fixture requires a path")?,
            )),
            "--haos-snapshot" => Some(AdapterKind::SupervisorSnapshot(
                args.next().ok_or("--haos-snapshot requires a path")?,
            )),
            "--host" => {
                let host = args.next().ok_or("--host requires a label")?;
                if !valid_host(&host) {
                    return Err("host label must be 1-40 ASCII letters, numbers, - or _".into());
                }
                if adapters.iter().any(|a| a.host == host) {
                    return Err("duplicate host label".into());
                }
                let a = adapters.last_mut().ok_or("--host must follow a source")?;
                a.host = host;
                None
            }
            "--serve" if bind.is_none() => {
                bind = Some(args.next().ok_or("--serve requires a loopback address")?);
                None
            }
            _ => return Err(format!("unknown/repeated argument: {arg}")),
        };
        if let Some(kind) = kind {
            if adapters.len() >= 16 {
                return Err("maximum 16 sources allowed".into());
            }
            // Optional host labels can be added with --host, scoped to the previous source.
            let index = adapters.len() + 1;
            adapters.push(Adapter {
                host: format!("host{index}"),
                kind,
            });
        } else {
            // --serve: no adapter added
        }
    }
    if adapters.is_empty() {
        return Err("select at least one of --docker, --fixture FILE, --haos-snapshot FILE, --supervisor-live".into());
    }
    let core = match (python_socket, python_capability) {
        (Some(socket), Some(capability)) => Some(CoreSidecar {
            socket: PathBuf::from(socket),
            capability: PathBuf::from(capability),
        }),
        (None, None) => None,
        _ => return Err("--python-socket and --python-capability must be paired".into()),
    };
    let signed = match (manifest_file, attestation_file, public_key_file) {
        (Some(m), Some(a), Some(k)) => {
            let mut paths = Vec::new();
            for a in &adapters {
                let p = match &a.kind {
                    AdapterKind::DockerFixture(p) | AdapterKind::SupervisorSnapshot(p) => p.clone(),
                    AdapterKind::DockerCli | AdapterKind::SupervisorLive(_) => {
                        return Err("signed mode needs snapshots".into())
                    }
                };
                paths.push(p);
            }
            Some(SignedContext {
                manifest: m,
                path: a,
                trust_key: k,
                sources: paths,
            })
        }
        (None, None, None) => None,
        _ => return Err("signed mode needs manifest, attestation and trust key".into()),
    };
    let verified = match signed.as_ref() {
        Some(s) => Some(s.verify()?),
        None => None,
    };
    if ingress_mode {
        if bind.is_none() {
            return Err("Supervisor ingress requires --serve".into());
        }
        if signed.is_some()
            || core.is_some()
            || adapters
                .iter()
                .any(|a| !matches!(a.kind, AdapterKind::SupervisorLive(_)))
        {
            return Err("Supervisor ingress requires only official live App adapters".into());
        }
        // Require official in-App auth; any caller can set an env variable,
        // but no HTTP peer other than the Supervisor IP is accepted.
        AppSupervisorClient::from_environment()
            .map_err(|_| "official Supervisor App token unavailable")?;
    }
    if let Some(addr) = bind {
        let addr: SocketAddr = addr.parse().map_err(|_| "invalid bind address")?;
        if ingress_mode {
            if !matches!(addr.ip(), IpAddr::V4(v) if v.is_unspecified()) || addr.port() < 1024 {
                return Err(
                    "Supervisor ingress requires a container interface and unprivileged port"
                        .into(),
                );
            }
        } else if !matches!(addr.ip(), IpAddr::V4(v) if v.is_loopback())
            && !matches!(addr.ip(), IpAddr::V6(v) if v.is_loopback())
        {
            return Err("the lab API must be bound to a loopback address only".into());
        }
        let mode = if ingress_mode {
            AccessMode::SupervisorIngress
        } else {
            AccessMode::Loopback
        };
        let listener = TcpListener::bind(addr).map_err(|e| format!("bind failed: {e}"))?;
        eprintln!("Rusty Home lab (read only): {addr}");
        let adapters = Arc::new(adapters);
        let signed = Arc::new(signed);
        let core = Arc::new(core);
        let (sender, receiver) = mpsc::sync_channel::<TcpStream>(LAB_HTTP_QUEUE);
        let receiver = Arc::new(Mutex::new(receiver));
        for _ in 0..LAB_HTTP_WORKERS {
            let receiver = Arc::clone(&receiver);
            let adapters = Arc::clone(&adapters);
            let signed = Arc::clone(&signed);
            let core = Arc::clone(&core);
            thread::spawn(move || loop {
                // Do not hold the receiver mutex while handling a request.
                let next = match receiver.lock() {
                    Ok(queue) => queue.recv(),
                    Err(_) => break,
                };
                let Ok(conn) = next else {
                    break;
                };
                handle(
                    conn,
                    &adapters,
                    signed.as_ref().as_ref(),
                    core.as_ref().as_ref(),
                    mode,
                );
            });
        }
        for incoming in listener.incoming() {
            match incoming {
                Ok(conn) => match sender.try_send(conn) {
                    Ok(()) => (),
                    Err(mpsc::TrySendError::Full(mut conn)) => {
                        // Capacity exhausted: shed load immediately. Do not
                        // allocate a thread or block the listener.
                        let _ = conn.set_write_timeout(Some(Duration::from_millis(100)));
                        let _ = reply(
                            &mut conn,
                            "503 Service Unavailable",
                            r#"{"error":"lab HTTP capacity exhausted","read_only":true}"#,
                            "application/json; charset=utf-8",
                        );
                    }
                    Err(mpsc::TrySendError::Disconnected(_)) => {
                        return Err("all lab HTTP workers disconnected".into());
                    }
                },
                Err(err) => eprintln!("connection failed: {err}"),
            }
        }
        Ok(())
    } else {
        let mut summary = report(&adapters, verified.as_ref());
        if let Some(sidecar) = core.as_ref() {
            summary["python_core"] = sidecar
                .snapshot_status()
                .map_err(|_| "authenticated local Python sidecar unavailable")?;
        }
        println!("{summary}");
        if summary["degraded"] == true {
            return Err("one or more read-only sources unavailable".into());
        }
        Ok(())
    }
}

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {err}\nUsage: ha-rust-bridge [--manifest FILE|--docker|--fixture FILE|--haos-snapshot FILE] [--host LABEL]... [--attestation FILE --trust-key FILE] [--serve 127.0.0.1:18790]");
        std::process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn job_detail_requires_current_verified_supervisor_admin_before_network() {
        let id = "0123456789abcdef0123456789abcdef";
        let no_sources: Vec<Adapter> = vec![];
        for (mode, identity, admin, expected) in [
            (AccessMode::Loopback, false, false, "404 Not Found"),
            (AccessMode::Loopback, true, true, "404 Not Found"),
            (AccessMode::SupervisorIngress, false, true, "403 Forbidden"),
            (AccessMode::SupervisorIngress, true, false, "403 Forbidden"),
            (AccessMode::SupervisorIngress, true, true, "404 Not Found"),
        ] {
            let (status, body) = live_job_response(&no_sources, id, mode, identity, admin);
            assert_eq!(status, expected);
            assert!(!body.contains("token"));
        }
    }

    #[test]
    fn job_discovery_requires_supervisor_admin_and_unique_live_source() {
        use super::*;
        let live = Adapter {
            host: "supervisor".into(),
            kind: AdapterKind::SupervisorLive(ApiVersion::V1),
        };
        for (mode, identity, admin, expected) in [
            (AccessMode::Loopback, true, true, "404 Not Found"),
            (AccessMode::SupervisorIngress, false, true, "403 Forbidden"),
            (AccessMode::SupervisorIngress, true, false, "403 Forbidden"),
            (AccessMode::SupervisorIngress, true, true, "404 Not Found"),
        ] {
            let (status, body) = live_jobs_response(&[], mode, identity, admin);
            assert_eq!(status, expected);
            assert!(!body.contains("token"));
        }
        let (status, _) = live_jobs_response(
            &[live.clone(), live],
            AccessMode::SupervisorIngress,
            true,
            true,
        );
        assert_eq!(status, "503 Service Unavailable");
    }

    use super::*;

    #[test]
    fn bounded_lab_http_resource_policy() {
        assert_eq!(LAB_HTTP_WORKERS, 8);
        assert_eq!(LAB_HTTP_QUEUE, 32);
    }

    #[test]
    fn supervisor_job_progress_route_is_canonical_and_read_only() {
        let id = "0123456789abcdef0123456789abcdef";
        for version in ["HTTP/1.1", "HTTP/1.0"] {
            let line = format!("GET /api/jobs/{id} {version}");
            assert_eq!(live_job_id(&line), Some(Ok(id)));
        }
        for bad in [
            "",
            "deadbeef",
            "ABCDEF0123456789ABCDEF0123456789",
            "../jobs",
            "a%2fb",
            "a?x=1",
            "a#fragment",
        ] {
            let line = format!("GET /api/jobs/{bad} HTTP/1.1");
            assert!(matches!(live_job_id(&line), Some(Err(_))), "accepted {bad}");
        }
        assert_eq!(live_job_id(&format!("POST /api/jobs/{id} HTTP/1.1")), None);
    }

    #[test]
    fn supervisor_job_progress_response_redacts_stages_and_rejects_forgery() {
        use hars_supervisor_apps::job_status::{JobStatus, State};
        let job = JobStatus {
            id: "0123456789abcdef0123456789abcdef".into(),
            state: State::Running,
            progress: 42.0,
            error_count: 1,
            stage: Some("SECRET PRIVATE STAGE".into()),
            read_only: true,
        };
        let view = public_job_summary(&job).expect("valid read-only job");
        assert_eq!(view["state"], "running");
        assert_eq!(view["progress"], 42.0);
        assert_eq!(view["error_count"], 1);
        assert_eq!(view["read_only"], true);
        assert!(!view.to_string().contains("SECRET PRIVATE STAGE"));
        assert!(view.get("stage").is_none());
        let mut forged = job.clone();
        forged.read_only = false;
        assert!(public_job_summary(&forged).is_none());
        forged.read_only = true;
        forged.progress = f64::NAN;
        assert!(public_job_summary(&forged).is_none());
    }

    #[test]
    fn supervisor_job_progress_dashboard_is_read_only_and_validates_official_id() {
        let html = include_str!("../../../web/index.html");
        for expected in [
            "id=\"supervisor-job-tracker\"",
            "id=\"job-id\"",
            "id=\"load-job\"",
            "api/jobs/",
            "refreshSupervisorJob",
            "data.read_only!==true",
            "j.read_only!==true",
            "j.id!==id",
            "Number.isSafeInteger(j.error_count)",
            "select(\"#load-job\").disabled=!(supervisorAvailable&&supervisorAdmin)",
        ] {
            assert!(
                html.contains(expected),
                "missing UI safety contract: {expected}"
            );
        }
        assert!(!html.contains("POST /api/jobs/"));
    }

    #[test]
    fn supervisor_job_progress_fails_closed_without_one_live_source() {
        let id = "0123456789abcdef0123456789abcdef";
        let empty: Vec<Adapter> = vec![];
        assert_eq!(
            live_job_response(&empty, id, AccessMode::SupervisorIngress, true, true).0,
            "404 Not Found"
        );
        let duplicated = vec![
            Adapter {
                host: "a".into(),
                kind: AdapterKind::SupervisorLive(ApiVersion::V1),
            },
            Adapter {
                host: "b".into(),
                kind: AdapterKind::SupervisorLive(ApiVersion::V2),
            },
        ];
        assert_eq!(
            live_job_response(&duplicated, id, AccessMode::SupervisorIngress, true, true).0,
            "503 Service Unavailable"
        );
    }

    #[test]
    fn local_http_rejects_dns_rebinding_cross_site_and_smuggling() {
        let addr: SocketAddr = "127.0.0.1:18790".parse().unwrap();
        let ok = b"GET /api/readiness HTTP/1.1\r\nHost: 127.0.0.1:18790\r\n\r\n";
        assert_eq!(
            parse_local_http_request(ok, addr).unwrap(),
            "GET /api/readiness HTTP/1.1"
        );
        let localhost = b"GET /health HTTP/1.0\r\nhost: localhost:18790\r\n\r\n";
        assert!(parse_local_http_request(localhost, addr).is_ok());
        let bad: &[&[u8]] = &[
            b"GET /health HTTP/1.1\r\nHost: evil.test:18790\r\n\r\n",
            b"GET /health HTTP/1.1\r\n\r\n",
            b"GET /health HTTP/1.1\r\nHost: localhost:18790\r\nHost: localhost:18790\r\n\r\n",
            b"GET /health HTTP/1.1\r\nHost: localhost:18790\r\nOrigin: https://evil.test\r\n\r\n",
            b"GET /health HTTP/1.1\r\nHost: localhost:18790\r\nOrigin: null\r\n\r\n",
            b"GET /health HTTP/1.1\r\nHost: localhost:18790\r\nSec-Fetch-Site: cross-site\r\n\r\n",
            b"GET http://localhost:18790/health HTTP/1.1\r\nHost: localhost:18790\r\n\r\n",
            b"GET /health HTTP/1.1\r\nHost: localhost:18790\r\nTransfer-Encoding: chunked\r\n\r\n",
            b"GET /health HTTP/1.1\r\nHost: localhost:18790\r\nContent-Length: 1\r\n\r\n",
            b"GET /health HTTP/1.1\nHost: localhost:18790\n\n",
            b"GET /health HTTP/1.1\r\nHost: localhost:18790\r\n Folded: value\r\n\r\n",
            b"GET /health HTTP/1.1\r\nHost: localhost:18790\r\nX-A: a\nInjected: x\r\n\r\n",
        ];
        for request in bad {
            assert!(
                parse_local_http_request(request, addr).is_err(),
                "accepted {request:?}"
            );
        }
    }

    #[test]
    fn local_http_requires_correct_ipv6_port_and_origin() {
        let addr: SocketAddr = "[::1]:19791".parse().unwrap();
        assert!(parse_local_http_request(
            b"GET /health HTTP/1.1\r\nHost: [::1]:19791\r\nOrigin: http://[::1]:19791\r\n\r\n",
            addr
        )
        .is_ok());
        assert!(parse_local_http_request(
            b"GET /health HTTP/1.1\r\nHost: [::1]:18790\r\n\r\n",
            addr
        )
        .is_err());
    }

    fn project_file(path: &str) -> String {
        format!("{}/../../{path}", env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn release_readiness_is_explicitly_blocked_in_lab() {
        let status = readiness_report();
        assert_eq!(status["production_ready"], false);
        assert_eq!(status["deployment_authorized"], false);
        assert_eq!(status["release_gates_total"], 15);
        assert!(status.get("haos_apps_total").is_none());
        assert!(status.get("haos_apps_not_production_qualified").is_none());
        assert_eq!(status["release_gates_blocked"], 15);
        let serialized = status.to_string();
        assert!(!serialized.contains("access_token"));
        assert!(!serialized.contains("private_key"));
    }

    #[test]
    fn docker_apps_and_unknown_containers() {
        let rows = docker_rows(include_str!("../../../fixtures/docker-ps.jsonl"), "lab").unwrap();
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0]["app_type"], "generic_container");
        assert_eq!(rows[1]["app_type"], "generic_container");
        assert_eq!(rows[2]["app_type"], "generic_container");
        assert_eq!(rows[3]["classification"], "docker");
        assert_eq!(rows[0]["id"], "lab:docker:esphome-lab");
    }

    #[test]
    fn sanitizes_labels_and_environment() {
        let row = r#"{"Names":"esphome","Image":"esphome","State":"running","Labels":"API_TOKEN=SECRET","Mounts":"PRIVATE"}"#;
        let rows = docker_rows(row, "lab").unwrap();
        let out = rows[0].to_string();
        assert!(!out.contains("SECRET"));
        assert!(!out.contains("PRIVATE"));
        assert!(!out.contains("Labels"));
        assert!(!out.contains("Mounts"));
    }

    #[test]
    fn supervisor_apps_real_snapshot() {
        let rows = supervisor_rows(include_str!("../../../audits/haos-apps.json"), "haos").unwrap();
        assert_eq!(rows.len(), 9);
        assert!(rows.iter().any(|r| r["display_name"] == "Music Assistant"));
        assert!(rows.iter().all(|r| r["backend"] == "supervisor"));
    }

    #[test]
    fn supervisor_api_envelope_and_no_options_leak() {
        let data = r#"{"result":"ok","data":{"addons":[{"slug":"core_matter_server","name":"Matter Server","state":"started","version":"9.2.0","options":{"password":"NEVER_EXPOSE"}}]}}"#;
        let rows = supervisor_rows(data, "haos").unwrap();
        assert_eq!(rows.len(), 1);
        assert!(!rows[0].to_string().contains("NEVER_EXPOSE"));
        assert_eq!(rows[0]["app_type"], "supervisor_app");
    }

    #[test]
    fn native_ingress_session_is_information_only_never_a_write_permit() {
        let (status, content) =
            supervisor_session_response(AccessMode::SupervisorIngress, true, true);
        assert_eq!(status, "200 OK");
        let data: Value = serde_json::from_str(&content).unwrap();
        assert_eq!(data["authenticated"], true);
        assert_eq!(data["administrator"], true);
        assert_eq!(data["actions_authorized"], false);
        assert_eq!(data["read_only"], true);
        for secret in ["user_id", "token", "group_ids", "SUPERVISOR_TOKEN"] {
            assert!(!content.contains(secret));
        }
        let (status, content) =
            supervisor_session_response(AccessMode::SupervisorIngress, true, false);
        assert_eq!(status, "200 OK");
        assert_eq!(
            serde_json::from_str::<Value>(&content).unwrap()["administrator"],
            false
        );
        assert_eq!(
            supervisor_session_response(AccessMode::SupervisorIngress, false, true).0,
            "403 Forbidden"
        );
        assert_eq!(
            supervisor_session_response(AccessMode::Loopback, false, false).0,
            "404 Not Found"
        );
    }

    #[test]
    fn genuinely_typed_supervisor_runtime_adapter_has_no_app_name_allowlist_or_secrets() {
        use hars_supervisor_apps::installed_inventory::parse_running_inventory;
        let sample: Vec<Value> = (0..1403)
            .rev()
            .map(|i| {
                json!({
                    "slug":format!("new_vendor_app_{i:05}"),
                    "name":format!("Unknown Future App {i}"),
                    "repository":format!("third_party_{}",i%29),
                    "version":"1.0","state":"started","update_available":i%3==0,
                    "options":{"password":"HIDE_ME"},"devices":["/dev/not-disclosed"]
                })
            })
            .collect();
        let raw = json!({"result":"ok","data":{"addons":sample}}).to_string();
        let typed = parse_running_inventory(raw.as_bytes(), ApiVersion::V1).unwrap();
        let rows = supervisor_live_rows(&typed, "dynamic-host");
        assert_eq!(rows.len(), 1403);
        assert_eq!(
            rows[0]["id"],
            "dynamic-host:supervisor:new_vendor_app_00000"
        );
        assert_eq!(rows[1402]["backend"], "supervisor");
        assert_eq!(rows[1402]["managed_by_bridge"], false);
        assert!(rows
            .iter()
            .all(|row| row["capabilities"] == json!(["view_status"])));
        let output = serde_json::to_string(&rows).unwrap();
        for private in [
            "HIDE_ME",
            "/dev/not-disclosed",
            "options",
            "devices",
            "password",
        ] {
            assert!(!output.contains(private));
        }
    }

    #[test]
    fn supervisor_live_never_claims_success_without_official_api_access() {
        // No source-side permitted URL or credentials may be supplied through
        // a snapshot manifest; only the official in-App token is ever used.
        let adapter = Adapter {
            host: "future-vendor".into(),
            kind: AdapterKind::SupervisorLive(ApiVersion::V1),
        };
        assert_eq!(adapter.kind(), "supervisor-live");
        assert!(matches!(adapter.kind, AdapterKind::SupervisorLive(_)));
        let source = supervisor_live_rows(
            &hars_supervisor_apps::installed_inventory::RunningInventory {
                api_version: ApiVersion::V1,
                apps: vec![],
                read_only: true,
            },
            "test-host",
        );
        assert!(source.is_empty());
    }

    #[test]
    fn official_changelog_route_rejects_unsafe_slugs_and_nonlive_sources() {
        for version in ["HTTP/1.0", "HTTP/1.1"] {
            let route = format!("GET /api/apps/future_vendor_999/changelog {version}");
            assert_eq!(live_changelog_slug(&route), Some(Ok("future_vendor_999")));
        }
        for slug in ["%2e%2e", "..", "a/b", "a?token=x", "a#b"] {
            let route = format!("GET /api/apps/{slug}/changelog HTTP/1.1");
            assert!(matches!(live_changelog_slug(&route), Some(Err(_))));
        }
        assert!(live_changelog_slug("POST /api/apps/a/changelog HTTP/1.1").is_none());
        let (status, response) = live_changelog_response(&[], "unknown_vendor_app");
        assert_eq!(status, "404 Not Found");
        assert!(response.contains("read_only"));
        let ambiguous = vec![
            Adapter {
                host: "one".into(),
                kind: AdapterKind::SupervisorLive(ApiVersion::V1),
            },
            Adapter {
                host: "two".into(),
                kind: AdapterKind::SupervisorLive(ApiVersion::V2),
            },
        ];
        assert_eq!(
            live_changelog_response(&ambiguous, "unknown_vendor_app").0,
            "503 Service Unavailable"
        );
        assert_eq!(
            live_changelog_response(&ambiguous, "../bad").0,
            "400 Bad Request"
        );
    }

    #[test]
    fn live_detail_routes_accept_unknown_future_vendors_and_reject_traversal() {
        for slug in ["core_mosquitto", "thirdparty_brand_A-493", "future_9000"] {
            for version in ["HTTP/1.1", "HTTP/1.0"] {
                let line = format!("GET /api/apps/{slug}/detail {version}");
                assert_eq!(live_detail_slug(&line), Some(Ok(slug)));
            }
        }
        for slug in ["", "..", "%2f", "a/b", "a?secret=true", "a%00b"] {
            let line = format!("GET /api/apps/{slug}/detail HTTP/1.1");
            assert_eq!(live_detail_slug(&line), Some(Err("invalid app slug")));
        }
        assert_eq!(
            live_detail_slug("POST /api/apps/core_mosquitto/detail HTTP/1.1"),
            None
        );
    }

    #[test]
    fn live_detail_requires_real_supervisor_not_signed_or_local_snapshots() {
        let sources = [Adapter {
            host: "any-vendor".into(),
            kind: AdapterKind::SupervisorSnapshot("runtime/fixture.json".into()),
        }];
        let (status, body) = live_detail_response(&sources, "core_mosquitto");
        assert_eq!(status, "404 Not Found");
        assert_eq!(
            serde_json::from_str::<Value>(&body).unwrap()["read_only"],
            true
        );
    }

    #[test]
    fn installed_stats_route_rejects_unsafe_slugs_and_snapshot_sources() {
        for version in ["HTTP/1.0", "HTTP/1.1"] {
            let line = format!("GET /api/apps/new_vendor_app/stats {version}");
            assert_eq!(live_stats_slug(&line), Some(Ok("new_vendor_app")));
        }
        for slug in ["", "..", "a/b", "x%2fsecret", "x?token=1"] {
            let line = format!("GET /api/apps/{slug}/stats HTTP/1.1");
            assert_eq!(live_stats_slug(&line), Some(Err("invalid app slug")));
        }
        assert_eq!(live_stats_slug("POST /api/apps/x/stats HTTP/1.1"), None);
        let snapshot = Adapter {
            host: "fixture".into(),
            kind: AdapterKind::SupervisorSnapshot("runtime/fixture.json".into()),
        };
        assert_eq!(
            live_stats_response(&[snapshot], "new_vendor_app").0,
            "404 Not Found"
        );
        let live = Adapter {
            host: "live".into(),
            kind: AdapterKind::SupervisorLive(ApiVersion::V1),
        };
        assert_eq!(
            live_stats_response(&[live.clone(), live], "new_vendor_app").0,
            "503 Service Unavailable"
        );
        assert_eq!(live_stats_response(&[], "../secret").0, "400 Bad Request");
        let html = include_str!("../../../web/index.html");
        assert!(html.contains("Ressources en direct"));
        assert!(html.contains("/stats"));
    }

    #[test]
    fn config_schema_is_get_only_for_arbitrary_vendor_and_never_uses_fixtures() {
        for http in ["HTTP/1.0", "HTTP/1.1"] {
            let route = format!("GET /api/apps/future_vendor_271/config-schema {http}");
            assert_eq!(live_config_slug(&route), Some(Ok("future_vendor_271")));
        }
        for bad in ["../x", "a%2fb", "x?secret=1", "a b"] {
            let route = format!("GET /api/apps/{bad}/config-schema HTTP/1.1");
            assert!(matches!(live_config_slug(&route), Some(Err(_))));
        }
        assert_eq!(
            live_config_slug("POST /api/apps/x/config-schema HTTP/1.1"),
            None
        );
        let lab = Adapter {
            host: "lab".into(),
            kind: AdapterKind::SupervisorSnapshot("fake".into()),
        };
        assert_eq!(
            live_config_response(&[lab], "future_vendor_271").0,
            "404 Not Found"
        );
        let live = Adapter {
            host: "one".into(),
            kind: AdapterKind::SupervisorLive(ApiVersion::V1),
        };
        assert_eq!(
            live_config_response(&[live.clone(), live], "future_vendor_271").0,
            "503 Service Unavailable"
        );
        assert_eq!(live_config_response(&[], "../bad").0, "400 Bad Request");
        assert!(include_str!("../../../web/index.html").contains("config-schema"));
    }

    #[test]
    fn lifecycle_operations_route_is_get_only_generic_and_fails_closed() {
        for version in ["HTTP/1.0", "HTTP/1.1"] {
            let line = format!("GET /api/store/apps/some_vendor_333/operations {version}");
            assert_eq!(store_operations_slug(&line), Some(Ok("some_vendor_333")));
        }
        for slug in ["", "..", "a/b", "x%2fsecret", "x?token=1"] {
            let line = format!("GET /api/store/apps/{slug}/operations HTTP/1.1");
            assert_eq!(
                store_operations_slug(&line),
                Some(Err("invalid store app slug"))
            );
        }
        assert_eq!(
            store_operations_slug("POST /api/store/apps/x/operations HTTP/1.1"),
            None
        );
        let lab = Adapter {
            host: "snapshot".into(),
            kind: AdapterKind::SupervisorSnapshot("runtime/fixture.json".into()),
        };
        assert_eq!(
            supervisor_store_operations_response(&[lab], "future_vendor_1").0,
            "404 Not Found"
        );
        let official = Adapter {
            host: "live".into(),
            kind: AdapterKind::SupervisorLive(ApiVersion::V2),
        };
        assert_eq!(
            supervisor_store_operations_response(&[official.clone(), official], "future_vendor_1")
                .0,
            "503 Service Unavailable"
        );
        assert_eq!(
            supervisor_store_operations_response(&[], "../unsafe").0,
            "400 Bad Request"
        );
        let html = include_str!("../../../web/index.html");
        assert!(html.contains("/operations"));
        assert!(html.contains("activation_authorized!==false"));
    }

    #[test]
    fn store_availability_route_is_read_only_and_rejects_unsafe_identifiers() {
        for version in ["HTTP/1.0", "HTTP/1.1"] {
            let line = format!("GET /api/store/apps/future_vendor_app/availability {version}");
            assert_eq!(
                store_availability_slug(&line),
                Some(Ok("future_vendor_app"))
            );
        }
        for slug in ["", "..", "a/b", "x%2fsecret", "x?token=1", "x#frag"] {
            let line = format!("GET /api/store/apps/{slug}/availability HTTP/1.1");
            assert_eq!(
                store_availability_slug(&line),
                Some(Err("invalid store app slug"))
            );
        }
        assert_eq!(
            store_availability_slug("POST /api/store/apps/x/availability HTTP/1.1"),
            None
        );
        let html = include_str!("../../../web/index.html");
        assert!(html.contains("api/store/apps/"));
        assert!(html.contains("/availability"));
        assert!(html.contains("activation_authorized!==false"));
    }

    #[test]
    fn availability_requires_exactly_one_live_supervisor_source() {
        let snapshot = Adapter {
            host: "synthetic".into(),
            kind: AdapterKind::SupervisorSnapshot("runtime/synthetic.json".into()),
        };
        assert_eq!(
            supervisor_store_availability_response(&[snapshot], "future_vendor_app").0,
            "404 Not Found"
        );
        let live = Adapter {
            host: "live".into(),
            kind: AdapterKind::SupervisorLive(ApiVersion::V1),
        };
        assert_eq!(
            supervisor_store_availability_response(&[live.clone(), live], "future_vendor_app").0,
            "503 Service Unavailable"
        );
        assert_eq!(
            supervisor_store_availability_response(&[], "../secret").0,
            "400 Bad Request"
        );
    }

    #[test]
    fn missing_store_app_does_not_trigger_detail_fetch() {
        use hars_supervisor_apps::official_store::{App, Catalog, Repo};
        use std::cell::Cell;
        let catalog = Catalog {
            api_version: ApiVersion::V1,
            repositories: vec![Repo {
                slug: "vendor".into(),
                name: "Vendor".into(),
            }],
            apps: vec![App {
                slug: "known_app".into(),
                repository: "vendor".into(),
                name: "Known".into(),
                available: true,
                installed: false,
                update_available: false,
            }],
            official_supervisor_owner: true,
            activation_authorized: false,
        };
        let called = Cell::new(false);
        let result = checked_store_detail(&catalog, "unknown_app", || {
            called.set(true);
            panic!("unlisted App detail must never be requested");
        });
        assert!(result.unwrap().is_none());
        assert!(!called.get());
    }

    #[test]
    fn store_details_are_reachable_from_real_dashboard_without_write_routes() {
        let html = include_str!("../../../web/index.html");
        assert!(html.contains("api/store/apps/"));
        assert!(html.contains("encodeURIComponent(app.slug)"));
        assert!(html.contains("data.app.activation_authorized!==false"));
        assert!(!html.contains("method:"));
    }

    #[test]
    fn store_details_route_supports_uninstalled_apps_without_traversal() {
        for slug in ["unlisted_vendor_2099", "core_mosquitto"] {
            for version in ["HTTP/1.0", "HTTP/1.1"] {
                let line = format!("GET /api/store/apps/{slug}/detail {version}");
                assert_eq!(store_detail_slug(&line), Some(Ok(slug)));
            }
        }
        for slug in ["", "..", "a/b", "x%2fprivate", "x?token=1", "x#fragment"] {
            let line = format!("GET /api/store/apps/{slug}/detail HTTP/1.1");
            assert_eq!(
                store_detail_slug(&line),
                Some(Err("invalid store app slug"))
            );
        }
        assert_eq!(
            store_detail_slug("POST /api/store/apps/vendor/detail HTTP/1.1"),
            None
        );
    }

    #[test]
    fn store_details_require_live_supervisor_not_snapshot_or_ambiguous_source() {
        let snapshot = Adapter {
            host: "synthetic".into(),
            kind: AdapterKind::SupervisorSnapshot("runtime/synthetic.json".into()),
        };
        assert_eq!(
            supervisor_store_detail_response(&[snapshot], "future_vendor").0,
            "404 Not Found"
        );
        let live = Adapter {
            host: "live".into(),
            kind: AdapterKind::SupervisorLive(ApiVersion::V1),
        };
        assert_eq!(
            supervisor_store_detail_response(&[live.clone(), live], "future_vendor").0,
            "503 Service Unavailable"
        );
    }

    #[test]
    fn official_store_is_unavailable_for_snapshots_or_untrusted_env() {
        let snapshots = [Adapter {
            host: "fixture".into(),
            kind: AdapterKind::SupervisorSnapshot("runtime/fixture.json".into()),
        }];
        let (status, body) = supervisor_store_response(&snapshots);
        assert_eq!(status, "404 Not Found");
        assert_eq!(
            serde_json::from_str::<Value>(&body).unwrap()["read_only"],
            true
        );
        let live = [Adapter {
            host: "future".into(),
            kind: AdapterKind::SupervisorLive(ApiVersion::V1),
        }];
        // Never interpret missing Supervisor token as an empty Store catalogue.
        // This test only holds if the environment has no Supervisor token.
        if std::env::var_os("SUPERVISOR_TOKEN").is_none() {
            let (status, _) = supervisor_store_response(&live);
            assert_eq!(status, "503 Service Unavailable");
        }
    }

    #[test]
    fn safe_detail_json_never_includes_app_options_or_credentials() {
        let raw = json!({"result":"ok","data":{
            "slug":"unfamiliar_vendor_a", "name":"Third-party App",
            "state":"started","version":"3.8",
            "options":{"password":"MUST_NOT_LEAK"},
            "devices":["/dev/private"],"ingress_token":"SECRET",
            "system_managed":false
        }})
        .to_string();
        let app = hars_supervisor_apps::app_detail::parse_app_detail(
            raw.as_bytes(),
            "unfamiliar_vendor_a",
        )
        .unwrap();
        let serialized = json!({"read_only":true,"app":app}).to_string();
        assert!(serialized.contains("unfamiliar_vendor_a"));
        assert_eq!(
            serde_json::from_str::<Value>(&serialized).unwrap()["app"]["activation_authorized"],
            false
        );
        for forbidden in [
            "MUST_NOT_LEAK",
            "SECRET",
            "/dev/private",
            "password",
            "devices",
        ] {
            assert!(!serialized.contains(forbidden));
        }
    }

    #[test]
    fn multi_host_aggregation_is_namespaced() {
        let adapters = vec![
            Adapter {
                host: "lab-node".into(),
                kind: AdapterKind::DockerFixture(project_file("fixtures/docker-ps.jsonl")),
            },
            Adapter {
                host: "lab-guest".into(),
                kind: AdapterKind::SupervisorSnapshot(project_file("audits/haos-apps.json")),
            },
        ];
        let report = aggregate(&adapters);
        assert_eq!(report["apps"].as_array().unwrap().len(), 13);
        assert_eq!(report["sources"].as_array().unwrap().len(), 2);
        assert_eq!(report["degraded"], false);
        assert_ne!(report["apps"][0]["id"], report["apps"][1]["id"]);
    }

    #[test]
    fn one_failed_source_does_not_hide_successful_ones() {
        let report = aggregate(&[
            Adapter {
                host: "ok".into(),
                kind: AdapterKind::SupervisorSnapshot(project_file("audits/haos-apps.json")),
            },
            Adapter {
                host: "down".into(),
                kind: AdapterKind::SupervisorSnapshot("/path/does/not/exist".into()),
            },
        ]);
        assert_eq!(report["degraded"], true);
        assert_eq!(report["apps"].as_array().unwrap().len(), 9);
        assert_eq!(report["sources"][1]["status"], "unavailable");
        assert!(!report.to_string().contains("/path/does/not/exist"));
    }

    #[test]
    fn rejects_unrecognized_json() {
        assert!(docker_rows("{bad", "lab").is_err());
        assert!(supervisor_rows("{bad", "haos").is_err());
    }

    #[test]
    fn manifest_only_accepts_runtime_snapshots() {
        let body = r#"{"sources":[
          {"host":"lab-guest","kind":"supervisor-snapshot","path":"runtime/haos-lab-guest.json"},
          {"host":"ct400","kind":"docker-fixture","path":"runtime/docker-ct400.jsonl"}
        ]}"#;
        let sources = manifest_sources(body).unwrap();
        assert_eq!(sources.len(), 2);
        assert_eq!(sources[0].host, "lab-guest");
        assert!(manifest_sources(
            r#"{"sources":[{"host":"ct1","kind":"docker-fixture","path":"../../.ssh/id_rsa"}]}"#
        )
        .is_err());
        assert!(manifest_sources(
            r#"{"sources":[{"host":"ct1","kind":"docker-fixture","path":"runtime/../secrets"}]}"#
        )
        .is_err());
        assert!(manifest_sources(
            r#"{"sources":[{"host":"ct1","kind":"invalid","path":"runtime/a"}]}"#
        )
        .is_err());
    }

    #[test]
    fn host_label_validation() {
        assert!(valid_host("labhost_dev"));
        assert!(!valid_host(""));
        assert!(!valid_host("127.0.0.1/../"));
        assert!(!valid_host("a\nheader"));
    }
}
