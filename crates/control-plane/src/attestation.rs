//! Short-lived Ed25519 authorization of local inventory snapshots.
//! Not an encrypted network transport and not a remote agent credential.
use ed25519_dalek::{Signature, VerifyingKey};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path};
use std::time::{SystemTime, UNIX_EPOCH};

const PREFIX: &str = "ha-rust-bridge-attestation/v1";
const MAX_FILE_BYTES: u64 = 5 * 1024 * 1024;

#[derive(Clone, Debug)]
pub struct Verified {
    pub issuer: &'static str,
    pub issued_at: u64,
    pub expires_at: u64,
}

pub fn safe_runtime_path(name: &str) -> bool {
    let mut components = Path::new(name).components();
    matches!(components.next(), Some(Component::Normal(part)) if part == "runtime")
        && matches!(components.next(), Some(Component::Normal(_)))
        && components.next().is_none()
}

fn safe_file_hash(path: &str) -> Result<String, &'static str> {
    if !safe_runtime_path(path) {
        return Err("untrusted snapshot path");
    }
    let meta = fs::symlink_metadata(path).map_err(|_| "snapshot missing")?;
    if !meta.file_type().is_file() || meta.len() > MAX_FILE_BYTES {
        return Err("snapshot must be a small regular file");
    }
    let bytes = fs::read(path).map_err(|_| "snapshot unreadable")?;
    Ok(hex::encode(Sha256::digest(bytes)))
}

fn signature_message(issued_at: u64, expires_at: u64, files: &[(String, String)]) -> String {
    let mut output = format!("{PREFIX}\nlabhost-pve\n{issued_at}\n{expires_at}\n");
    for (name, digest) in files {
        output.push_str(name);
        output.push(':');
        output.push_str(digest);
        output.push('\n');
    }
    output
}

fn check_signature(
    public_hex: &str,
    signature_hex: &str,
    message: &str,
) -> Result<(), &'static str> {
    let pub_bytes = hex::decode(public_hex.trim()).map_err(|_| "invalid public trust key")?;
    let public: [u8; 32] = pub_bytes
        .try_into()
        .map_err(|_| "invalid public trust key length")?;
    let verifying = VerifyingKey::from_bytes(&public).map_err(|_| "invalid public trust key")?;
    let bytes = hex::decode(signature_hex).map_err(|_| "invalid signature encoding")?;
    let signature = Signature::from_slice(&bytes).map_err(|_| "invalid signature size")?;
    verifying
        .verify_strict(message.as_bytes(), &signature)
        .map_err(|_| "snapshot signature verification failed")
}

pub fn verify(
    manifest: &str,
    attestation_path: &str,
    public_path: &str,
    source_paths: &[String],
) -> Result<Verified, &'static str> {
    let body = fs::read_to_string(attestation_path).map_err(|_| "attestation missing")?;
    let att: Value = serde_json::from_str(&body).map_err(|_| "invalid attestation format")?;
    if att["schema"] != 1 || att["issuer"] != "labhost-pve" {
        return Err("unsupported attestation schema or issuer");
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "system time unavailable")?
        .as_secs();
    let issued_at = att["issued_at"].as_u64().ok_or("invalid issuance time")?;
    let expires_at = att["expires_at"].as_u64().ok_or("invalid expiry")?;
    if issued_at > now.saturating_add(60)
        || expires_at <= now
        || expires_at <= issued_at
        || expires_at - issued_at > 3600
    {
        return Err("attestation expired, too old or future-dated");
    }

    let source_manifest: Value =
        serde_json::from_slice(&fs::read(manifest).map_err(|_| "source manifest missing")?)
            .map_err(|_| "invalid source manifest")?;
    let collected_at = source_manifest["collected_at"]
        .as_u64()
        .ok_or("missing manifest collection time")?;
    if collected_at > issued_at.saturating_add(60) || issued_at.saturating_sub(collected_at) > 120 {
        return Err("signed inventory was collected too long before signing");
    }
    let rows = att["files"].as_array().ok_or("missing signed file list")?;
    if rows.len() > 17 || rows.is_empty() {
        return Err("invalid signed file count");
    }
    let mut files: Vec<(String, String)> = Vec::with_capacity(rows.len());
    for row in rows {
        let name = row["path"].as_str().ok_or("invalid signed path")?;
        let declared_digest = row["sha256"].as_str().ok_or("invalid digest")?;
        if declared_digest.len() != 64 || !declared_digest.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("invalid signed hash");
        }
        let actual = safe_file_hash(name)?;
        if actual != declared_digest {
            return Err("snapshot content changed after signing");
        }
        files.push((name.to_string(), declared_digest.to_string()));
    }
    // Strict sorted ordering is part of the canonical signature representation.
    if !files.windows(2).all(|w| w[0].0 < w[1].0) {
        return Err("signed files must be unique and sorted");
    }
    let mut required = BTreeSet::new();
    required.insert(manifest.to_owned());
    for path in source_paths {
        required.insert(path.clone());
    }
    let signed: BTreeSet<_> = files.iter().map(|f| f.0.clone()).collect();
    if required != signed || !safe_runtime_path(manifest) {
        return Err("attestation does not match source manifest");
    }
    let public_hex = fs::read_to_string(public_path).map_err(|_| "trusted public key missing")?;
    let signature = att["signature"].as_str().ok_or("signature missing")?;
    let msg = signature_message(issued_at, expires_at, &files);
    check_signature(&public_hex, signature, &msg)?;
    Ok(Verified {
        issuer: "labhost-pve",
        issued_at,
        expires_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    #[test]
    fn known_signature_cannot_be_forged() {
        let private = SigningKey::from_bytes(&[7u8; 32]);
        let public_hex = hex::encode(private.verifying_key().to_bytes());
        let msg = signature_message(100, 120, &[("runtime/a.json".into(), "a".repeat(64))]);
        let signed = hex::encode(private.sign(msg.as_bytes()).to_bytes());
        assert!(check_signature(&public_hex, &signed, &msg).is_ok());
        assert!(check_signature(&public_hex, &signed, &(msg + "tamper")).is_err());
    }

    #[test]
    fn snapshot_path_restriction() {
        assert!(safe_runtime_path("runtime/sources.json"));
        assert!(!safe_runtime_path("runtime/../.ssh"));
        assert!(!safe_runtime_path("runtime/foo/../bar"));
        assert!(!safe_runtime_path("/etc/shadow"));
        assert!(!safe_runtime_path("references/other"));
    }

    #[test]
    fn signing_order_is_stable() {
        let a = signature_message(10, 20, &[("runtime/one".into(), "0".repeat(64))]);
        let b = signature_message(10, 20, &[("runtime/one".into(), "0".repeat(64))]);
        assert_eq!(a, b);
        assert!(a.starts_with("ha-rust-bridge-attestation/v1\nlabhost-pve\n"));
    }
}
