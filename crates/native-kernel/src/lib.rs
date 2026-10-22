//! First Rust-native pipeline milestone. No network, Python or HAOS dependencies.
//! Reuses Moli OS upstream hub, supervised driver and deterministic automation.
//! Inert synthetic driver ONLY; it is NOT an operational replacement for HAOS.
pub mod automation_moli;
pub mod automation_yaml;
pub mod command_ledger;
pub mod daemon;
pub mod driver_fencing;
pub mod fencing;
pub mod mqtt5_rap;
pub mod mqtt5_reconcile;
pub mod mqtt_discovery;
pub mod mqtt_hub;
pub mod mqtt_live;
pub mod mqtt_registry;
pub mod radio_lease_lab;
use anyhow::{Context, Result, bail};
use moli_automation::{Actor, Author, Automation, Automations, Config, Graph, Mode};
use moli_core::{
    Access, Device, DriverStatus, Event, InstanceId, Kind, Origin, PointId, PointSpec, Semantic,
    Value,
};
use moli_runtime::{BoxFuture, Driver, DriverCtx, Hub, HubOptions, spawn_driver};
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

#[derive(Clone, Debug)]
pub struct NativeProof {
    pub trigger_observed: bool,
    pub automation_live: bool,
    pub actuator_state_confirmed: bool,
    pub restored_sensor: bool,
    pub restored_actuator: bool,
    pub events_seen: usize,
    pub journal_entries_recovered: usize,
    pub http_read_ok: bool,
    pub http_mutation_denied: bool,
    pub used_python: bool,
}

fn binary_point() -> PointSpec {
    PointSpec {
        key: "on".into(),
        label: "On".into(),
        kind: Kind::Binary,
        access: Access {
            read: true,
            write: true,
        },
        unit: None,
        semantic: Semantic::OnOff,
    }
}

struct Synthetic;
impl Driver for Synthetic {
    fn kind(&self) -> &'static str {
        "hars-synthetic-native"
    }
    fn run<'a>(&'a self, ctx: &'a mut DriverCtx) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            for name in ["sensor", "actuator"] {
                let id = ctx.device_id(name);
                ctx.upsert_device(Device {
                    id: id.clone(),
                    instance: ctx.instance().clone(),
                    native_name: name.into(),
                    manufacturer: None,
                    model: None,
                    description: None,
                    native_room: None,
                    members: vec![],
                    points: vec![binary_point()],
                });
                // Never overwrite a restored state when a driver reconnects.
                if ctx.current(&id, "on").is_none() {
                    ctx.set_state(&id, "on", Value::Bool(false));
                }
            }
            ctx.ready();
            while let Some(command) = ctx.next_command().await {
                // Only a device owned by this synthetic driver can be written:
                // DriverCtx and Hub enforce instance and point scopes.
                ctx.set_state(&command.device.id, &command.key, command.value.clone());
                command.reply(Ok(()));
            }
            Ok(())
        })
    }
}

fn human() -> Actor {
    Actor {
        human: true,
        author: Author::Human,
    }
}

async fn ready(hub: &Hub) -> Result<()> {
    let point = PointId::from("hars-lab:sensor/on");
    for _ in 0..200 {
        let snapshot = hub.snapshot();
        let driver_running = snapshot
            .drivers
            .iter()
            .any(|d| matches!(d.status, DriverStatus::Running));
        let both_registered = hub
            .device(&moli_core::DeviceId::from("hars-lab:sensor"))
            .is_some()
            && hub
                .device(&moli_core::DeviceId::from("hars-lab:actuator"))
                .is_some();
        if driver_running
            && both_registered
            && hub.state(&point).is_some()
            && hub.state(&PointId::from("hars-lab:actuator/on")).is_some()
        {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    bail!("synthetic native hub driver never became ready")
}

async fn wait_for(hub: &Hub, point: &str, on: bool) -> Result<()> {
    for _ in 0..100 {
        if hub.state(&PointId::from(point)).map(|s| s.value) == Some(Value::Bool(on)) {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    bail!("native actuator state {point} never reached {on}")
}
fn rule() -> Automation {
    let graph: Graph = serde_json::from_value(serde_json::json!({
        "nodes":[
            {"id":"trigger","type":"when_state","point":"hars-lab:sensor/on","to":true},
            {"id":"action","type":"set","point":"hars-lab:actuator/on","value":true}
        ],
        "edges":[{"from":"trigger","to":"action"}]
    }))
    .expect("constant automation must decode");
    Automation {
        id: String::new(),
        name: "Proof: sensor triggers synthetic actuator".into(),
        enabled: false,
        mode: Mode::Restart,
        graph,
        author: Author::Human,
        approved: None,
        approved_version: None,
        note: None,
        created: 0,
        updated: 0,
    }
}

/// Strict laboratory preflight: upstream JournalLog filters malformed JSON lines
/// while reading. A future production audit trail must never silently drop them.
/// This does not prevent a concurrent path swap (TOCTOU); file-descriptor based
/// reads and an atomic transaction log remain mandatory before any deployment.
fn validate_existing_journal(path: &std::path::Path) -> Result<()> {
    let meta = match std::fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e.into()),
    };
    if !meta.is_file() || meta.file_type().is_symlink() || meta.len() > 32 * 1024 * 1024 {
        bail!("native journal is not a safe regular file");
    }
    let data = std::fs::read(path)?;
    if !data.is_empty() && data.last() != Some(&b'\n') {
        bail!("native journal ends with an incomplete line");
    }
    for row in data
        .split(|byte| *byte == b'\n')
        .filter(|row| !row.is_empty())
    {
        if row.len() > 16 * 1024 {
            bail!("oversized native audit entry");
        }
        let _: moli_core::JournalEntry = serde_json::from_slice(row)
            .context("corrupt native journal entry refused, original preserved")?;
    }
    Ok(())
}

/// Runs the actual upstream Moli supervised driver and automation manager in
/// this Rust process; no Home Assistant/Python code loaded. Creates only
/// synthetic devices, using the caller-provided disposable directory.
pub async fn native_cycle(folder: &std::path::Path) -> Result<NativeProof> {
    let state_path = folder.join("native-states.json");
    let journal_path = folder.join("native-journal.jsonl");
    validate_existing_journal(&journal_path)?;
    let hub = Hub::new(HubOptions {
        state_path: Some(state_path.clone()),
        journal_path: Some(journal_path.clone()),
        ..HubOptions::default()
    })?;
    let cancel = CancellationToken::new();
    let driver = spawn_driver(
        &hub,
        InstanceId::from("hars-lab"),
        Arc::new(Synthetic),
        cancel.clone(),
    );
    ready(&hub).await?;
    let mut events = hub.subscribe();
    let (autos, engine) =
        Automations::start(&hub, Config::default(), Some(folder), cancel.clone()).await?;
    let saved = autos.save(rule(), human()).await?;
    if saved.is_live() {
        bail!("automation must require an explicit approval");
    }
    let approved = autos
        .approve(&saved.id, &saved.fingerprint(), human())
        .await?;
    if !approved.is_live() {
        bail!("human-approved automation not active");
    }
    // Engine subscribes before command. Allow its first state snapshot to settle.
    tokio::time::sleep(Duration::from_millis(70)).await;
    hub.command(
        &PointId::from("hars-lab:sensor/on"),
        Value::Bool(true),
        Origin::Ui,
        None,
    )
    .await
    .context("driver refused synthetic sensor command")?;
    wait_for(&hub, "hars-lab:actuator/on", true).await?;

    let mut state_events = 0;
    while let Ok(event) = events.try_recv() {
        if matches!(event, Event::State { .. }) {
            state_events += 1;
        }
    }
    if state_events < 2 {
        bail!("expected event bus events from sensor AND actuator, got {state_events}");
    }
    let (http_read_ok, http_mutation_denied) = verify_local_api(&hub).await?;
    if !hub.persist_state().await? {
        bail!("native hub did not flush dirty state");
    }
    // Upstream Moli journal is append-only and asynchronous, NOT a commit log.
    // Await its actual on-disk entries before shutdown; use a bounded deadline.
    let mut journal_rows = 0;
    for _ in 0..150 {
        if let Ok(raw) = std::fs::read_to_string(&journal_path) {
            let rows: Vec<_> = raw.lines().collect();
            if rows.len() >= 2 {
                for row in &rows {
                    let _: serde_json::Value = serde_json::from_str(row)?;
                }
                journal_rows = rows.len();
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    if journal_rows < 2 {
        bail!("native upstream journal did not write the two service events");
    }
    std::fs::File::open(&journal_path)?.sync_all()?;
    std::fs::File::open(&state_path)?.sync_all()?;
    std::fs::File::open(folder)?.sync_all()?;
    cancel.cancel();
    // Supervisor/automation tasks own no external resources.
    let _ = tokio::time::timeout(Duration::from_secs(3), driver).await;
    let _ = tokio::time::timeout(Duration::from_secs(3), engine).await;
    drop(autos);
    drop(hub);

    validate_existing_journal(&journal_path)?;
    let restarted = Hub::new(HubOptions {
        state_path: Some(state_path),
        journal_path: Some(journal_path),
        ..HubOptions::default()
    })?;
    let shutdown = CancellationToken::new();
    let task = spawn_driver(
        &restarted,
        InstanceId::from("hars-lab"),
        Arc::new(Synthetic),
        shutdown.clone(),
    );
    ready(&restarted).await?;
    let restored_sensor = restarted
        .state(&PointId::from("hars-lab:sensor/on"))
        .map(|s| s.value)
        == Some(Value::Bool(true));
    let restored_actuator = restarted
        .state(&PointId::from("hars-lab:actuator/on"))
        .map(|s| s.value)
        == Some(Value::Bool(true));
    shutdown.cancel();
    let _ = tokio::time::timeout(Duration::from_secs(3), task).await;
    let journal_entries_recovered = restarted.journal(500).len();
    if journal_entries_recovered < 2 {
        bail!("native journal entries missing after restart");
    }
    if !(restored_sensor && restored_actuator) {
        bail!("native state not recovered after restart");
    }
    Ok(NativeProof {
        trigger_observed: true,
        automation_live: true,
        actuator_state_confirmed: true,
        restored_sensor,
        restored_actuator,
        events_seen: state_events,
        journal_entries_recovered,
        http_read_ok,
        http_mutation_denied,
        used_python: false,
    })
}

/// Minimal, temporary loopback-only diagnostic surface for the in-memory Moli
/// model. No authenticator, no mutation route: NEVER expose outside isolated isolated Linux lab.
async fn verify_local_api(hub: &Hub) -> Result<(bool, bool)> {
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    let listener = TcpListener::bind("127.0.0.1:0")?;
    listener.set_nonblocking(true)?;
    let addr = listener.local_addr()?;
    let hub = hub.clone();
    let server = std::thread::spawn(move || -> Result<()> {
        let start = std::time::Instant::now();
        let mut processed = 0;
        while processed < 3 && start.elapsed() < Duration::from_secs(4) {
            let (mut client, _) = match listener.accept() {
                Ok(pair) => pair,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(5));
                    continue;
                }
                Err(e) => return Err(e.into()),
            };
            client.set_read_timeout(Some(Duration::from_secs(2)))?;
            let mut request = [0u8; 4096];
            let n = client.read(&mut request)?;
            let first = String::from_utf8_lossy(&request[..n]);
            let path = first.lines().next().unwrap_or("");
            let (code, body) = match path {
                "GET /health HTTP/1.1" => {
                    (200, serde_json::json!({"healthy":true,"native_rust":true}))
                }
                "GET /api/native/state HTTP/1.1" => {
                    let values = hub.snapshot();
                    (
                        200,
                        serde_json::json!({
                            "native_rust":true,
                            "source":"moli-runtime",
                            "devices":values.devices.len(),
                            "sensor_on":hub.state(&PointId::from("hars-lab:sensor/on")).map(|v|v.value)==Some(Value::Bool(true)),
                            "actuator_on":hub.state(&PointId::from("hars-lab:actuator/on")).map(|v|v.value)==Some(Value::Bool(true)),
                            "read_only":true,
                            "production_ready":false,
                        }),
                    )
                }
                path if path.starts_with("POST ")
                    || path.starts_with("PUT ")
                    || path.starts_with("DELETE ") =>
                {
                    (
                        405,
                        serde_json::json!({"error":"read_only","read_only":true}),
                    )
                }
                _ => (404, serde_json::json!({"error":"not_found"})),
            };
            let body = body.to_string();
            let http = format!(
                "HTTP/1.1 {code} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\n\r\n{body}",
                if code == 200 {
                    "OK"
                } else if code == 405 {
                    "Method Not Allowed"
                } else {
                    "Not Found"
                },
                body.len()
            );
            client.write_all(http.as_bytes())?;
            client.flush()?;
            processed += 1;
        }
        if processed != 3 {
            bail!("native lab listener received only {processed} of 3 checks")
        }
        Ok(())
    });
    let requests = [
        "GET /health HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
        "GET /api/native/state HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
        "POST /api/native/state HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\nContent-Length: 2\r\n\r\n{}",
    ];
    let response = tokio::task::spawn_blocking(move || -> Result<Vec<String>> {
        let mut out = Vec::new();
        for request in requests {
            let mut client = TcpStream::connect_timeout(&addr, Duration::from_secs(2))?;
            client.set_read_timeout(Some(Duration::from_secs(2)))?;
            client.write_all(request.as_bytes())?;
            let mut raw = String::new();
            client.read_to_string(&mut raw)?;
            out.push(raw);
        }
        Ok(out)
    })
    .await??;
    server
        .join()
        .map_err(|_| anyhow::anyhow!("native HTTP lab thread panicked"))??;
    let get_ok = response[0].starts_with("HTTP/1.1 200")
        && response[1].starts_with("HTTP/1.1 200")
        && response[1].contains("\"sensor_on\":true")
        && response[1].contains("\"actuator_on\":true")
        && response[1].contains("\"production_ready\":false");
    let mutation_denied =
        response[2].starts_with("HTTP/1.1 405") && response[2].contains("\"read_only\":true");
    if !get_ok || !mutation_denied {
        bail!("native Rust HTTP loopback checks not satisfied");
    }
    Ok((get_ok, mutation_denied))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn approved_native_rule_emits_events_and_restores_without_python() {
        let root = tempfile::tempdir().unwrap();
        let p = native_cycle(root.path()).await.unwrap();
        assert!(
            p.automation_live
                && p.actuator_state_confirmed
                && p.restored_sensor
                && p.restored_actuator
        );
        assert!(!p.used_python);
        assert!(p.events_seen >= 2);
        assert!(p.journal_entries_recovered >= 2);
        assert!(p.http_read_ok && p.http_mutation_denied);
    }
    #[test]
    fn corrupted_and_symlinked_native_journal_fail_closed() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("native-journal.jsonl");
        assert!(validate_existing_journal(&p).is_ok());
        std::fs::write(&p, b"{bad json}\\n").unwrap();
        assert!(validate_existing_journal(&p).is_err());
        std::fs::write(&p, b"{} ").unwrap();
        assert!(validate_existing_journal(&p).is_err());
        std::fs::remove_file(&p).unwrap();
        let outside = dir.path().join("other.jsonl");
        std::fs::write(&outside, b"").unwrap();
        std::os::unix::fs::symlink(&outside, &p).unwrap();
        assert!(validate_existing_journal(&p).is_err());
    }
    #[tokio::test]
    async fn unapproved_rule_cannot_be_declared_live() {
        assert!(!rule().is_live());
    }
}
