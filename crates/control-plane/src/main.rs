//! Rusty Home P3b prototype: read-only, multi-source application inventory.
//! No Docker socket exposed, no write paths, no HAOS device control or automation.
mod attestation;
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
                    "live":matches!(a.kind, AdapterKind::DockerCli),
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

// Bounded, independent request handling: 8 workers, at most 32 pending.
// No unbounded thread-per-connection allocation or blocking send in accept().
const LAB_HTTP_WORKERS: usize = 8;
const LAB_HTTP_QUEUE: usize = 32;

fn handle(
    mut conn: TcpStream,
    adapters: &[Adapter],
    signed: Option<&SignedContext>,
    core: Option<&CoreSidecar>,
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
    let line = match parse_local_http_request(&buffer[..size], local) {
        Ok(line) => line,
        Err(_) => {
            let _ = reply(
                &mut conn,
                "400 Bad Request",
                r#"{"error":"invalid local HTTP request"}"#,
                "application/json; charset=utf-8",
            );
            return;
        }
    };
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
    } else if line == "GET /api/apps HTTP/1.1" || line == "GET /api/apps HTTP/1.0" {
        ("200 OK", report(adapters, verified).to_string())
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
        return Err("select at least one of --docker, --fixture FILE, --haos-snapshot FILE".into());
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
                    AdapterKind::DockerCli => return Err("signed mode needs snapshots".into()),
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
    if let Some(addr) = bind {
        let addr: SocketAddr = addr.parse().map_err(|_| "invalid bind address")?;
        if !matches!(addr.ip(), IpAddr::V4(v) if v.is_loopback())
            && !matches!(addr.ip(), IpAddr::V6(v) if v.is_loopback())
        {
            return Err("the lab API must be bound to a loopback address only".into());
        }
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
    use super::*;

    #[test]
    fn bounded_lab_http_resource_policy() {
        assert_eq!(LAB_HTTP_WORKERS, 8);
        assert_eq!(LAB_HTTP_QUEUE, 32);
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
