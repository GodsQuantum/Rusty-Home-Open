//! P1d lab-only durable command intent ledger. Not a hardware exactly-once protocol.
//! A dispatch attempt is marked durable BEFORE a side effect. After a crash,
//! in-flight commands remain UNCERTAIN, never automatically redelivered.
use anyhow::{Context, Result, bail};
use redb::{Database, Durability, ReadableDatabase, ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;

const COMMANDS: TableDefinition<&str, &str> = TableDefinition::new("lab_command_intents_v1");
const REVIEWS: TableDefinition<&str, &str> = TableDefinition::new("lab_command_reviews_v1");
const MAX_KEY: usize = 96;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Prepared,
    InFlight,
    Acknowledged,
    Failed,
    /// Effect is unknown; permanently blocked from redispatch.
    Quarantined,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Intent {
    pub version: u8,
    pub id: String,
    pub target: String,
    pub requested_on: bool,
    pub phase: Phase,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReconciliationReview {
    pub version: u8,
    pub id: String,
    /// Opaque, non-secret test evidence reference. Does NOT mean applied/failed.
    pub evidence_ref: String,
    pub outcome: String,
}
pub struct CommandLedger {
    db: Database,
}

fn validate_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= MAX_KEY
        && id
            .bytes()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == b'-' || ch == b'_')
}
fn check_private_dir(root: &Path) -> Result<()> {
    let m = fs::symlink_metadata(root)?;
    if !m.is_dir()
        || m.file_type().is_symlink()
        || m.uid() != nix::unistd::Uid::current().as_raw()
        || m.permissions().mode() & 0o077 != 0
    {
        bail!("ledger requires a private, owner-matched mode-0700 directory")
    }
    Ok(())
}
impl CommandLedger {
    pub fn open(root: &Path) -> Result<Self> {
        check_private_dir(root)?;
        let file = root.join("commands.redb");
        match fs::symlink_metadata(&file) {
            Ok(m)
                if !m.is_file()
                    || m.file_type().is_symlink()
                    || m.uid() != nix::unistd::Uid::current().as_raw()
                    || m.permissions().mode() & 0o077 != 0 =>
            {
                bail!("ledger file is unsafe")
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
        let db = Database::create(&file).context("redb cannot open command ledger")?;
        fs::set_permissions(&file, fs::Permissions::from_mode(0o600))?;
        let mut tx = db.begin_write()?;
        tx.set_durability(Durability::Immediate)?;
        tx.set_two_phase_commit(true);
        {
            let _ = tx.open_table(COMMANDS)?;
            let _ = tx.open_table(REVIEWS)?;
        }
        tx.commit()?;
        fs::File::open(root)?.sync_all()?;
        Ok(Self { db })
    }
    pub fn lookup(&self, id: &str) -> Result<Option<Intent>> {
        if !validate_id(id) {
            bail!("invalid id")
        }
        let tx = self.db.begin_read()?;
        let table = tx.open_table(COMMANDS)?;
        let raw = table.get(id)?;
        let val = raw
            .map(|entry| serde_json::from_str::<Intent>(entry.value()))
            .transpose()?;
        Ok(val)
    }
    pub fn prepare(&self, id: &str, target: &str, requested_on: bool) -> Result<Intent> {
        if !validate_id(id) || target != "hars-lab:actuator/on" {
            bail!("lab ledger accepts only synthetic actuator and safe id")
        }
        let mut tx = self.db.begin_write()?;
        tx.set_durability(Durability::Immediate)?;
        tx.set_two_phase_commit(true);
        let record;
        {
            let mut table = tx.open_table(COMMANDS)?;
            let existing = table.get(id)?.map(|v| v.value().to_string());
            if let Some(existing) = existing {
                let saved: Intent = serde_json::from_str(&existing)?;
                if saved.version != 1
                    || saved.target != target
                    || saved.requested_on != requested_on
                {
                    bail!("idempotency key already used for a different command")
                }
                // Transaction is dropped without a write; never replays an in-flight attempt.
                return Ok(saved);
            }
            record = Intent {
                version: 1,
                id: id.to_string(),
                target: target.to_string(),
                requested_on,
                phase: Phase::Prepared,
            };
            let encoded = serde_json::to_string(&record)?;
            table.insert(id, encoded.as_str())?;
        }
        tx.commit().context("intent not durably prepared")?;
        Ok(record)
    }
    fn transition(&self, id: &str, from: Phase, to: Phase) -> Result<Intent> {
        if !validate_id(id) {
            bail!("invalid id")
        }
        let mut tx = self.db.begin_write()?;
        tx.set_durability(Durability::Immediate)?;
        tx.set_two_phase_commit(true);
        let next;
        {
            let mut table = tx.open_table(COMMANDS)?;
            let current = table
                .get(id)?
                .ok_or_else(|| anyhow::anyhow!("unknown idempotency key"))?;
            let mut record: Intent = serde_json::from_str(current.value())?;
            drop(current);
            if record.version != 1 || record.phase != from {
                bail!("transition forbidden; requires explicit reconciliation after a crash")
            }
            record.phase = to;
            let encoded = serde_json::to_string(&record)?;
            table.insert(id, encoded.as_str())?;
            next = record;
        }
        tx.commit()
            .context("command status not durably committed")?;
        Ok(next)
    }
    /// Must commit BEFORE emitting an external command. Never claim twice.
    pub fn claim_once(&self, id: &str) -> Result<Intent> {
        self.transition(id, Phase::Prepared, Phase::InFlight)
    }
    /// Only call after a validated driver response. Cannot be atomic with hardware.
    pub fn acknowledge(&self, id: &str) -> Result<Intent> {
        self.transition(id, Phase::InFlight, Phase::Acknowledged)
    }
    pub fn reject(&self, id: &str) -> Result<Intent> {
        self.transition(id, Phase::InFlight, Phase::Failed)
    }
    /// Read-only, deterministic triage of uncertain commands; no command emitted.
    /// A quarantined command retains an UNKNOWN hardware outcome.
    pub fn needs_reconciliation(&self) -> Result<Vec<Intent>> {
        let tx = self.db.begin_read()?;
        let table = tx.open_table(COMMANDS)?;
        let mut unresolved = Vec::new();
        for entry in table.iter()? {
            let (key, raw) = entry?;
            let intent: Intent = serde_json::from_str(raw.value())?;
            if intent.version != 1
                || intent.id != key.value()
                || !validate_id(&intent.id)
                || intent.target != "hars-lab:actuator/on"
            {
                bail!("corrupt command record, refusing reconciliation scan");
            }
            if matches!(intent.phase, Phase::InFlight | Phase::Quarantined) {
                unresolved.push(intent);
            }
        }
        Ok(unresolved)
    }
    pub fn reconciliation_review(&self, id: &str) -> Result<Option<ReconciliationReview>> {
        if !validate_id(id) {
            bail!("invalid id")
        }
        let tx = self.db.begin_read()?;
        let table = tx.open_table(REVIEWS)?;
        let review = table
            .get(id)?
            .map(|v| serde_json::from_str::<ReconciliationReview>(v.value()))
            .transpose()?;
        Ok(review)
    }
    /// Lab only. Explicitly quarantine an IN_FLIGHT intent with an opaque proof
    /// reference. This does NOT confirm command success or failure; it only
    /// disables all automatic retries and documents that outcome is unknown.
    /// State and evidence are committed atomically to the *same* redb database.
    pub fn quarantine_uncertain(&self, id: &str, evidence_ref: &str) -> Result<Intent> {
        if !validate_id(id) || !validate_id(evidence_ref) {
            bail!("id and evidence reference must be safe, bounded opaque identifiers");
        }
        let mut tx = self.db.begin_write()?;
        tx.set_durability(Durability::Immediate)?;
        tx.set_two_phase_commit(true);
        let next;
        {
            let mut commands = tx.open_table(COMMANDS)?;
            let current = commands
                .get(id)?
                .ok_or_else(|| anyhow::anyhow!("unknown intent"))?;
            let mut record: Intent = serde_json::from_str(current.value())?;
            drop(current);
            if record.version != 1
                || record.id != id
                || record.target != "hars-lab:actuator/on"
                || record.phase != Phase::InFlight
            {
                bail!("only an authentic in-flight synthetic command may be quarantined");
            }
            record.phase = Phase::Quarantined;
            let encoded = serde_json::to_string(&record)?;
            commands.insert(id, encoded.as_str())?;
            next = record;
        }
        {
            let mut reviews = tx.open_table(REVIEWS)?;
            if reviews.get(id)?.is_some() {
                bail!("reconciliation evidence already exists")
            }
            let audit = ReconciliationReview {
                version: 1,
                id: id.to_string(),
                evidence_ref: evidence_ref.to_string(),
                outcome: "effect_unknown_no_retry".to_string(),
            };
            let encoded = serde_json::to_string(&audit)?;
            reviews.insert(id, encoded.as_str())?;
        }
        tx.commit()
            .context("reconciliation quarantine not durably recorded")?;
        Ok(next)
    }
    /// Lab-only adapter to Moli Hub's *synthetic* actuator.
    /// A recovered IN_FLIGHT remains ambiguous and is NEVER sent again.
    /// No hardware-level exactly-once guarantee.
    pub async fn dispatch_synthetic_once(
        &self,
        hub: &moli_runtime::Hub,
        id: &str,
        requested_on: bool,
    ) -> Result<bool> {
        let existing = self.prepare(id, "hars-lab:actuator/on", requested_on)?;
        if existing.phase != Phase::Prepared {
            return Ok(false);
        }
        self.claim_once(id)?;
        // If the call returns Err, preserve IN_FLIGHT: transport failures do
        // not prove that a real actuator did not execute a command.
        hub.command(
            &moli_core::PointId::from("hars-lab:actuator/on"),
            moli_core::Value::Bool(requested_on),
            moli_core::Origin::Ui,
            None,
        )
        .await
        .context("Moli synthetic driver call uncertain; leave IN_FLIGHT")?;
        self.acknowledge(id)?;
        Ok(true)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn setup() -> tempfile::TempDir {
        let temp = tempfile::tempdir().unwrap();
        fs::set_permissions(temp.path(), fs::Permissions::from_mode(0o700)).unwrap();
        temp
    }
    #[test]
    fn transactions_survive_reopen_and_deduplicate() {
        let d = setup();
        {
            let ledger = CommandLedger::open(d.path()).unwrap();
            let p = ledger
                .prepare("key_123", "hars-lab:actuator/on", true)
                .unwrap();
            assert_eq!(p.phase, Phase::Prepared);
            assert_eq!(
                ledger
                    .prepare("key_123", "hars-lab:actuator/on", true)
                    .unwrap(),
                p
            );
            assert!(
                ledger
                    .prepare("key_123", "hars-lab:actuator/on", false)
                    .is_err()
            );
            assert_eq!(ledger.claim_once("key_123").unwrap().phase, Phase::InFlight);
            assert!(ledger.claim_once("key_123").is_err());
        }
        {
            let ledger = CommandLedger::open(d.path()).unwrap();
            assert_eq!(
                ledger.lookup("key_123").unwrap().unwrap().phase,
                Phase::InFlight
            );
            assert_eq!(
                ledger
                    .prepare("key_123", "hars-lab:actuator/on", true)
                    .unwrap()
                    .phase,
                Phase::InFlight
            );
            assert!(
                ledger.claim_once("key_123").is_err(),
                "must not re-dispatch on restart"
            );
            assert_eq!(
                ledger.acknowledge("key_123").unwrap().phase,
                Phase::Acknowledged
            );
            assert!(ledger.acknowledge("key_123").is_err());
        }
        assert_eq!(
            CommandLedger::open(d.path())
                .unwrap()
                .lookup("key_123")
                .unwrap()
                .unwrap()
                .phase,
            Phase::Acknowledged
        );
    }
    #[test]
    fn unsafe_path_and_conflicting_keys_are_denied() {
        let d = setup();
        let path = d.path().join("commands.redb");
        std::os::unix::fs::symlink("/tmp/not-ours", &path).unwrap();
        assert!(CommandLedger::open(d.path()).is_err());
        fs::remove_file(&path).unwrap();
        let l = CommandLedger::open(d.path()).unwrap();
        assert!(
            l.prepare("../../hmm", "hars-lab:actuator/on", true)
                .is_err()
        );
        assert!(l.prepare("good", "real-device/on", true).is_err());
        assert!(l.claim_once("unknown").is_err());
        assert!(l.reject("unknown").is_err());
    }
    #[tokio::test]
    async fn native_moli_synthetic_actuator_receives_only_one_idempotent_order() {
        use moli_runtime::{Hub, HubOptions, spawn_driver};
        use std::sync::Arc;
        use tokio_util::sync::CancellationToken;
        let d = setup();
        let hub = Hub::new(HubOptions::default()).unwrap();
        let stop = CancellationToken::new();
        let task = spawn_driver(
            &hub,
            moli_core::InstanceId::from("hars-lab"),
            Arc::new(crate::Synthetic),
            stop.clone(),
        );
        crate::ready(&hub).await.unwrap();
        let ledger = CommandLedger::open(d.path()).unwrap();
        assert!(
            ledger
                .dispatch_synthetic_once(&hub, "unique-order", true)
                .await
                .unwrap()
        );
        assert_eq!(
            hub.state(&moli_core::PointId::from("hars-lab:actuator/on"))
                .unwrap()
                .value,
            moli_core::Value::Bool(true)
        );
        assert_eq!(
            ledger.lookup("unique-order").unwrap().unwrap().phase,
            Phase::Acknowledged
        );
        assert!(
            !ledger
                .dispatch_synthetic_once(&hub, "unique-order", true)
                .await
                .unwrap()
        );
        drop(ledger);
        let reopened = CommandLedger::open(d.path()).unwrap();
        assert!(
            !reopened
                .dispatch_synthetic_once(&hub, "unique-order", true)
                .await
                .unwrap()
        );
        assert!(
            reopened
                .dispatch_synthetic_once(&hub, "unique-order", false)
                .await
                .is_err()
        );
        stop.cancel();
        let _ = tokio::time::timeout(std::time::Duration::from_secs(3), task).await;
    }
    #[test]
    fn uncertain_intent_is_quarantined_with_atomic_evidence_across_reopen() {
        let d = setup();
        {
            let ledger = CommandLedger::open(d.path()).unwrap();
            ledger
                .prepare("lost_ack", "hars-lab:actuator/on", true)
                .unwrap();
            ledger.claim_once("lost_ack").unwrap();
            ledger
                .prepare("not_dispatched", "hars-lab:actuator/on", false)
                .unwrap();
            let pending = ledger.needs_reconciliation().unwrap();
            assert_eq!(pending.len(), 1);
            assert_eq!(pending[0].id, "lost_ack");
            assert_eq!(pending[0].phase, Phase::InFlight);
            let quarantined = ledger
                .quarantine_uncertain("lost_ack", "synthetic_review_001")
                .unwrap();
            assert_eq!(quarantined.phase, Phase::Quarantined);
            assert!(ledger.acknowledge("lost_ack").is_err());
            assert!(ledger.reject("lost_ack").is_err());
            assert!(ledger.claim_once("lost_ack").is_err());
            assert!(
                ledger
                    .quarantine_uncertain("lost_ack", "synthetic_review_001")
                    .is_err()
            );
        }
        let reopened = CommandLedger::open(d.path()).unwrap();
        let pending = reopened.needs_reconciliation().unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].phase, Phase::Quarantined);
        let review = reopened.reconciliation_review("lost_ack").unwrap().unwrap();
        assert_eq!(review.evidence_ref, "synthetic_review_001");
        assert_eq!(review.outcome, "effect_unknown_no_retry");
        assert!(
            !reopened
                .prepare("lost_ack", "hars-lab:actuator/on", true)
                .unwrap()
                .phase
                .eq(&Phase::Prepared)
        );
        assert!(
            reopened
                .prepare("lost_ack", "hars-lab:actuator/on", false)
                .is_err()
        );
        assert!(
            reopened
                .reconciliation_review("not_dispatched")
                .unwrap()
                .is_none()
        );
    }
    #[test]
    fn quarantine_requires_matching_inflight_and_nonempty_safe_evidence() {
        let d = setup();
        let l = CommandLedger::open(d.path()).unwrap();
        l.prepare("x", "hars-lab:actuator/on", true).unwrap();
        assert!(l.quarantine_uncertain("x", "review1").is_err());
        assert!(l.reconciliation_review("x").unwrap().is_none());
        l.claim_once("x").unwrap();
        for bad in [
            "",
            "../../outside",
            "has whitespace",
            "a".repeat(99).as_str(),
        ] {
            assert!(l.quarantine_uncertain("x", bad).is_err());
        }
        assert!(l.reconciliation_review("x").unwrap().is_none());
        assert_eq!(l.lookup("x").unwrap().unwrap().phase, Phase::InFlight);
        assert!(l.quarantine_uncertain("unknown", "review1").is_err());
        assert_eq!(
            l.quarantine_uncertain("x", "manual_fake_ref_1")
                .unwrap()
                .phase,
            Phase::Quarantined
        );
        assert!(l.acknowledge("x").is_err());
    }
    #[test]
    fn pending_scan_rejects_corrupt_intents_instead_of_silently_skipping() {
        let d = setup();
        let l = CommandLedger::open(d.path()).unwrap();
        let mut tx = l.db.begin_write().unwrap();
        tx.set_durability(Durability::Immediate).unwrap();
        {
            let mut table = tx.open_table(COMMANDS).unwrap();
            table
                .insert("evil", "{\"version\":1,\"id\":\"elsewhere\"}")
                .unwrap();
        }
        tx.commit().unwrap();
        assert!(l.needs_reconciliation().is_err());
    }
    #[test]
    fn no_implicit_delivery_after_restart() {
        let d = setup();
        let l = CommandLedger::open(d.path()).unwrap();
        assert_eq!(
            l.prepare("waiting", "hars-lab:actuator/on", false)
                .unwrap()
                .phase,
            Phase::Prepared
        );
        drop(l);
        let l = CommandLedger::open(d.path()).unwrap();
        assert_eq!(l.lookup("waiting").unwrap().unwrap().phase, Phase::Prepared);
        assert_eq!(l.claim_once("waiting").unwrap().phase, Phase::InFlight);
        assert_eq!(l.reject("waiting").unwrap().phase, Phase::Failed);
        assert!(l.claim_once("waiting").is_err());
    }
}
