//! P1f: locally fenced SYNTHETIC actuator, redb transaction = authority + effect.
//! NOT a driver lease protocol: devices must independently reject old epochs.
use anyhow::{Context, Result, bail};
use redb::{Database, Durability, ReadableDatabase, ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;

const OWNER: TableDefinition<&str, &str> = TableDefinition::new("lab_fence_owner_v1");
const EFFECTS: TableDefinition<&str, &str> = TableDefinition::new("lab_fenced_effects_v1");
const STATE: TableDefinition<&str, &str> = TableDefinition::new("lab_fenced_state_v1");

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Epoch {
    pub version: u8,
    pub value: u64,
    pub owner: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Effect {
    pub version: u8,
    pub command_id: String,
    pub owner: String,
    pub epoch: u64,
    pub requested_on: bool,
}
pub struct FencedSink {
    db: Database,
}

fn safe_id(id: &str) -> bool {
    (1..=96).contains(&id.len())
        && id
            .bytes()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == b'-' || ch == b'_')
}
fn private_dir(root: &Path) -> Result<()> {
    let meta = fs::symlink_metadata(root)?;
    if !meta.is_dir()
        || meta.file_type().is_symlink()
        || meta.uid() != nix::unistd::Uid::current().as_raw()
        || meta.permissions().mode() & 0o077 != 0
    {
        bail!("lab fenced sink requires owned private directory mode 0700");
    }
    Ok(())
}
fn valid_epoch(e: &Epoch) -> bool {
    e.version == 1 && e.value > 0 && safe_id(&e.owner)
}
impl FencedSink {
    pub fn open(root: &Path) -> Result<Self> {
        private_dir(root)?;
        let path = root.join("fence.redb");
        match fs::symlink_metadata(&path) {
            Ok(meta)
                if !meta.is_file()
                    || meta.file_type().is_symlink()
                    || meta.uid() != nix::unistd::Uid::current().as_raw()
                    || meta.permissions().mode() & 0o077 != 0 =>
            {
                bail!("unsafe fence database path")
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
        let db = Database::create(&path).context("fenced sink open failed")?;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
        let mut tx = db.begin_write()?;
        tx.set_durability(Durability::Immediate)?;
        tx.set_two_phase_commit(true);
        {
            let _a = tx.open_table(OWNER)?;
            let _b = tx.open_table(EFFECTS)?;
            let _c = tx.open_table(STATE)?;
        }
        tx.commit()?;
        fs::File::open(root)?.sync_all()?;
        Ok(Self { db })
    }
    fn snapshot_epoch(&self) -> Result<Option<Epoch>> {
        let tx = self.db.begin_read()?;
        let table = tx.open_table(OWNER)?;
        let found = table
            .get("sole")?
            .map(|v| serde_json::from_str::<Epoch>(v.value()))
            .transpose()?;
        if found.as_ref().is_some_and(|e| !valid_epoch(e)) {
            bail!("corrupt fence authority; fail closed");
        }
        Ok(found)
    }
    pub fn current_epoch(&self) -> Result<Option<Epoch>> {
        self.snapshot_epoch()
    }
    pub fn grant_first(&self, owner: &str) -> Result<Epoch> {
        if !safe_id(owner) {
            bail!("invalid owner identifier");
        }
        let mut tx = self.db.begin_write()?;
        tx.set_durability(Durability::Immediate)?;
        tx.set_two_phase_commit(true);
        let next = Epoch {
            version: 1,
            value: 1,
            owner: owner.into(),
        };
        {
            let mut table = tx.open_table(OWNER)?;
            if table.get("sole")?.is_some() {
                bail!("owner already issued; use explicit transfer");
            }
            let payload = serde_json::to_string(&next)?;
            table.insert("sole", payload.as_str())?;
        }
        tx.commit()?;
        Ok(next)
    }
    /// Administrative takeover: MUST know exact current epoch/owner.
    /// No timeout- or wall-clock-based implicit lease expiry.
    pub fn transfer(&self, old_owner: &str, old_epoch: u64, new_owner: &str) -> Result<Epoch> {
        if !safe_id(old_owner) || !safe_id(new_owner) || old_owner == new_owner {
            bail!("invalid transfer participants");
        }
        let mut tx = self.db.begin_write()?;
        tx.set_durability(Durability::Immediate)?;
        tx.set_two_phase_commit(true);
        let next;
        {
            let mut table = tx.open_table(OWNER)?;
            let old = table
                .get("sole")?
                .ok_or_else(|| anyhow::anyhow!("fence uninitialized"))?;
            let current: Epoch = serde_json::from_str(old.value())?;
            drop(old);
            if !valid_epoch(&current) || current.owner != old_owner || current.value != old_epoch {
                bail!("fence transfer denied: wrong current epoch or owner");
            }
            next = Epoch {
                version: 1,
                value: current.value.checked_add(1).context("epoch overflow")?,
                owner: new_owner.into(),
            };
            let payload = serde_json::to_string(&next)?;
            table.insert("sole", payload.as_str())?;
        }
        tx.commit()?;
        Ok(next)
    }
    /// All synthetic state changes occur in the SAME redb write transaction
    /// as the current epoch check; obsolete controllers cannot commit here.
    pub fn apply(&self, owner: &str, epoch: u64, command_id: &str, value: bool) -> Result<bool> {
        if !safe_id(owner) || !safe_id(command_id) || epoch == 0 {
            bail!("unsafe synthetic fence operation");
        }
        let mut tx = self.db.begin_write()?;
        tx.set_durability(Durability::Immediate)?;
        tx.set_two_phase_commit(true);
        {
            let table = tx.open_table(OWNER)?;
            let raw = table
                .get("sole")?
                .ok_or_else(|| anyhow::anyhow!("fence not initialized"))?;
            let current: Epoch = serde_json::from_str(raw.value())?;
            if !valid_epoch(&current) || current.owner != owner || current.value != epoch {
                bail!("stale controller fenced before effect");
            }
        }
        let effect = Effect {
            version: 1,
            command_id: command_id.into(),
            owner: owner.into(),
            epoch,
            requested_on: value,
        };
        {
            let mut table = tx.open_table(EFFECTS)?;
            if let Some(prev) = table.get(command_id)? {
                let previous: Effect = serde_json::from_str(prev.value())?;
                if previous != effect {
                    bail!("idempotency collision on synthetic effect");
                }
                return Ok(false);
            }
            let payload = serde_json::to_string(&effect)?;
            table.insert(command_id, payload.as_str())?;
        }
        {
            let mut table = tx.open_table(STATE)?;
            let payload = serde_json::to_string(&effect)?;
            table.insert("actuator", payload.as_str())?;
        }
        tx.commit()
            .context("synthetic fenced effect not committed")?;
        Ok(true)
    }
    pub fn actuator(&self) -> Result<Option<Effect>> {
        let tx = self.db.begin_read()?;
        let table = tx.open_table(STATE)?;
        let effect = table
            .get("actuator")?
            .map(|v| serde_json::from_str::<Effect>(v.value()))
            .transpose()?;
        if effect.as_ref().is_some_and(|e| {
            e.version != 1 || !safe_id(&e.owner) || !safe_id(&e.command_id) || e.epoch == 0
        }) {
            bail!("invalid synthetic actuator state");
        }
        Ok(effect)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn setup() -> (tempfile::TempDir, FencedSink) {
        let dir = tempfile::tempdir().unwrap();
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let db = FencedSink::open(dir.path()).unwrap();
        (dir, db)
    }
    #[test]
    fn epoch_increases_and_old_owner_never_reenters() {
        let (_, db) = setup();
        let first = db.grant_first("driver_one").unwrap();
        assert_eq!(first.value, 1);
        assert!(db.grant_first("driver_one").is_err());
        assert!(db.apply("driver_one", 1, "op1", true).unwrap());
        assert!(!db.apply("driver_one", 1, "op1", true).unwrap());
        assert!(db.apply("driver_one", 1, "op1", false).is_err());
        let next = db.transfer("driver_one", 1, "driver_two").unwrap();
        assert_eq!(next.value, 2);
        assert!(db.apply("driver_one", 1, "old-op", false).is_err());
        assert!(db.transfer("driver_one", 1, "driver_three").is_err());
        assert!(db.apply("driver_two", 2, "op2", false).unwrap());
        assert!(!db.actuator().unwrap().unwrap().requested_on);
    }
    #[test]
    fn reboot_retains_monotonic_epoch_and_effect() {
        let (dir, db) = setup();
        db.grant_first("driver_one").unwrap();
        db.apply("driver_one", 1, "op1", true).unwrap();
        drop(db);
        let db = FencedSink::open(dir.path()).unwrap();
        assert_eq!(db.current_epoch().unwrap().unwrap().value, 1);
        assert!(db.actuator().unwrap().unwrap().requested_on);
        db.transfer("driver_one", 1, "driver_two").unwrap();
        drop(db);
        let db = FencedSink::open(dir.path()).unwrap();
        assert!(db.apply("driver_one", 1, "stale-op", false).is_err());
        assert_eq!(db.current_epoch().unwrap().unwrap().value, 2);
    }
    #[test]
    fn symlink_and_public_directory_refused() {
        let (dir, db) = setup();
        drop(db);
        let link = dir.path().join("alias");
        std::os::unix::fs::symlink(dir.path().join("fence.redb"), &link).unwrap();
        assert!(link.exists());
        let dir2 = tempfile::tempdir().unwrap();
        fs::set_permissions(dir2.path(), fs::Permissions::from_mode(0o755)).unwrap();
        assert!(FencedSink::open(dir2.path()).is_err());
        let alias = dir.path().join("fence.redb");
        fs::remove_file(&alias).unwrap();
        std::os::unix::fs::symlink(&link, &alias).unwrap();
        assert!(FencedSink::open(dir.path()).is_err());
    }
    #[test]
    fn rejects_unsafe_ids_and_unauthorized_takeover() {
        let (_, db) = setup();
        assert!(db.grant_first("../bad").is_err());
        assert!(db.grant_first("one").is_ok());
        assert!(db.transfer("wrong", 1, "two").is_err());
        assert!(db.transfer("one", 55, "two").is_err());
        assert!(db.apply("one", 1, "../bad", true).is_err());
    }
}
