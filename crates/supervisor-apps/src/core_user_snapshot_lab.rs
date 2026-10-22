//! P8i.d.5: Core user status from a lab-only WebSocket result fixture.
//! NOT AUTHENTICATED: production must use an authenticated Core adapter.
//! No Actor, grant, HTTP endpoint or Supervisor capability is produced.
use crate::ingress_identity_lab::IngressUserClaim;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, PartialEq, Eq)]
pub enum CoreUserError {
    Invalid,
    Mismatch,
    Duplicate,
    Stale,
}
#[derive(Debug, PartialEq, Eq)]
pub enum CoreAccountStatus {
    ActiveAdmin,
    ActiveNonAdmin,
    Denied,
}
struct Flags {
    active: bool,
    owner: bool,
    system: bool,
    admin_group: bool,
}
pub struct CoreUserSnapshot {
    observed: u64,
    users: BTreeMap<String, Flags>,
}
fn id_valid(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}
impl CoreUserSnapshot {
    /// Only parse data that a future authenticated, authorized Core adapter
    /// verified. Never accept the data from an Ingress HTTP client.
    pub fn parse_fixture(
        wire: &[u8],
        correlation: u64,
        observed: u64,
    ) -> Result<Self, CoreUserError> {
        if wire.is_empty() || wire.len() > 524_288 || correlation == 0 {
            return Err(CoreUserError::Invalid);
        }
        let value = serde_json::from_slice::<crate::NoDuplicates>(wire)
            .map_err(|_| CoreUserError::Invalid)?
            .0;
        if value.get("type").and_then(Value::as_str) != Some("result")
            || value.get("id").and_then(Value::as_u64) != Some(correlation)
        {
            return Err(CoreUserError::Mismatch);
        }
        if value.get("success").and_then(Value::as_bool) != Some(true) {
            return Err(CoreUserError::Invalid);
        }
        let records = value
            .get("result")
            .and_then(Value::as_array)
            .ok_or(CoreUserError::Invalid)?;
        if records.len() > 4096 {
            return Err(CoreUserError::Invalid);
        }
        let mut users = BTreeMap::new();
        for u in records {
            let record = u.as_object().ok_or(CoreUserError::Invalid)?;
            let id = record
                .get("id")
                .and_then(Value::as_str)
                .filter(|s| id_valid(s))
                .ok_or(CoreUserError::Invalid)?;
            let bit = |key: &str| -> Result<bool, CoreUserError> {
                record
                    .get(key)
                    .and_then(Value::as_bool)
                    .ok_or(CoreUserError::Invalid)
            };
            let groups = record
                .get("group_ids")
                .and_then(Value::as_array)
                .filter(|x| x.len() <= 32)
                .ok_or(CoreUserError::Invalid)?;
            let mut admin = false;
            for group in groups {
                let name = group
                    .as_str()
                    .filter(|x| id_valid(x))
                    .ok_or(CoreUserError::Invalid)?;
                if name == "system-admin" {
                    admin = true;
                }
            }
            let flags = Flags {
                active: bit("is_active")?,
                owner: bit("is_owner")?,
                system: bit("system_generated")?,
                admin_group: admin,
            };
            if users.insert(id.to_string(), flags).is_some() {
                return Err(CoreUserError::Duplicate);
            }
        }
        Ok(Self { observed, users })
    }
    /// Information only. Snapshot max age 15s; does not issue authority.
    pub fn status(
        &self,
        claim: &IngressUserClaim,
        now: u64,
    ) -> Result<CoreAccountStatus, CoreUserError> {
        if now < self.observed || now - self.observed >= 15 {
            return Err(CoreUserError::Stale);
        }
        Ok(match self.users.get(claim.as_str()) {
            None => CoreAccountStatus::Denied,
            Some(u) if !u.active || u.system => CoreAccountStatus::Denied,
            Some(u) if u.owner || u.admin_group => CoreAccountStatus::ActiveAdmin,
            Some(_) => CoreAccountStatus::ActiveNonAdmin,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingress_identity_lab::ingress_user_claim;
    use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
    fn claim(id: &str) -> IngressUserClaim {
        ingress_user_claim(
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(172, 30, 32, 2), 8099)),
            &[("X-Remote-User-Id", id)],
        )
        .unwrap()
    }
    fn rows() -> Vec<Value> {
        vec![
            serde_json::json!({"id":"ownerA","is_owner":true,"is_active":true,"system_generated":false,"group_ids":["system-users"]}),
            serde_json::json!({"id":"adminB","is_owner":false,"is_active":true,"system_generated":false,"group_ids":["system-admin"]}),
            serde_json::json!({"id":"householdC","is_owner":false,"is_active":true,"system_generated":false,"group_ids":["system-users"]}),
            serde_json::json!({"id":"disabledD","is_owner":false,"is_active":false,"system_generated":false,"group_ids":["system-admin"]}),
            serde_json::json!({"id":"systemE","is_owner":false,"is_active":true,"system_generated":true,"group_ids":["system-admin"]}),
        ]
    }
    fn msg(rows: Vec<Value>) -> Vec<u8> {
        serde_json::to_vec(
            &serde_json::json!({"id":17,"type":"result","success":true,"result":rows}),
        )
        .unwrap()
    }
    #[test]
    fn real_groups_owner_disabled_system_and_unknown_are_not_equivalent() {
        let s = CoreUserSnapshot::parse_fixture(&msg(rows()), 17, 100).unwrap();
        assert_eq!(
            s.status(&claim("ownerA"), 100).unwrap(),
            CoreAccountStatus::ActiveAdmin
        );
        assert_eq!(
            s.status(&claim("adminB"), 101).unwrap(),
            CoreAccountStatus::ActiveAdmin
        );
        assert_eq!(
            s.status(&claim("householdC"), 102).unwrap(),
            CoreAccountStatus::ActiveNonAdmin
        );
        for id in ["disabledD", "systemE", "unknownF"] {
            assert_eq!(
                s.status(&claim(id), 103).unwrap(),
                CoreAccountStatus::Denied
            );
        }
    }
    #[test]
    fn demotion_and_deactivation_are_observed_on_new_snapshot() {
        let old = CoreUserSnapshot::parse_fixture(&msg(rows()), 17, 100).unwrap();
        assert_eq!(
            old.status(&claim("adminB"), 101).unwrap(),
            CoreAccountStatus::ActiveAdmin
        );
        let mut changed = rows();
        changed[1]["group_ids"] = serde_json::json!(["system-users"]);
        let new = CoreUserSnapshot::parse_fixture(&msg(changed), 17, 102).unwrap();
        assert_eq!(
            new.status(&claim("adminB"), 103).unwrap(),
            CoreAccountStatus::ActiveNonAdmin
        );
        let mut changed = rows();
        changed[0]["is_active"] = Value::Bool(false);
        let revoked = CoreUserSnapshot::parse_fixture(&msg(changed), 17, 104).unwrap();
        assert_eq!(
            revoked.status(&claim("ownerA"), 105).unwrap(),
            CoreAccountStatus::Denied
        );
    }
    #[test]
    fn time_bounds_and_envelope_correlation_fail_closed() {
        let snap = CoreUserSnapshot::parse_fixture(&msg(rows()), 17, 100).unwrap();
        assert_eq!(
            snap.status(&claim("adminB"), 114).unwrap(),
            CoreAccountStatus::ActiveAdmin
        );
        assert_eq!(
            snap.status(&claim("adminB"), 115).err(),
            Some(CoreUserError::Stale)
        );
        assert_eq!(
            snap.status(&claim("adminB"), 99).err(),
            Some(CoreUserError::Stale)
        );
        assert_eq!(
            CoreUserSnapshot::parse_fixture(&msg(rows()), 18, 100).err(),
            Some(CoreUserError::Mismatch)
        );
        let dupe = br#"{"id":17,"type":"result","success":true,"success":true,"result":[]}"#;
        assert!(CoreUserSnapshot::parse_fixture(dupe, 17, 100).is_err());
        for wire in [
            br#"{"id":17,"type":"result","success":false,"result":[]}"#.as_slice(),
            br#"{"id":17,"type":"event","success":true,"result":[]}"#,
            br#"{"id":17,"type":"result","success":true,"result":{}}"#,
        ] {
            assert!(CoreUserSnapshot::parse_fixture(wire, 17, 100).is_err());
        }
    }
    #[test]
    fn malformed_flags_groups_and_duplicate_users_rejected() {
        let mut a = rows();
        a[0].as_object_mut().unwrap().remove("is_active");
        assert!(CoreUserSnapshot::parse_fixture(&msg(a), 17, 100).is_err());
        let mut a = rows();
        a[1]["is_active"] = Value::String("true".into());
        assert!(CoreUserSnapshot::parse_fixture(&msg(a), 17, 100).is_err());
        let mut a = rows();
        a[1]["group_ids"] = serde_json::json!(["system-admin", 42]);
        assert!(CoreUserSnapshot::parse_fixture(&msg(a), 17, 100).is_err());
        let mut a = rows();
        a.push(a[0].clone());
        assert_eq!(
            CoreUserSnapshot::parse_fixture(&msg(a), 17, 100).err(),
            Some(CoreUserError::Duplicate)
        );
    }

    #[test]
    fn bounded_adversarial_auth_payloads_never_panic_or_escalate_unknown_user() {
        // Deterministic mutation smoke is NOT libFuzzer/sanitizer coverage or a
        // production security certification. It probes the real bounded JSON
        // decoder with malformed and near-valid WebSocket user responses.
        let valid = msg(rows());
        let unknown = claim("never_present_test_user_5487");

        let assert_fail_closed = |wire: &[u8]| {
            if let Ok(snapshot) = CoreUserSnapshot::parse_fixture(wire, 17, 100) {
                assert_eq!(
                    snapshot.status(&unknown, 100),
                    Ok(CoreAccountStatus::Denied),
                    "Unexpected privileged status for user not in the fixture"
                );
                assert_eq!(
                    snapshot.status(&unknown, 115),
                    Err(CoreUserError::Stale),
                    "An older snapshot must not authorize anyone"
                );
            } // Malformed JSON is expected and must return an error.
        };
        assert_fail_closed(&valid);

        // Exercise every byte of an otherwise correctly correlated frame.
        for index in 0..valid.len() {
            for substitute in [0u8, 0xff, b'"', b'\\', b'[', b'}'] {
                let mut modified = valid.clone();
                modified[index] = substitute;
                assert_fail_closed(&modified);
            }
        }
        // Deep nesting, top-level duplicate keys, JSON type confusion and
        // payloads just above the response cap must always be rejected.
        for wire in [
            br#"{"type":"result","id":17,"success":true,"result":[]}"#.as_slice(),
            br#"{"id":17,"id":17,"type":"result","success":true,"result":[]}"#,
            br#"{"id":17,"type":"event","success":true,"result":[]}"#,
            br#"{"id":"17","type":"result","success":true,"result":[]}"#,
            b"null",
            b"[]",
            b"{",
        ] {
            assert_fail_closed(wire);
        }
        let oversized = vec![b' '; 524_289];
        assert!(matches!(
            CoreUserSnapshot::parse_fixture(&oversized, 17, 100),
            Err(CoreUserError::Invalid)
        ));

        // Reproducible, varied bounded bytes across lengths 0..2047.
        let mut seed = 0xded1_caf7_c3b0_1223_u64;
        for round in 0..20_000_usize {
            let mut wire = vec![0_u8; round % 2048];
            for byte in &mut wire {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                *byte = (seed & 0xff) as u8;
            }
            assert_fail_closed(&wire);
        }
    }
}
