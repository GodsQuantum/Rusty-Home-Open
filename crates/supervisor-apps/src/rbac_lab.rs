//! Pure, LAB-ONLY access-policy evaluator. Not an authentication provider.
//! Caller identities/roles and acknowledgements MUST be established by a future
//! authenticated, replay-protected gateway, not copied from HTTP request JSON.
//! Deliberately disconnected from Supervisor transport and default binaries.

pub use crate::managed_apps::Action;
use crate::managed_apps::{AppSlug, PreparedRequest};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    Administrator,
    AppOperator,
    Household,
    ReadOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Actor {
    subject: String,
    role: Role,
}

impl Actor {
    pub fn new(subject: &str, role: Role) -> Result<Self, &'static str> {
        if !valid_subject(subject) {
            return Err("invalid actor identifier");
        }
        Ok(Self {
            subject: subject.to_string(),
            role,
        })
    }
    /// Read-only subject for binding a durable fixture session to Core's
    /// independently checked account. Never a role/authority by itself.
    pub fn subject(&self) -> &str {
        &self.subject
    }
    pub fn role(&self) -> Role {
        self.role
    }
}

fn valid_subject(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .next()
            .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        && value
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_' || c == b'-')
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionCode {
    Allowed,
    Unauthenticated,
    RoleDenied,
    UnscopedApp,
    UnscopedOperation,
    AcknowledgementRequired,
    AcknowledgementMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditRecord {
    pub subject: String,
    pub app: String,
    pub action: Action,
    pub code: DecisionCode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    pub code: DecisionCode,
    pub audit: AuditRecord,
}

impl Decision {
    pub fn allowed(&self) -> bool {
        self.code == DecisionCode::Allowed
    }
}

/// A correlation of actor and *complete* typed operation parameters.
/// LAB ONLY: NOT a signed user consent, not replay protection or CSRF defence.
#[derive(Clone)]
pub struct Acknowledgement {
    subject: String,
    request: PreparedRequest,
}

impl Acknowledgement {
    pub fn bind(actor: &Actor, request: &PreparedRequest) -> Self {
        Self {
            subject: actor.subject.clone(),
            request: request.clone(),
        }
    }

    fn matches(&self, actor: &Actor, request: &PreparedRequest) -> bool {
        self.subject == actor.subject && self.request == *request
    }
}

/// In-memory fixture entitlements; NOT durable user accounts or role store.
/// No wildcard App grants. Future backend must get these from trusted state.
#[derive(Default)]
pub struct Policy {
    grants: BTreeMap<(String, String), BTreeSet<Action>>,
}

impl Policy {
    pub fn grant(&mut self, subject: &str, app: &AppSlug, actions: &[Action]) {
        if !valid_subject(subject) {
            return;
        }
        self.grants
            .entry((subject.to_string(), app.as_str().to_string()))
            .or_default()
            .extend(actions.iter().copied());
    }

    pub fn evaluate(
        &self,
        actor: Option<&Actor>,
        request: &PreparedRequest,
        acknowledgement: Option<&Acknowledgement>,
    ) -> Decision {
        let action = request.action();
        let code = match actor {
            None => DecisionCode::Unauthenticated,
            Some(actor) => {
                if !role_can(actor.role, action) {
                    DecisionCode::RoleDenied
                } else if let Some(granted_actions) = self
                    .grants
                    .get(&(actor.subject.clone(), request.target().to_string()))
                {
                    if !granted_actions.contains(&action) {
                        DecisionCode::UnscopedOperation
                    } else if requires_acknowledgement(action) {
                        match acknowledgement {
                            None => DecisionCode::AcknowledgementRequired,
                            Some(a) if !a.matches(actor, request) => {
                                DecisionCode::AcknowledgementMismatch
                            }
                            Some(_) => DecisionCode::Allowed,
                        }
                    } else {
                        DecisionCode::Allowed
                    }
                } else {
                    DecisionCode::UnscopedApp
                }
            }
        };
        Decision {
            code,
            audit: AuditRecord {
                subject: actor.map_or_else(|| "anonymous".to_string(), |a| a.subject.clone()),
                app: request.target().to_string(),
                action,
                code,
            },
        }
    }
}

fn role_can(role: Role, action: Action) -> bool {
    match role {
        Role::Administrator => true,
        Role::AppOperator => matches!(
            action,
            Action::Start | Action::Stop | Action::Restart | Action::Backup
        ),
        Role::Household | Role::ReadOnly => false,
    }
}

fn requires_acknowledgement(action: Action) -> bool {
    matches!(
        action,
        Action::Install | Action::Stop | Action::Update | Action::Restore
    )
}
