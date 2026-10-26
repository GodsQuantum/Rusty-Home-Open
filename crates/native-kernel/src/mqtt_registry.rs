//! Transactional, fail-closed MQTT discovery cache (P2g laboratory only).
//! Recovery always yields STALE_UNVERIFIED: a cache is not broker authority.
use crate::mqtt_discovery::{self, Intent};
use anyhow::{Context, Result, bail};
use redb::{Database, Durability, ReadableDatabase, ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::Path;

const CONFIGS: TableDefinition<&str, &str> = TableDefinition::new("lab_mqtt_discovery_v1");

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoredSensor {
    pub unique_id: String,
    pub platform: String,
    pub state_topic: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoredConfig {
    pub version: u8,
    pub id: String,
    pub topic: String,
    pub sensors: Vec<StoredSensor>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recovered {
    pub config: StoredConfig,
    /// On process restart a persisted config can NEVER authorize device use.
    pub stale_unverified: bool,
}

pub struct DiscoveryRegistry {
    db: Database,
}

fn private_dir(root: &Path) -> Result<()> {
    let m = fs::symlink_metadata(root)?;
    if !m.is_dir()
        || m.file_type().is_symlink()
        || m.uid() != nix::unistd::Uid::current().as_raw()
        || m.permissions().mode() & 0o077 != 0
    {
        bail!("MQTT discovery registry requires private owner-matched directory mode 0700");
    }
    Ok(())
}

fn safe_file(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(m) => {
            if !m.is_file()
                || m.file_type().is_symlink()
                || m.uid() != nix::unistd::Uid::current().as_raw()
                || m.permissions().mode() & 0o077 != 0
            {
                bail!("MQTT discovery registry refuses unsafe existing file");
            }
            Ok(true)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e.into()),
    }
}

fn valid_identity(raw: &str) -> bool {
    !raw.is_empty()
        && raw.len() <= 100
        && raw
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-'))
}

fn valid_topic(raw: &str) -> bool {
    raw.starts_with("lab/readonly/")
        && raw.len() <= 128
        && raw
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'/' | b'_' | b'-'))
}

fn decode(raw: &str) -> Result<StoredConfig> {
    let config: StoredConfig = serde_json::from_str(raw)?;
    if config.version != 1
        || config.sensors.is_empty()
        || config.sensors.len() > 16
        || config.id.len() > 240
        || !matches!(mqtt_discovery::preview(&config.topic, b"", true), Ok(Intent::Remove { id }) if id == config.id)
        || config.sensors.iter().any(|s| {
            !valid_identity(&s.unique_id)
                || !valid_topic(&s.state_topic)
                || !matches!(s.platform.as_str(), "sensor" | "binary_sensor")
        })
    {
        bail!("invalid persisted MQTT discovery entry");
    }
    let mut unique = HashSet::new();
    let mut topics = HashSet::new();
    if config.sensors.iter().any(|s| !unique.insert(&s.unique_id)) {
        bail!("duplicate persisted MQTT sensor ID");
    }
    // A device payload may contain distinct unique_ids yet reuse one state
    // topic. The native read-only Moli adapter cannot safely attribute it.
    if config
        .sensors
        .iter()
        .any(|s| !topics.insert(&s.state_topic))
    {
        bail!("duplicate persisted MQTT state topic in one config");
    }
    Ok(config)
}

impl DiscoveryRegistry {
    pub fn open(root: &Path) -> Result<Self> {
        private_dir(root)?;
        let path = root.join("mqtt-discovery.redb");
        if !safe_file(&path)? {
            // Atomic create with mode 0600 from the very first filesystem
            // operation; never expose the database under a permissive umask.
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&path)
                .context("cannot atomically create private registry")?;
        }
        let db = Database::create(&path).context("cannot open redb MQTT registry")?;
        let mut tx = db.begin_write()?;
        tx.set_durability(Durability::Immediate)?;
        tx.set_two_phase_commit(true);
        {
            let _ = tx.open_table(CONFIGS)?;
        }
        tx.commit()?;
        fs::File::open(root)?.sync_all()?;
        Ok(Self { db })
    }

    /// Only a validated broker-retained configuration is durably staged.
    /// This does NOT register Moli devices or grant runtime write authority.
    pub fn retain_validated(&self, topic: &str, payload: &[u8]) -> Result<()> {
        self.apply_discovery(topic, payload, true)
    }

    pub fn apply_discovery(&self, topic: &str, payload: &[u8], retained: bool) -> Result<()> {
        if !retained || payload.is_empty() {
            bail!("MQTT discovery registry only accepts nonempty retained configurations");
        }
        let Intent::Preview { id, sensors, .. } = mqtt_discovery::preview(topic, payload, retained)
            .map_err(|err| anyhow::anyhow!("MQTT discovery validation rejected: {err:?}"))?
        else {
            bail!("MQTT discovery cannot persist commands, migrations or deletions");
        };
        let config = StoredConfig {
            version: 1,
            id,
            topic: topic.to_owned(),
            sensors: sensors
                .into_iter()
                .map(|s| StoredSensor {
                    unique_id: s.unique_id,
                    platform: s.platform,
                    state_topic: s.state_topic,
                })
                .collect(),
        };
        let _ = decode(&serde_json::to_string(&config)?)?;
        let mut tx = self.db.begin_write()?;
        tx.set_durability(Durability::Immediate)?;
        tx.set_two_phase_commit(true);
        {
            let mut table = tx.open_table(CONFIGS)?;
            for entry in table.iter()? {
                let (key, value) = entry?;
                let existing = decode(value.value())?;
                if key.value() == config.id {
                    if existing != config {
                        bail!("MQTT discovery config changed without reconciliation");
                    }
                    continue;
                }
                for previous in &existing.sensors {
                    if config.sensors.iter().any(|new| {
                        new.unique_id == previous.unique_id
                            || new.state_topic == previous.state_topic
                    }) {
                        bail!(
                            "MQTT discovery identity or state-topic collision across retained configs"
                        );
                    }
                }
            }
            let raw = serde_json::to_string(&config)?;
            table.insert(config.id.as_str(), raw.as_str())?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Delete only on a broker RETAIN=true empty publication with the exact
    /// original configuration topic. In particular, an unrelated component
    /// with the same parsed ID cannot erase a sensor. Never grant authority.
    pub fn remove_retained(&self, topic: &str, retained: bool) -> Result<bool> {
        if !retained {
            bail!("a non-retained MQTT empty payload cannot remove a registry entry");
        }
        let Intent::Remove { id } = mqtt_discovery::preview(topic, b"", true)
            .map_err(|e| anyhow::anyhow!("invalid retained tombstone: {e:?}"))?
        else {
            bail!("expected retained tombstone");
        };
        let mut tx = self.db.begin_write()?;
        tx.set_durability(Durability::Immediate)?;
        tx.set_two_phase_commit(true);
        let did_remove = {
            let mut table = tx.open_table(CONFIGS)?;
            let exists = {
                let result = table.get(id.as_str())?;
                if let Some(entry) = result {
                    let existing = decode(entry.value())?;
                    if existing.topic != topic {
                        bail!("tombstone topic does not match originally registered configuration");
                    }
                    true
                } else {
                    false
                }
            };
            if exists {
                table.remove(id.as_str())?;
            }
            exists
        };
        tx.commit()?;
        Ok(did_remove)
    }

    /// A durable record can be inspected after restart, but remains stale
    /// until an independent, verified live broker reconciliation mechanism.
    pub fn recover_unverified(&self) -> Result<Vec<Recovered>> {
        let tx = self.db.begin_read()?;
        let table = tx.open_table(CONFIGS)?;
        let mut records = Vec::new();
        // Older registry versions may contain rows that individually decode
        // but collectively alias a sensor state topic. Refuse the WHOLE
        // recovered cache rather than selecting an arbitrary owner.
        let mut recovered_topics = HashSet::new();
        let mut recovered_ids = HashSet::new();
        for entry in table.iter()? {
            let (key, value) = entry?;
            let record = decode(value.value())?;
            if key.value() != record.id {
                bail!("MQTT registry key differs from stored identity");
            }
            for sensor in &record.sensors {
                if !recovered_topics.insert(sensor.state_topic.clone()) {
                    bail!("ambiguous MQTT state topic in legacy persisted cache");
                }
                if !recovered_ids.insert(sensor.unique_id.clone()) {
                    bail!("ambiguous MQTT sensor identity in legacy persisted cache");
                }
            }
            records.push(Recovered {
                config: record,
                stale_unverified: true,
            });
            if records.len() > 64 {
                bail!("MQTT discovery cache size exceeds safety bound");
            }
        }
        Ok(records)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    const TOPIC: &str = "homeassistant/sensor/bedroom/temp/config";
    const INPUT: &[u8] = br#"{"unique_id":"lab_bedroom","state_topic":"lab/readonly/bedroom"}"#;

    fn private_root() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        fs::set_permissions(tmp.path(), fs::Permissions::from_mode(0o700)).unwrap();
        tmp
    }

    #[test]
    fn retained_config_survives_restart_but_never_claims_authority() {
        let root = private_root();
        {
            let store = DiscoveryRegistry::open(root.path()).unwrap();
            store.retain_validated(TOPIC, INPUT).unwrap();
        }
        let reopened = DiscoveryRegistry::open(root.path()).unwrap();
        let found = reopened.recover_unverified().unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].config.id, "bedroom/temp");
        assert_eq!(found[0].config.sensors[0].unique_id, "lab_bedroom");
        assert!(found[0].stale_unverified);
        assert_eq!(
            fs::metadata(root.path().join("mqtt-discovery.redb"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }

    #[test]
    fn nonretained_and_identity_collisions_fail_closed() {
        let root = private_root();
        let store = DiscoveryRegistry::open(root.path()).unwrap();
        store.retain_validated(TOPIC, INPUT).unwrap();
        assert!(store.apply_discovery(TOPIC, INPUT, false).is_err());
        assert!(
            store
                .retain_validated("homeassistant/sensor/study/temp/config", INPUT)
                .is_err()
        );
        assert_eq!(store.recover_unverified().unwrap().len(), 1);
        assert!(store.retain_validated(TOPIC, b"").is_err());
        assert!(
            store
                .retain_validated("homeassistant/light/lamp/config", INPUT)
                .is_err()
        );
    }

    #[test]
    fn tombstone_requires_exact_original_topic_and_retain_bit() {
        let root = private_root();
        let store = DiscoveryRegistry::open(root.path()).unwrap();
        store.retain_validated(TOPIC, INPUT).unwrap();
        assert!(store.remove_retained(TOPIC, false).is_err());
        assert!(
            store
                .remove_retained("homeassistant/binary_sensor/bedroom/temp/config", true)
                .is_err()
        );
        assert_eq!(store.recover_unverified().unwrap().len(), 1);
        assert!(store.remove_retained(TOPIC, true).unwrap());
        assert!(store.recover_unverified().unwrap().is_empty());
        assert!(!store.remove_retained(TOPIC, true).unwrap());
    }

    #[test]
    fn p2n_durable_topic_collision_is_atomic_across_batches_and_process_reopen() {
        let root = private_root();
        let rival_topic = "homeassistant/sensor/study/temperature/config";
        let rival = br#"{"unique_id":"lab_study","state_topic":"lab/readonly/bedroom"}"#;
        let batch = br#"{"dev":{"ids":"lab_device","name":"Lab device"},"o":{"name":"Lab origin"},"cmps":{"one":{"p":"sensor","uniq_id":"lab_one","stat_t":"lab/readonly/duplicate"},"two":{"p":"sensor","uniq_id":"lab_two","stat_t":"lab/readonly/duplicate"}}}"#;
        let mixed = br#"{"dev":{"ids":"lab_mixed","name":"Lab mixed"},"o":{"name":"Lab origin"},"cmps":{"one":{"p":"sensor","uniq_id":"lab_new","stat_t":"lab/readonly/fresh"},"two":{"p":"sensor","uniq_id":"lab_collision","stat_t":"lab/readonly/bedroom"}}}"#;
        {
            let store = DiscoveryRegistry::open(root.path()).unwrap();
            store.retain_validated(TOPIC, INPUT).unwrap();
            assert!(
                store.retain_validated(rival_topic, rival).is_err(),
                "a second unique_id must not durably claim the same state_topic"
            );
            assert!(
                store
                    .retain_validated("homeassistant/device/multi/config", batch)
                    .is_err(),
                "an internally ambiguous device batch must be rejected atomically"
            );
            assert!(
                store
                    .retain_validated("homeassistant/device/mixed/config", mixed)
                    .is_err(),
                "a partially valid batch must not register its uncollided sensor"
            );
            let records = store.recover_unverified().unwrap();
            assert_eq!(records.len(), 1);
            assert_eq!(records[0].config.id, "bedroom/temp");
        }
        {
            let store = DiscoveryRegistry::open(root.path()).unwrap();
            let records = store.recover_unverified().unwrap();
            assert_eq!(records.len(), 1);
            assert!(records[0].stale_unverified);
            assert_eq!(records[0].config.sensors[0].unique_id, "lab_bedroom");
            assert!(store.remove_retained(TOPIC, true).unwrap());
            store.retain_validated(rival_topic, rival).unwrap();
        }
        let reopened = DiscoveryRegistry::open(root.path()).unwrap();
        let records = reopened.recover_unverified().unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].config.sensors[0].unique_id, "lab_study");
        assert!(records[0].stale_unverified);
        assert_eq!(
            records[0].config.sensors[0].state_topic,
            "lab/readonly/bedroom"
        );
    }

    #[test]
    fn p2n_legacy_ambiguous_redb_cache_fails_closed_without_purging_owner() {
        let root = private_root();
        let store = DiscoveryRegistry::open(root.path()).unwrap();
        store.retain_validated(TOPIC, INPUT).unwrap();
        // Simulate an old on-disk database produced before the ownership fix.
        // The malicious row bypasses the public registry writer deliberately.
        let old = StoredConfig {
            version: 1,
            id: "study/temp".into(),
            topic: "homeassistant/sensor/study/temp/config".into(),
            sensors: vec![StoredSensor {
                unique_id: "lab_study".into(),
                platform: "sensor".into(),
                state_topic: "lab/readonly/bedroom".into(),
            }],
        };
        {
            let mut tx = store.db.begin_write().unwrap();
            tx.set_durability(Durability::Immediate).unwrap();
            tx.set_two_phase_commit(true);
            {
                let mut table = tx.open_table(CONFIGS).unwrap();
                table
                    .insert(
                        old.id.as_str(),
                        serde_json::to_string(&old).unwrap().as_str(),
                    )
                    .unwrap();
            }
            tx.commit().unwrap();
        }
        drop(store);
        let reopened = DiscoveryRegistry::open(root.path()).unwrap();
        assert!(
            reopened.recover_unverified().is_err(),
            "pre-fix ambiguous persisted state topics must never be trusted on recovery"
        );
        // The legitimate original row remains available for explicit operator
        // reconciliation; no destructive automatic repair is permitted.
        assert!(reopened.remove_retained(&old.topic, true).unwrap());
        let recovered = reopened.recover_unverified().unwrap();
        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered[0].config.id, "bedroom/temp");
        assert!(recovered[0].stale_unverified);
    }

    #[test]
    fn p2n_legacy_duplicate_identity_is_rejected_after_reopen() {
        let root = private_root();
        let store = DiscoveryRegistry::open(root.path()).unwrap();
        store.retain_validated(TOPIC, INPUT).unwrap();
        let old = StoredConfig {
            version: 1,
            id: "study/temp".into(),
            topic: "homeassistant/sensor/study/temp/config".into(),
            sensors: vec![StoredSensor {
                unique_id: "lab_bedroom".into(),
                platform: "sensor".into(),
                state_topic: "lab/readonly/independent".into(),
            }],
        };
        {
            let mut tx = store.db.begin_write().unwrap();
            tx.set_durability(Durability::Immediate).unwrap();
            tx.set_two_phase_commit(true);
            {
                let mut table = tx.open_table(CONFIGS).unwrap();
                table
                    .insert(
                        old.id.as_str(),
                        serde_json::to_string(&old).unwrap().as_str(),
                    )
                    .unwrap();
            }
            tx.commit().unwrap();
        }
        drop(store);
        let reopened = DiscoveryRegistry::open(root.path()).unwrap();
        assert!(
            reopened.recover_unverified().is_err(),
            "pre-fix duplicate sensor identities must not be recovered as distinct owners"
        );
        assert!(reopened.remove_retained(&old.topic, true).unwrap());
        assert_eq!(reopened.recover_unverified().unwrap().len(), 1);
    }

    #[test]
    fn private_owner_mode_and_symlink_are_required() {
        let root = private_root();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o755)).unwrap();
        assert!(DiscoveryRegistry::open(root.path()).is_err());
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let store = DiscoveryRegistry::open(root.path()).unwrap();
        drop(store);
        fs::set_permissions(
            root.path().join("mqtt-discovery.redb"),
            fs::Permissions::from_mode(0o644),
        )
        .unwrap();
        assert!(DiscoveryRegistry::open(root.path()).is_err());
        let file = root.path().join("mqtt-discovery.redb");
        fs::remove_file(&file).unwrap();
        let shadow = root.path().join("shadow.redb");
        fs::write(&shadow, b"private").unwrap();
        std::os::unix::fs::symlink(&shadow, &file).unwrap();
        assert!(DiscoveryRegistry::open(root.path()).is_err());
        let link = root.path().with_extension("linked-dir");
        std::os::unix::fs::symlink(root.path(), &link).unwrap();
        assert!(DiscoveryRegistry::open(&link).is_err());
        fs::remove_file(link).unwrap();
    }
}
