//! Minimal local Unix-domain read-only bridge from Rust to Python.
//! Purpose: preserve Home Assistant Core/Python/HACS authoritative state.
//! Auth uses BOTH filesystem confinement and SO_PEERCRED, plus an opaque
//! per-fixture capability token. No real home devices or apps are accessed.
//!
//! This is NOT remote authenticated transport or production-grade RBAC.
use anyhow::{Context, Result, bail};
use nix::sys::socket::{getsockopt, sockopt::PeerCredentials};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

const MAX_RESPONSE: usize = 1_048_576;
const MAX_STATES: usize = 10_000;
const TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EntityState {
    pub entity_id: String,
    pub state: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub protocol: String,
    pub read_only: bool,
    pub generation: u64,
    pub entities: Vec<EntityState>,
}

#[derive(Serialize)]
struct Request<'a> {
    protocol: &'static str,
    op: &'static str,
    token: &'a str,
}

fn check_private_regular(path: &Path, expected_uid: u32) -> Result<File> {
    let parent = path.parent().context("capability parent missing")?;
    let dir = fs::symlink_metadata(parent).context("capability parent absent")?;
    if !dir.file_type().is_dir()
        || dir.uid() != expected_uid
        || dir.permissions().mode() & 0o077 != 0
    {
        bail!("capability parent must be a private real directory");
    }
    // Opening first with O_NOFOLLOW and checking the *opened descriptor*
    // avoids trusting a path stat that may be swapped before File::open.
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(nix::libc::O_NOFOLLOW | nix::libc::O_CLOEXEC)
        .open(path)
        .context("cannot safely open private capability")?;
    let meta = file.metadata().context("cannot fstat capability")?;
    if !meta.file_type().is_file()
        || meta.uid() != expected_uid
        || meta.permissions().mode() & 0o077 != 0
        || meta.len() > 256
    {
        bail!("capability descriptor owner, type, mode or length not allowed");
    }
    Ok(file)
}

fn check_socket(path: &Path, expected_uid: u32) -> Result<()> {
    if !path.is_absolute() {
        bail!("UDS path must be absolute");
    }
    let parent = path.parent().context("UDS parent missing")?;
    let root = fs::symlink_metadata(parent).context("UDS parent absent")?;
    if !root.is_dir() || root.uid() != expected_uid || root.permissions().mode() & 0o077 != 0 {
        bail!("socket directory is not private");
    }
    let socket = fs::symlink_metadata(path).context("UDS endpoint unavailable")?;
    if !socket.file_type().is_socket()
        || socket.uid() != expected_uid
        || socket.permissions().mode() & 0o077 != 0
    {
        bail!("socket must be private owned socket, not symlink or ordinary file");
    }
    Ok(())
}

/// Client-side read-only projection, size-bounded and short-lived.
/// Caller must independently invalidate prior sessions before each reconnect.
pub fn fetch_snapshot(
    socket_path: &Path,
    capability_path: &Path,
    expected_uid: u32,
) -> Result<Snapshot> {
    check_socket(socket_path, expected_uid)?;
    if capability_path.parent() != socket_path.parent() {
        bail!("capability and sidecar socket must share one private directory");
    }
    let capability_file = check_private_regular(capability_path, expected_uid)?;
    let mut secret = String::new();
    capability_file.take(257).read_to_string(&mut secret)?;
    if secret.len() < 32
        || secret.len() > 128
        || !secret.is_ascii()
        || secret
            .chars()
            .any(|c| !c.is_ascii_alphanumeric() && c != '-' && c != '_')
    {
        bail!("invalid capability token format");
    }
    let mut peer = UnixStream::connect(socket_path).context("sidecar UDS connect failed")?;
    peer.set_read_timeout(Some(TIMEOUT))?;
    peer.set_write_timeout(Some(TIMEOUT))?;
    let creds = getsockopt(&peer, PeerCredentials).context("SO_PEERCRED unavailable")?;
    if creds.uid() != expected_uid {
        bail!("sidecar peer UID mismatch");
    }
    let mut request = serde_json::to_vec(&Request {
        protocol: "hars-sidecar-v1",
        op: "snapshot",
        token: &secret,
    })?;
    if request.len() + 1 > 512 {
        bail!("request exceeds transport limit");
    }
    request.push(b'\n');
    peer.write_all(&request)?;
    peer.flush()?;
    let mut response = Vec::new();
    peer.take((MAX_RESPONSE + 1) as u64)
        .read_to_end(&mut response)
        .context("sidecar response timed out or unreadable")?;
    if response.len() > MAX_RESPONSE || response.last() != Some(&b'\n') {
        bail!("sidecar response overflow or missing newline");
    }
    let data: Snapshot =
        serde_json::from_slice(&response).context("sidecar sent invalid payload")?;
    if data.protocol != "hars-sidecar-v1"
        || !data.read_only
        || data.generation == 0
        || data.entities.len() > MAX_STATES
    {
        bail!("sidecar protocol/authority contract violated");
    }
    let mut observed = std::collections::HashSet::new();
    for e in &data.entities {
        if !e.entity_id.contains('.')
            || e.entity_id.len() > 255
            || !observed.insert(e.entity_id.as_str())
            || e.state.len() > 4096
        {
            bail!("sidecar invalid/duplicate entity id or excessive state");
        }
    }
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader};
    use std::os::unix::net::UnixListener;
    use std::thread;

    fn setup() -> (
        tempfile::TempDir,
        std::path::PathBuf,
        std::path::PathBuf,
        u32,
    ) {
        let dir = tempfile::tempdir().unwrap();
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let socket = dir.path().join("ha-core.sock");
        let token = dir.path().join("capability");
        fs::write(
            &token,
            "SYNTHETIC_ONLY_READ_CAPABILITY_ABCDEFGHIJKLMNOPQRSTUVWXYZ",
        )
        .unwrap();
        fs::set_permissions(&token, fs::Permissions::from_mode(0o600)).unwrap();
        let uid = fs::metadata(&token).unwrap().uid();
        (dir, socket, token, uid)
    }

    #[test]
    fn authenticated_read_only_fixture() {
        let (_dir, socket, token, uid) = setup();
        let listener = UnixListener::bind(&socket).unwrap();
        fs::set_permissions(&socket, fs::Permissions::from_mode(0o600)).unwrap();
        let thread = thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let mut message = Vec::new();
            BufReader::new(&mut s)
                .read_until(b'\n', &mut message)
                .unwrap();
            assert!(message.len() <= 512 && message.last() == Some(&b'\n'));
            let req: serde_json::Value = serde_json::from_slice(&message).unwrap();
            assert_eq!(req["op"], "snapshot");
            assert_eq!(req["protocol"], "hars-sidecar-v1");
            assert_eq!(
                req["token"],
                "SYNTHETIC_ONLY_READ_CAPABILITY_ABCDEFGHIJKLMNOPQRSTUVWXYZ"
            );
            s.write_all(br#"{"protocol":"hars-sidecar-v1","read_only":true,"generation":1,"entities":[{"entity_id":"sensor.mock","state":"42"}]}"#).unwrap();
            s.write_all(b"\n").unwrap();
        });
        let result = fetch_snapshot(&socket, &token, uid).unwrap();
        assert_eq!(result.entities.len(), 1);
        assert_eq!(result.entities[0].state, "42");
        thread.join().unwrap();
    }
    #[test]
    fn fragmented_request_has_complete_newline_framing() {
        let (_dir, socket, token, uid) = setup();
        let listener = UnixListener::bind(&socket).unwrap();
        fs::set_permissions(&socket, fs::Permissions::from_mode(0o600)).unwrap();
        let thread = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut payload = Vec::new();
            loop {
                let mut byte = [0u8; 1];
                stream.read_exact(&mut byte).unwrap();
                payload.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
                assert!(payload.len() < 512);
            }
            let parsed: serde_json::Value = serde_json::from_slice(&payload).unwrap();
            assert_eq!(parsed["op"], "snapshot");
            let reply =
                br#"{"protocol":"hars-sidecar-v1","read_only":true,"generation":2,"entities":[]}
"#;
            stream.write_all(reply).unwrap();
        });
        let snapshot = fetch_snapshot(&socket, &token, uid).unwrap();
        assert_eq!(snapshot.generation, 2);
        assert!(snapshot.entities.is_empty());
        thread.join().unwrap();
    }

    #[test]
    fn capability_parent_must_not_be_group_accessible() {
        let (dir, _socket, token, uid) = setup();
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o750)).unwrap();
        assert!(check_private_regular(&token, uid).is_err());
    }

    #[test]
    fn capability_parent_alias_is_refused() {
        let (dir, _socket, _token, uid) = setup();
        let alias = dir.path().join("indirect");
        std::os::unix::fs::symlink(dir.path(), &alias).unwrap();
        assert!(check_private_regular(&alias.join("capability"), uid).is_err());
    }

    #[test]
    fn independent_private_token_directory_is_not_same_sidecar() {
        let (dir, socket, token, uid) = setup();
        let separate = dir.path().join("other-private");
        fs::create_dir(&separate).unwrap();
        fs::set_permissions(&separate, fs::Permissions::from_mode(0o700)).unwrap();
        let copied = separate.join("capability");
        fs::copy(&token, &copied).unwrap();
        fs::set_permissions(&copied, fs::Permissions::from_mode(0o600)).unwrap();
        let listener = UnixListener::bind(&socket).unwrap();
        fs::set_permissions(&socket, fs::Permissions::from_mode(0o600)).unwrap();
        assert!(fetch_snapshot(&socket, &copied, uid).is_err());
        drop(listener);
    }

    #[test]
    fn untrusted_socket_and_capability_refused() {
        let (dir, socket, token, uid) = setup();
        fs::write(&socket, "not a socket").unwrap();
        assert!(fetch_snapshot(&socket, &token, uid).is_err());
        fs::remove_file(&socket).unwrap();
        let listener = UnixListener::bind(&socket).unwrap();
        fs::set_permissions(&socket, fs::Permissions::from_mode(0o600)).unwrap();
        fs::set_permissions(&token, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(fetch_snapshot(&socket, &token, uid).is_err());
        fs::set_permissions(&token, fs::Permissions::from_mode(0o600)).unwrap();
        assert!(fetch_snapshot(&socket, &token, uid + 1).is_err());
        drop(listener);
        let link = dir.path().join("fake-sock");
        std::os::unix::fs::symlink(&socket, &link).unwrap();
        assert!(fetch_snapshot(&link, &token, uid).is_err());
    }
    #[test]
    fn response_with_mutation_claim_or_duplicates_is_refused() {
        for body in [
            r#"{"protocol":"hars-sidecar-v1","read_only":false,"generation":1,"entities":[]}"#,
            r#"{"protocol":"hars-sidecar-v1","read_only":true,"generation":0,"entities":[]}"#,
            r#"{"protocol":"hars-sidecar-v1","read_only":true,"generation":1,"entities":[{"entity_id":"sensor.x","state":"1"},{"entity_id":"sensor.x","state":"2"}]}"#,
        ] {
            let (_dir, socket, token, uid) = setup();
            let listener = UnixListener::bind(&socket).unwrap();
            fs::set_permissions(&socket, fs::Permissions::from_mode(0o600)).unwrap();
            let data = body.to_owned();
            let thread = thread::spawn(move || {
                let (mut s, _) = listener.accept().unwrap();
                let mut request = Vec::new();
                BufReader::new(&mut s)
                    .read_until(b'\n', &mut request)
                    .unwrap();
                assert!(request.len() <= 512);
                s.write_all(data.as_bytes()).unwrap();
                s.write_all(b"\n").unwrap();
            });
            assert!(fetch_snapshot(&socket, &token, uid).is_err());
            thread.join().unwrap();
        }
    }
}
