//! PREVIEW ONLY. Read-only authorization decision using *two independent*
//! observations: verified Supervisor ingress identity and fresh authenticated
//! Home Assistant Core user status. Caller must never supply fake Core frames
//! as authority. This module has NO HTTP route, no Supervisor POST, no token,
//! no user persistence and no replay-safe consent: it cannot execute writes.
use crate::core_user_snapshot_lab::{CoreAccountStatus, CoreUserError, CoreUserSnapshot};
use crate::ingress_identity_lab::IngressUserClaim;
use crate::managed_apps::PreparedRequest;
use crate::rbac_lab::{Actor, Decision, Policy, Role};

/// Only explicit per-App grants and currently active Core administrators can
/// pass safe non-destructive preview policy. Dangerous actions always lack
/// consent here; their operations cannot be authorized by this function.
pub fn preview_core_authorization(
    core: &CoreUserSnapshot,
    ingress: &IngressUserClaim,
    now: u64,
    policy: &Policy,
    request: &PreparedRequest,
) -> Result<Decision, CoreUserError> {
    let actor = match core.status(ingress, now)? {
        CoreAccountStatus::Denied => None,
        CoreAccountStatus::ActiveAdmin => Actor::new(ingress.as_str(), Role::Administrator).ok(),
        CoreAccountStatus::ActiveNonAdmin => Actor::new(ingress.as_str(), Role::Household).ok(),
    };
    Ok(policy.evaluate(actor.as_ref(), request, None))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingress_identity_lab::ingress_user_claim;
    use crate::managed_apps::{Action, AppSlug, Operation, prepare};
    use crate::rbac_lab::DecisionCode;
    use serde_json::json;
    use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

    fn claim(subject: &str) -> IngressUserClaim {
        ingress_user_claim(
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(172, 30, 32, 2), 8099)),
            &[("X-Remote-User-Id", subject)],
        )
        .unwrap()
    }
    fn core() -> CoreUserSnapshot {
        let wire = json!({"type":"result","id":17,"success":true,"result":[
            {"id":"owner123","is_owner":true,"is_active":true,
             "system_generated":false,"group_ids":["system-users"]},
            {"id":"operator123","is_owner":false,"is_active":true,
             "system_generated":false,"group_ids":["system-users"]},
            {"id":"disabled123","is_owner":true,"is_active":false,
             "system_generated":false,"group_ids":["system-admin"]},
            {"id":"system123","is_owner":true,"is_active":true,
             "system_generated":true,"group_ids":["system-admin"]}
        ]});
        CoreUserSnapshot::parse_fixture(wire.to_string().as_bytes(), 17, 100).unwrap()
    }
    #[test]
    fn only_current_core_admin_with_exact_app_grant_can_preview_start() {
        let app = AppSlug::parse("future_vendor_random").unwrap();
        let other = AppSlug::parse("other_vendor").unwrap();
        let start = prepare(&app, Operation::Start).unwrap();
        let other_start = prepare(&other, Operation::Start).unwrap();
        let mut policy = Policy::default();
        policy.grant("owner123", &app, &[Action::Start, Action::Install]);
        policy.grant("operator123", &app, &[Action::Start]);
        let state = core();
        assert_eq!(
            preview_core_authorization(&state, &claim("owner123"), 101, &policy, &start)
                .unwrap()
                .code,
            DecisionCode::Allowed
        );
        assert_eq!(
            preview_core_authorization(&state, &claim("owner123"), 101, &policy, &other_start)
                .unwrap()
                .code,
            DecisionCode::UnscopedApp
        );
        assert_eq!(
            preview_core_authorization(&state, &claim("operator123"), 101, &policy, &start)
                .unwrap()
                .code,
            DecisionCode::RoleDenied
        );
        for user in ["disabled123", "system123", "unknown123"] {
            assert_eq!(
                preview_core_authorization(&state, &claim(user), 101, &policy, &start)
                    .unwrap()
                    .code,
                DecisionCode::Unauthenticated
            );
        }
    }
    #[test]
    fn destructive_previews_never_derive_consent_from_core_admin_role() {
        let app = AppSlug::parse("third_party_app").unwrap();
        let mut policy = Policy::default();
        policy.grant(
            "owner123",
            &app,
            &[Action::Install, Action::Update, Action::Stop],
        );
        for op in [
            Operation::Install,
            Operation::UpdateWithBackup,
            Operation::Stop,
        ] {
            let req = prepare(&app, op).unwrap();
            assert_eq!(
                preview_core_authorization(&core(), &claim("owner123"), 101, &policy, &req)
                    .unwrap()
                    .code,
                DecisionCode::AcknowledgementRequired
            );
        }
        let request = prepare(&app, Operation::Install).unwrap();
        assert_eq!(
            preview_core_authorization(&core(), &claim("owner123"), 115, &policy, &request),
            Err(CoreUserError::Stale)
        );
        assert_eq!(
            preview_core_authorization(&core(), &claim("owner123"), 99, &policy, &request),
            Err(CoreUserError::Stale)
        );
    }
}
