//! Opt-in localhost-only, read-only lab daemon using native Moli Hub.
//! NOT a Home Assistant, authentication, durability, or production release.
use super::{Synthetic, ready};
use anyhow::{Context, Result, bail};
use moli_core::{InstanceId, Origin, PointId, Value};
use moli_runtime::{Hub, HubOptions, spawn_driver};
use nix::sys::stat::{Mode, umask};
use nix::unistd::Uid;
use std::fs;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use subtle::ConstantTimeEq;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_util::sync::CancellationToken;

const MAX_HEADER: usize = 4096;
const READ_TIMEOUT: Duration = Duration::from_secs(2);
const MAX_CONNECTIONS: usize = 32;

fn check_private_dir(p: &Path) -> Result<()> {
    let m = fs::symlink_metadata(p).context("private state directory unavailable")?;
    if !m.is_dir()
        || m.file_type().is_symlink()
        || m.uid() != Uid::current().as_raw()
        || m.permissions().mode() & 0o077 != 0
    {
        bail!("state directory must be real, owned and mode 0700")
    }
    Ok(())
}

fn read_private_token(dir: &Path, p: &Path) -> Result<[u8; 64]> {
    check_private_dir(dir)?;
    if !p.is_absolute() || p.parent() != Some(dir) {
        bail!("token path must be directly within private state directory");
    }
    let m = fs::symlink_metadata(p).context("token file unavailable")?;
    if !m.is_file()
        || m.file_type().is_symlink()
        || m.uid() != Uid::current().as_raw()
        || m.permissions().mode() & 0o077 != 0
        || m.len() != 64
    {
        bail!("token must be private 0600, regular, owner-matched and exactly 64 ASCII hex bytes")
    }
    let raw = fs::read(p)?;
    if raw.len() != 64 || !raw.iter().all(u8::is_ascii_hexdigit) {
        bail!("token must contain exactly 64 hex bytes with no newline")
    }
    let mut token = [0u8; 64];
    token.copy_from_slice(&raw);
    Ok(token)
}

/// Parse a bounded HTTP/1.1 request; returns false unless exactly one valid
/// Bearer header is present. Neither tokens nor household data are logged.
fn authorized(request: &[u8], secret: &[u8; 64]) -> bool {
    let Ok(text) = std::str::from_utf8(request) else {
        return false;
    };
    let Some(headers) = text.strip_suffix("\r\n\r\n") else {
        return false;
    };
    let mut lines = headers.split("\r\n");
    if !lines.next().is_some_and(|r| {
        matches!(
            r,
            "GET /health HTTP/1.1"
                | "GET /api/native/state HTTP/1.1"
                | "POST /api/native/state HTTP/1.1"
        )
    }) {
        return false;
    }
    let mut found = 0;
    let mut authorized = false;
    for line in lines {
        if let Some((k, v)) = line.split_once(':') {
            if k.eq_ignore_ascii_case("Authorization") {
                found += 1;
                if let Some(s) = v.trim().strip_prefix("Bearer ") {
                    authorized = s.len() == 64 && bool::from(s.as_bytes().ct_eq(secret.as_slice()));
                }
            }
        } else {
            return false;
        }
    }
    found == 1 && authorized
}

async fn serve_one(mut stream: TcpStream, hub: &Hub, token: &[u8; 64]) -> Result<()> {
    stream.set_nodelay(true)?;
    let mut buf = [0u8; MAX_HEADER + 1];
    let mut len = 0;
    let mut too_large = false;
    let mut complete = false;
    while len < buf.len() {
        let n = match tokio::time::timeout(READ_TIMEOUT, stream.read(&mut buf[len..])).await {
            Ok(Ok(n)) => n,
            _ => 0,
        };
        if n == 0 {
            break;
        }
        len += n;
        if buf[..len].windows(4).any(|x| x == b"\r\n\r\n") {
            complete = true;
            break;
        }
    }
    if len > MAX_HEADER {
        too_large = true
    }
    let raw = &buf[..len];
    let (status, body) = if !complete || too_large {
        (400, serde_json::json!({"error":"bad_request"}))
    } else if !authorized(raw, token) {
        (401, serde_json::json!({"error":"unauthorized"}))
    } else {
        let first = std::str::from_utf8(raw)?.split("\r\n").next().unwrap_or("");
        match first {
            "GET /health HTTP/1.1" => (
                200,
                serde_json::json!({"ready":true,"native_rust":true,"production_ready":false}),
            ),
            "GET /api/native/state HTTP/1.1" => (
                200,
                serde_json::json!({
                    "native_rust":true,
                    "source":"moli-runtime",
                    "devices":hub.snapshot().devices.len(),
                    "sensor_on":hub.state(&PointId::from("hars-lab:sensor/on")).map(|v|v.value)==Some(Value::Bool(true)),
                    "actuator_on":hub.state(&PointId::from("hars-lab:actuator/on")).map(|v|v.value)==Some(Value::Bool(true)),
                    "read_only":true,
                    "production_ready":false
                }),
            ),
            _ => (
                405,
                serde_json::json!({"error":"read_only","read_only":true}),
            ),
        }
    };
    let body = body.to_string();
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        405 => "Method Not Allowed",
        _ => "Error",
    };
    let reply = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(reply.as_bytes()).await?;
    stream.shutdown().await?;
    Ok(())
}

/// Harden Moli's atomically rewritten cache after each explicit flush.
/// A 0700 parent prevents other Unix users accessing the file before chmod.
/// Does not solve same-UID TOCTOU or provide transaction durability.
fn protect_saved_cache(state_dir: &Path, state_path: &Path) -> Result<()> {
    check_private_dir(state_dir)?;
    let m = fs::symlink_metadata(state_path)?;
    if !m.is_file()
        || m.file_type().is_symlink()
        || m.uid() != Uid::current().as_raw()
        || m.len() > 32 * 1024 * 1024
    {
        bail!("upstream native state cache is unsafe")
    }
    fs::set_permissions(state_path, fs::Permissions::from_mode(0o600))?;
    let m = fs::symlink_metadata(state_path)?;
    if !m.is_file() || m.file_type().is_symlink() || m.permissions().mode() & 0o077 != 0 {
        bail!("upstream native state cache permissions were not hardened")
    }
    fs::File::open(state_path)?.sync_all()?;
    fs::File::open(state_dir)?.sync_all()?;
    Ok(())
}

/// Explicit single-process localhost lab daemon. The token must be generated
/// externally and secured by the operator. This is NOT full user RBAC/TLS.
pub async fn run_lab_daemon(
    state_dir: &Path,
    token_path: &Path,
    port: u16,
    seed_synthetic: bool,
) -> Result<()> {
    if Uid::current().is_root() {
        bail!("refusing to start lab daemon as root")
    }
    if port == 0 {
        bail!("lab daemon requires an explicit nonzero localhost port")
    }
    check_private_dir(state_dir)?;
    let token = read_private_token(state_dir, token_path)?;
    // Upstream Moli uses fs::write and isolated Linux lab can have umask 0002; constrain
    // all future state files to owner-only permissions before starting Hub.
    umask(Mode::from_bits_truncate(0o077));
    let state_path = state_dir.join("native-states.json");
    if state_path.exists() {
        let m = fs::symlink_metadata(&state_path)?;
        if !m.is_file()
            || m.file_type().is_symlink()
            || m.uid() != Uid::current().as_raw()
            || m.permissions().mode() & 0o077 != 0
        {
            bail!("unsafe native state cache path")
        }
    }
    let hub = Hub::new(HubOptions {
        state_path: Some(state_path.clone()),
        ..HubOptions::default()
    })?;
    let cancel = CancellationToken::new();
    let task = spawn_driver(
        &hub,
        InstanceId::from("hars-lab"),
        Arc::new(Synthetic),
        cancel.clone(),
    );
    ready(&hub).await?;
    if seed_synthetic {
        hub.command(
            &PointId::from("hars-lab:sensor/on"),
            Value::Bool(true),
            Origin::Ui,
            None,
        )
        .await
        .context("synthetic seed failed")?;
        if !hub.persist_state().await? {
            bail!("synthetic seed could not be persisted")
        }
        protect_saved_cache(state_dir, &state_path)?;
    }
    // No binding to 0.0.0.0 / IPv6 / LAN, even with CLI manipulation.
    let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, port)).await?;
    println!("LAB_ONLY_READY port={port} native_rust=true production_ready=false");
    let stop = tokio::signal::ctrl_c();
    tokio::pin!(stop);
    let mut accepted = 0usize;
    loop {
        tokio::select! {
            result=listener.accept()=>{
                let (stream,_)=result?;
                let _=serve_one(stream,&hub,&token).await;
                accepted+=1;
                if accepted>=MAX_CONNECTIONS {
                    // Bounded lab transport; operators must restart by design.
                    break;
                }
            }
            _=&mut stop=>break,
        }
    }
    let _ = hub.persist_state().await?;
    protect_saved_cache(state_dir, &state_path)?;
    cancel.cancel();
    let _ = tokio::time::timeout(Duration::from_secs(3), task).await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn token_permissions_and_symlinks_rejected() {
        let dir = tempfile::tempdir().unwrap();
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let token = dir.path().join("capability");
        let mut f = File::create(&token).unwrap();
        f.write_all(&[b'a'; 64]).unwrap();
        fs::set_permissions(&token, fs::Permissions::from_mode(0o600)).unwrap();
        assert!(read_private_token(dir.path(), &token).is_ok());
        fs::set_permissions(&token, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(read_private_token(dir.path(), &token).is_err());
        fs::set_permissions(&token, fs::Permissions::from_mode(0o600)).unwrap();
        let bad = dir.path().join("link");
        std::os::unix::fs::symlink(&token, &bad).unwrap();
        assert!(read_private_token(dir.path(), &bad).is_err());
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o755)).unwrap();
        assert!(read_private_token(dir.path(), &token).is_err());
    }

    #[test]
    fn http_bearer_contract_fail_closed() {
        let good = [b'a'; 64];
        let valid = format!(
            "GET /api/native/state HTTP/1.1\r\nHost: 127.0.0.1\r\nAuthorization: Bearer {}\r\n\r\n",
            "a".repeat(64)
        );
        assert!(authorized(valid.as_bytes(), &good));
        assert!(!authorized(
            valid.replace("Bearer ", "Bearer b").as_bytes(),
            &good
        ));
        assert!(!authorized(
            valid.replace("Authorization", "X-Authorization").as_bytes(),
            &good
        ));
        assert!(!authorized(
            valid
                .replace("\r\n\r\n", "\r\nAuthorization: Bearer a\r\n\r\n")
                .as_bytes(),
            &good
        ));
        assert!(!authorized(valid.replace("GET ", "PUT ").as_bytes(), &good));
    }
}
