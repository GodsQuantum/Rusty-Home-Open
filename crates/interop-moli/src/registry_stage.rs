//! Offline HA entity-registry to Rust/Moli identity STAGING.
//! NO HAOS restore, Python integration execution, device writes, or deployment.
//! This is an opt-in private read-only identity shadow; missing live state is NULL.
//! Every activation is durable in redb and can be atomically rolled back.
use crate::{HaEntity, project};
use anyhow::{Context, Result, bail};
use redb::{Database, Durability, ReadableDatabase, ReadableTable, TableDefinition};
use serde::{
    Deserialize, Deserializer, Serialize,
    de::{self, MapAccess, SeqAccess, Visitor},
};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs::{self, File},
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::Path,
};
const BATCHES: TableDefinition<&str, &str> = TableDefinition::new("registry_stage_batches_v1");
const CONTROL: TableDefinition<&str, &str> = TableDefinition::new("registry_stage_control_v1");
const MAX_EXPORT: usize = 24 * 1024 * 1024;
const MAX_ENTITIES: usize = 50000;

/// Reject duplicate keys at *every* level, including inside per-entity data.
/// serde_json::Value alone silently lets last-wins keys forge unique_id/slug.
#[derive(Debug)]
pub(crate) struct StrictValue(pub(crate) Value);
impl<'de> Deserialize<'de> for StrictValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        struct StrictVisitor;
        impl<'de> Visitor<'de> for StrictVisitor {
            type Value = StrictValue;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("JSON without duplicate object keys")
            }
            fn visit_bool<E: de::Error>(self, v: bool) -> std::result::Result<Self::Value, E> {
                Ok(StrictValue(Value::Bool(v)))
            }
            fn visit_i64<E: de::Error>(self, v: i64) -> std::result::Result<Self::Value, E> {
                Ok(StrictValue(v.into()))
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> std::result::Result<Self::Value, E> {
                Ok(StrictValue(v.into()))
            }
            fn visit_f64<E: de::Error>(self, v: f64) -> std::result::Result<Self::Value, E> {
                serde_json::Number::from_f64(v)
                    .map(|n| StrictValue(Value::Number(n)))
                    .ok_or_else(|| E::custom("nonfinite number"))
            }
            fn visit_str<E: de::Error>(self, v: &str) -> std::result::Result<Self::Value, E> {
                Ok(StrictValue(Value::String(v.into())))
            }
            fn visit_string<E: de::Error>(self, v: String) -> std::result::Result<Self::Value, E> {
                Ok(StrictValue(Value::String(v)))
            }
            fn visit_none<E: de::Error>(self) -> std::result::Result<Self::Value, E> {
                Ok(StrictValue(Value::Null))
            }
            fn visit_unit<E: de::Error>(self) -> std::result::Result<Self::Value, E> {
                Ok(StrictValue(Value::Null))
            }
            fn visit_some<D: Deserializer<'de>>(
                self,
                d: D,
            ) -> std::result::Result<Self::Value, D::Error> {
                StrictValue::deserialize(d)
            }
            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut a: A,
            ) -> std::result::Result<Self::Value, A::Error> {
                let mut v = Vec::new();
                while let Some(item) = a.next_element::<StrictValue>()? {
                    v.push(item.0);
                    if v.len() > MAX_ENTITIES * 2 {
                        return Err(de::Error::custom("oversized JSON array"));
                    }
                }
                Ok(StrictValue(Value::Array(v)))
            }
            fn visit_map<A: MapAccess<'de>>(
                self,
                mut a: A,
            ) -> std::result::Result<Self::Value, A::Error> {
                let mut m = Map::new();
                while let Some((k, v)) = a.next_entry::<String, StrictValue>()? {
                    if m.contains_key(&k) {
                        return Err(de::Error::custom("duplicate JSON key"));
                    }
                    m.insert(k, v.0);
                }
                Ok(StrictValue(Value::Object(m)))
            }
        }
        deserializer.deserialize_any(StrictVisitor)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Identity {
    pub entity_id: String,
    pub platform: String,
    pub config_entry_id: Option<String>,
    pub unique_id: String,
    pub native_point: String,
    pub stable: bool,
    /// Historical HA entity registry never represents the current device state.
    pub state_known: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stage {
    pub version: u8,
    pub batch_sha256: String,
    pub entity_count: usize,
    pub stable_count: usize,
    pub entities: Vec<Identity>,
    pub read_only: bool,
    pub no_device_commands: bool,
    pub no_live_states: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadOnlyState {
    pub entity_id: String,
    pub state: String,
}
/// Strict official Core State.as_dict JSON snapshot, not a subscription.
pub fn parse_core_state_snapshot(raw: &[u8]) -> Result<Vec<ReadOnlyState>> {
    if raw.is_empty() || raw.len() > MAX_EXPORT {
        bail!("missing or oversized Core state snapshot")
    }
    let StrictValue(value): StrictValue =
        serde_json::from_slice(raw).context("invalid or duplicate state snapshot keys")?;
    let list = value
        .as_array()
        .context("Core state snapshot must be array")?;
    if list.len() > MAX_ENTITIES {
        bail!("too many Core states")
    }
    let mut seen = BTreeSet::new();
    let mut states = Vec::with_capacity(list.len());
    for item in list {
        let id = read_string(item, "entity_id")?;
        let state = read_string(item, "state")?;
        let timestamp = read_string(item, "last_updated")?;
        if id.len() > 255
            || id
                .bytes()
                .any(|b| b.is_ascii_control() || b == b'/' || b == b'\\')
            || state.len() > 4096
            || state.chars().any(char::is_control)
            || timestamp.len() > 64
            || timestamp.chars().any(char::is_control)
            || !(timestamp.ends_with("+00:00") || timestamp.ends_with('Z'))
            || !seen.insert(id.to_owned())
        {
            bail!("unsafe Core state payload")
        }
        states.push(ReadOnlyState {
            entity_id: id.to_owned(),
            state: state.to_owned(),
        });
    }
    states.sort_by(|a, b| a.entity_id.cmp(&b.entity_id));
    Ok(states)
}

fn is_digest(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
/// Revalidate persisted stage before it can become active in an actual Moli Hub.
/// A private redb record is not assumed trustworthy after an interrupted write,
/// manual file modification or future importer schema change.
fn verify_stage(s: &Stage) -> Result<()> {
    if s.version != 1
        || !is_digest(&s.batch_sha256)
        || !s.read_only
        || !s.no_device_commands
        || !s.no_live_states
        || s.entity_count != s.entities.len()
        || s.entity_count > MAX_ENTITIES
        || s.stable_count != s.entities.iter().filter(|e| e.stable).count()
    {
        bail!("untrusted persisted registry batch")
    }
    let serialized = serde_json::to_vec(&s.entities)?;
    if hex::encode(Sha256::digest(&serialized)) != s.batch_sha256 {
        bail!("registry batch checksum mismatch")
    }
    let mut previous: Option<&str> = None;
    let mut native = BTreeSet::new();
    for e in &s.entities {
        if e.entity_id.len() > 255
            || e.entity_id.is_empty()
            || e.entity_id
                .bytes()
                .any(|b| b.is_ascii_control() || b == b'/' || b == b'\\')
            || e.platform.len() > 128
            || e.platform.is_empty()
            || e.unique_id.len() > 4096
            || e.unique_id.is_empty()
            || e.state_known
            || previous.is_some_and(|p| p >= e.entity_id.as_str())
            || !native.insert(e.native_point.as_str())
        {
            bail!("invalid or duplicate persisted registry identity")
        }
        previous = Some(&e.entity_id);
        let mapped = project(&HaEntity {
            entity_id: e.entity_id.clone(),
            platform: e.platform.clone(),
            config_entry_id: e.config_entry_id.clone(),
            unique_id: Some(e.unique_id.clone()),
            state: Value::Null,
            attributes: Map::new(),
        })
        .map_err(|_| anyhow::anyhow!("cannot verify persisted identity"))?;
        if mapped.point_id.as_str() != e.native_point || mapped.stable_identity != e.stable {
            bail!("native identity differs from its original Core mapping")
        }
    }
    Ok(())
}
fn read_string<'a>(v: &'a Value, k: &str) -> Result<&'a str> {
    v.get(k)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow::anyhow!("missing registry identity field {k}"))
}
pub fn parse_export(raw: &[u8]) -> Result<Stage> {
    if raw.is_empty() || raw.len() > MAX_EXPORT {
        bail!("registry export must be nonempty, under 24 MiB")
    }
    let StrictValue(v): StrictValue =
        serde_json::from_slice(raw).context("invalid/duplicate JSON")?;
    if read_string(&v, "key")? != "core.entity_registry" {
        bail!("wrong HA storage registry kind")
    }
    let ver = v.get("version").and_then(Value::as_u64).unwrap_or(0);
    if !(1..=10).contains(&ver) {
        bail!("unsupported HA storage schema version")
    }
    let list = v
        .pointer("/data/entities")
        .and_then(Value::as_array)
        .context("missing entity registry entries")?;
    if list.len() > MAX_ENTITIES {
        bail!("too many entities")
    }
    let mut identities = Vec::with_capacity(list.len());
    let mut ids = BTreeSet::new();
    let mut native = BTreeSet::new();
    for item in list {
        let id = read_string(item, "entity_id")?;
        if id.len() > 255
            || id
                .bytes()
                .any(|b| b.is_ascii_control() || b == b'/' || b == b'\\')
        {
            bail!("invalid entity id")
        }
        let platform = read_string(item, "platform")?;
        if platform.len() > 128 || platform.chars().any(char::is_control) {
            bail!("invalid platform")
        }
        let unique = read_string(item, "unique_id")?;
        if unique.len() > 4096 || unique.chars().any(char::is_control) {
            bail!("invalid unique id")
        }
        let config = item.get("config_entry_id").map_or(Ok(None), |val| {
            if val.is_null() {
                Ok(None)
            } else {
                val.as_str()
                    .filter(|s| !s.is_empty() && s.len() <= 256 && !s.chars().any(char::is_control))
                    .map(|s| Some(s.to_owned()))
                    .ok_or_else(|| anyhow::anyhow!("invalid config entry id"))
            }
        })?;
        if !ids.insert(id.to_owned()) {
            bail!("duplicate entity id")
        }
        let mapped = project(&HaEntity {
            entity_id: id.to_owned(),
            platform: platform.to_owned(),
            config_entry_id: config.clone(),
            unique_id: Some(unique.to_owned()),
            state: Value::Null,
            attributes: Map::new(),
        })
        .map_err(|_| anyhow::anyhow!("unprojectable HA entity"))?;
        let native_id = mapped.point_id.as_str().to_owned();
        if !native.insert(native_id.clone()) {
            bail!("two HA entities collide on stable native identity")
        }
        identities.push(Identity {
            entity_id: id.into(),
            platform: platform.into(),
            config_entry_id: config,
            unique_id: unique.into(),
            native_point: native_id,
            stable: mapped.stable_identity,
            state_known: false,
        });
    }
    // Stage fingerprint is order-independent: sorting by entity_id guarantees
    // equivalent registry exports do not produce duplicate migration batches.
    identities.sort_by(|a, b| a.entity_id.cmp(&b.entity_id));
    let body = serde_json::to_vec(&identities)?;
    let digest = hex::encode(Sha256::digest(&body));
    let stable_count = identities.iter().filter(|e| e.stable).count();
    Ok(Stage {
        version: 1,
        batch_sha256: digest,
        entity_count: identities.len(),
        stable_count,
        entities: identities,
        read_only: true,
        no_device_commands: true,
        no_live_states: true,
    })
}

pub struct RegistryStager {
    db: Database,
}
impl RegistryStager {
    /// Caller supplies an existing owner-private 0700 lab directory.
    pub fn open(dir: &Path) -> Result<Self> {
        let m = fs::symlink_metadata(dir)?;
        if !m.is_dir()
            || m.file_type().is_symlink()
            || m.uid() != nix::unistd::Uid::current().as_raw()
            || m.permissions().mode() & 0o077 != 0
        {
            bail!("registry store requires owner-private 0700 real directory")
        }
        let path = dir.join("registry-shadow.redb");
        if let Ok(m) = fs::symlink_metadata(&path)
            && (!m.is_file()
                || m.file_type().is_symlink()
                || m.uid() != nix::unistd::Uid::current().as_raw()
                || m.permissions().mode() & 0o077 != 0)
        {
            bail!("unsafe existing registry shadow")
        }
        let db = Database::create(&path)?;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
        let mut tx = db.begin_write()?;
        tx.set_durability(Durability::Immediate)?;
        tx.set_two_phase_commit(true);
        {
            let _ = tx.open_table(BATCHES)?;
            let _ = tx.open_table(CONTROL)?;
        }
        tx.commit()?;
        File::open(dir)?.sync_all()?;
        Ok(Self { db })
    }
    pub fn stage(&self, raw: &[u8]) -> Result<Stage> {
        let proposal = parse_export(raw)?;
        let text = serde_json::to_string(&proposal)?;
        let mut tx = self.db.begin_write()?;
        tx.set_durability(Durability::Immediate)?;
        tx.set_two_phase_commit(true);
        {
            let mut table = tx.open_table(BATCHES)?;
            if let Some(old) = table.get(proposal.batch_sha256.as_str())? {
                if old.value() != text {
                    bail!("digest conflicts with an earlier batch")
                }
                return Ok(proposal);
            }
            table.insert(proposal.batch_sha256.as_str(), text.as_str())?;
        }
        tx.commit()?;
        Ok(proposal)
    }
    /// Read-only SHADOW activation. There are NO driver hooks or service actions.
    /// CAS on the previous batch prevents racing independent operators.
    pub fn activate(&self, batch: &str, expected_previous: Option<&str>) -> Result<()> {
        if !is_digest(batch) || expected_previous.is_some_and(|id| !is_digest(id)) {
            bail!("invalid digest")
        }
        let mut tx = self.db.begin_write()?;
        tx.set_durability(Durability::Immediate)?;
        tx.set_two_phase_commit(true);
        {
            let table = tx.open_table(BATCHES)?;
            let Some(saved) = table.get(batch)? else {
                bail!("batch has not been staged")
            };
            let s: Stage = serde_json::from_str(saved.value())?;
            if s.batch_sha256 != batch {
                bail!("wrong staged batch")
            }
            verify_stage(&s)?;
            drop(saved);
            drop(table);
            let mut control = tx.open_table(CONTROL)?;
            let prior = control.get("active")?.map(|v| v.value().to_owned());
            if prior.as_deref() != expected_previous {
                bail!("stale active batch CAS")
            }
            if prior.as_deref() == Some(batch) {
                return Ok(());
            }
            if let Some(old) = prior.as_deref() {
                control.insert("previous", old)?;
            } else {
                control.remove("previous")?;
            }
            control.insert("active", batch)?;
        }
        tx.commit()?;
        Ok(())
    }
    /// Compare-and-swap rollback: never modifies imported HA data or radios.
    pub fn rollback(&self, expected_active: &str) -> Result<Option<String>> {
        if !is_digest(expected_active) {
            bail!("invalid digest")
        }
        let mut tx = self.db.begin_write()?;
        tx.set_durability(Durability::Immediate)?;
        tx.set_two_phase_commit(true);
        let previous;
        {
            let mut control = tx.open_table(CONTROL)?;
            let active = control.get("active")?.map(|v| v.value().to_owned());
            if active.as_deref() != Some(expected_active) {
                bail!("stale rollback target")
            }
            previous = control.get("previous")?.map(|v| v.value().to_owned());
            if let Some(old) = previous.as_deref() {
                control.insert("active", old)?;
            } else {
                control.remove("active")?;
            }
            control.remove("previous")?;
        }
        tx.commit()?;
        Ok(previous)
    }
    pub fn active(&self) -> Result<Option<Stage>> {
        let read = self.db.begin_read()?;
        let control = read.open_table(CONTROL)?;
        let active = control.get("active")?.map(|v| v.value().to_owned());
        drop(control);
        let Some(digest) = active else {
            return Ok(None);
        };
        if !is_digest(&digest) {
            bail!("corrupt active pointer")
        }
        let table = read.open_table(BATCHES)?;
        let stage = table
            .get(digest.as_str())?
            .ok_or_else(|| anyhow::anyhow!("orphan active pointer"))?;
        let s: Stage = serde_json::from_str(stage.value())?;
        if s.batch_sha256 != digest {
            bail!("active pointer mismatch")
        }
        verify_stage(&s)?;
        Ok(Some(s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn registry(entries: Vec<Value>) -> Vec<u8> {
        serde_json::to_vec(
            &json!({"key":"core.entity_registry","version":1,"minor_version":14,
            "data":{"entities":entries}}),
        )
        .unwrap()
    }
    fn entity(id: &str, unique: &str) -> Value {
        json!({"entity_id":id,"platform":"mqtt","unique_id":unique,
            "config_entry_id":"synthetic-entry","disabled_by":null,
            "device_id":"sample","name":null})
    }
    #[test]
    fn identity_stage_native_projection_matches_existing_moli_adapter() {
        let raw = registry(vec![
            entity("sensor.a", "123/a"),
            entity("switch.b", "123_b"),
        ]);
        let staged = parse_export(&raw).unwrap();
        assert_eq!(staged.entity_count, 2);
        assert_eq!(staged.stable_count, 2);
        for row in &staged.entities {
            assert!(!row.state_known);
            assert!(row.native_point.starts_with("ha-compat:"));
        }
        let mut reversed = vec![entity("switch.b", "123_b"), entity("sensor.a", "123/a")];
        assert_eq!(
            staged.batch_sha256,
            parse_export(&registry(reversed.clone()))
                .unwrap()
                .batch_sha256
        );
        reversed.push(entity("sensor.c", "new"));
        assert_ne!(
            staged.batch_sha256,
            parse_export(&registry(reversed)).unwrap().batch_sha256
        );
    }
    #[test]
    fn genuinely_durable_activation_rollback_and_cas() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path().join("private");
        fs::create_dir(&d).unwrap();
        fs::set_permissions(&d, fs::Permissions::from_mode(0o700)).unwrap();
        let batch_a = registry(vec![entity("sensor.a", "id1")]);
        let batch_b = registry(vec![entity("sensor.a", "id1"), entity("sensor.b", "id2")]);
        let first;
        let second;
        {
            let db = RegistryStager::open(&d).unwrap();
            first = db.stage(&batch_a).unwrap();
            second = db.stage(&batch_b).unwrap();
            assert!(db.active().unwrap().is_none());
            db.activate(&first.batch_sha256, None).unwrap();
            assert!(db.activate(&second.batch_sha256, None).is_err());
            db.activate(&second.batch_sha256, Some(&first.batch_sha256))
                .unwrap();
            assert!(db.rollback(&first.batch_sha256).is_err());
        }
        {
            let db = RegistryStager::open(&d).unwrap();
            assert_eq!(db.active().unwrap().unwrap(), second);
            assert_eq!(
                db.rollback(&second.batch_sha256).unwrap(),
                Some(first.batch_sha256.clone())
            );
            assert_eq!(db.active().unwrap().unwrap(), first);
            assert!(db.rollback(&second.batch_sha256).is_err());
        }
        assert_eq!(
            fs::metadata(d.join("registry-shadow.redb"))
                .unwrap()
                .permissions()
                .mode()
                & 0o077,
            0
        );
    }
    #[test]
    fn empty_and_missing_unique_do_not_invent_data() {
        let missing = registry(vec![
            json!({"entity_id":"sensor.unknown","platform":"template",
            "config_entry_id":null,"unique_id":"standalone"}),
        ]);
        let s = parse_export(&missing).unwrap();
        assert_eq!(s.stable_count, 0);
        assert!(!s.entities[0].state_known);
        assert!(
            parse_export(&registry(vec![
                json!({"entity_id":"sensor.none","platform":"mqtt"})
            ]))
            .is_err()
        );
        assert_eq!(parse_export(&registry(vec![])).unwrap().entity_count, 0);
    }
    #[test]
    fn reject_duplicate_keys_collisions_untrusted_version_and_unsafe_names() {
        let duplicate=br#"{"key":"core.entity_registry","version":1,"data":{"entities":[{"entity_id":"sensor.x","entity_id":"sensor.y","platform":"mqtt","unique_id":"a"}]}}"#;
        assert!(parse_export(duplicate).is_err());
        let duplicate_top=br#"{"key":"core.entity_registry","key":"core.entity_registry","version":1,"data":{"entities":[]}}"#;
        assert!(parse_export(duplicate_top).is_err());
        assert!(
            parse_export(&registry(vec![
                entity("sensor.a", "x"),
                entity("sensor.a", "x")
            ]))
            .is_err()
        );
        assert!(
            parse_export(&registry(vec![
                entity("sensor.a", "x"),
                entity("sensor.b", "x")
            ]))
            .is_err()
        );
        assert!(parse_export(&registry(vec![entity("../etc/passwd", "x")])).is_err());
        let other = serde_json::to_vec(&json!({"key":"core.config_entries","version":1,
            "data":{"entities":[]}}))
        .unwrap();
        assert!(parse_export(&other).is_err());
        assert!(parse_export(&vec![b' '; MAX_EXPORT + 1]).is_err());
    }
    #[test]
    fn authentic_state_shape_and_adversarial_values() {
        let good = serde_json::json!([
            {"entity_id":"sensor.value","state":"21.5",
             "last_updated":"2026-10-24T23:59:00+00:00","attributes":{"unit":"°C"}},
            {"entity_id":"switch.value","state":"on",
             "last_updated":"2026-10-24T23:59:00+00:00"}
        ]);
        let rows = parse_core_state_snapshot(&serde_json::to_vec(&good).unwrap()).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].state, "21.5");
        assert_eq!(rows[1].state, "on");
        assert!(parse_core_state_snapshot(br#"[{"entity_id":"sensor.a","entity_id":"sensor.b","state":"on","last_updated":"2026-10-24T23:59:00+00:00"}]"#).is_err());
        assert!(parse_core_state_snapshot(br#"[{"entity_id":"sensor.a","state":"on","last_updated":"2026-10-24T23:59:00+00:00"},{"entity_id":"sensor.a","state":"off","last_updated":"2026-10-24T23:59:00+00:00"}]"#).is_err());
        assert!(parse_core_state_snapshot(br#"[{"entity_id":"sensor.a","state":"on"}]"#).is_err());
        assert!(parse_core_state_snapshot(br#"[{"entity_id":"sensor.a","state":42,"last_updated":"2026-10-24T23:59:00+00:00"}]"#).is_err());
    }

    #[test]
    fn tampered_private_database_cannot_publish_or_activate_wrong_mapping() {
        let dir = tempfile::tempdir().unwrap();
        let private = dir.path().join("private");
        fs::create_dir(&private).unwrap();
        fs::set_permissions(&private, fs::Permissions::from_mode(0o700)).unwrap();
        let raw = registry(vec![entity("sensor.original", "serial/1")]);
        let db = RegistryStager::open(&private).unwrap();
        let original = db.stage(&raw).unwrap();
        db.activate(&original.batch_sha256, None).unwrap();
        drop(db);
        let path = private.join("registry-shadow.redb");
        let disk = Database::create(&path).unwrap();
        let mut tx = disk.begin_write().unwrap();
        tx.set_two_phase_commit(true);
        {
            let mut stages = tx.open_table(BATCHES).unwrap();
            let mut corrupt = original.clone();
            corrupt.entities[0].native_point = "different:forged/state".into();
            let json = serde_json::to_string(&corrupt).unwrap();
            stages
                .insert(original.batch_sha256.as_str(), json.as_str())
                .unwrap();
        }
        tx.commit().unwrap();
        drop(disk);
        let db = RegistryStager::open(&private).unwrap();
        assert!(
            db.active().is_err(),
            "checksum mismatch could publish a forged device"
        );
        assert!(
            db.activate(&original.batch_sha256, Some(&original.batch_sha256))
                .is_err()
        );
    }

    #[test]
    fn deny_unsafe_state_directory_and_links() {
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("real");
        fs::create_dir(&real).unwrap();
        assert!(RegistryStager::open(&real).is_err());
        fs::set_permissions(&real, fs::Permissions::from_mode(0o700)).unwrap();
        let link = dir.path().join("symlink");
        std::os::unix::fs::symlink(&real, &link).unwrap();
        assert!(RegistryStager::open(&link).is_err());
        assert!(RegistryStager::open(&real).is_ok());
    }
}
