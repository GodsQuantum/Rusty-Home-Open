//! Read-only Home Assistant YAML automations subset. No live services, ever.
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct Plan {
    pub source_entity: String,
    pub from: Option<String>,
    pub to: String,
    pub condition_entity: Option<String>,
    pub condition_state: Option<String>,
    pub proposed_service: String,
    pub proposed_target: String,
}
#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct Simulation {
    pub triggered: bool,
    pub proposed_action: Option<String>,
    pub read_only: bool,
    pub production_ready: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawAutomation {
    id: Option<String>,
    alias: Option<String>,
    mode: Option<String>,
    triggers: Option<Vec<RawTrigger>>,
    trigger: Option<Vec<RawTrigger>>,
    conditions: Option<Vec<RawCondition>>,
    condition: Option<Vec<RawCondition>>,
    actions: Option<Vec<RawAction>>,
    action: Option<Vec<RawAction>>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DisabledBackupAutomation {
    id: Option<String>,
    alias: Option<String>,
    mode: Option<String>,
    initial_state: Option<bool>,
    triggers: Option<Vec<RawTrigger>>,
    trigger: Option<Vec<RawTrigger>>,
    conditions: Option<Vec<RawCondition>>,
    condition: Option<Vec<RawCondition>>,
    actions: Option<Vec<RawAction>>,
    action: Option<Vec<RawAction>>,
}
impl DisabledBackupAutomation {
    fn into_raw(self) -> Result<RawAutomation> {
        if self.initial_state != Some(false) {
            bail!("HAOS source automation is not explicitly disabled")
        }
        Ok(RawAutomation {
            id: self.id,
            alias: self.alias,
            mode: self.mode,
            triggers: self.triggers,
            trigger: self.trigger,
            conditions: self.conditions,
            condition: self.condition,
            actions: self.actions,
            action: self.action,
        })
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTrigger {
    trigger: Option<String>,
    platform: Option<String>,
    entity_id: String,
    from: Option<String>,
    to: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCondition {
    condition: String,
    entity_id: String,
    state: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawAction {
    action: Option<String>,
    service: Option<String>,
    target: RawTarget,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTarget {
    entity_id: String,
}

fn lab_entity(value: &str, domain: &str) -> bool {
    value
        .strip_prefix(domain)
        .and_then(|tail| tail.strip_prefix(".lab_"))
        .is_some_and(|tail| {
            !tail.is_empty()
                && tail.len() <= 64
                && tail
                    .bytes()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_')
        })
}
/// Compile a bounded, immutable **non-executable** HA YAML subset.
/// Duplicate YAML keys and unsupported metadata must fail before preview.
fn check_yaml(yaml: &str) -> Result<()> {
    if yaml.is_empty()
        || yaml.len() > 8192
        || yaml.chars().any(|c| c == '\0')
        || ["{{", "{%", "!include", "!!", "!<", "<<:", "&", "*"]
            .iter()
            .any(|part| yaml.contains(part))
    {
        bail!("unsafe or oversized YAML automation");
    }
    Ok(())
}

/// A full real HAOS backup uses a LIST of automations. This deliberately
/// accepts only ONE explicitly disabled synthetic recipe. Imported YAML is
/// NEVER activated as a real Home Assistant or household service.
pub fn parse_disabled_backup_plan(yaml: &str) -> Result<Plan> {
    check_yaml(yaml)?;
    let mut list: Vec<DisabledBackupAutomation> = serde_saphyr::from_str_with_options(
        yaml,
        serde_saphyr::options! { reject_unsupported_tags: true },
    )?;
    if list.len() != 1 {
        bail!("only one disabled synthetic automation may be bridged to Moli")
    }
    plan_from_raw(list.pop().expect("length checked").into_raw()?)
}

pub fn parse_plan(yaml: &str) -> Result<Plan> {
    check_yaml(yaml)?;
    let raw: RawAutomation = serde_saphyr::from_str_with_options(
        yaml,
        serde_saphyr::options! { reject_unsupported_tags: true },
    )?;
    plan_from_raw(raw)
}

fn plan_from_raw(mut raw: RawAutomation) -> Result<Plan> {
    if raw.triggers.is_some() && raw.trigger.is_some()
        || raw.conditions.is_some() && raw.condition.is_some()
        || raw.actions.is_some() && raw.action.is_some()
    {
        bail!("conflicting modern/legacy automation keys");
    }
    if raw.mode.as_deref().unwrap_or("single") != "single" {
        bail!("non-single HA automation modes require separate run semantics");
    }
    if raw
        .id
        .as_ref()
        .is_some_and(|id| id.is_empty() || id.len() > 128)
        || raw.alias.as_ref().is_some_and(|alias| alias.len() > 128)
    {
        bail!("oversized automation metadata");
    }
    let mut triggers = raw
        .triggers
        .take()
        .or(raw.trigger.take())
        .ok_or_else(|| anyhow::anyhow!("missing HA triggers"))?;
    let mut actions = raw
        .actions
        .take()
        .or(raw.action.take())
        .ok_or_else(|| anyhow::anyhow!("missing HA actions"))?;
    let mut conditions = raw
        .conditions
        .take()
        .or(raw.condition.take())
        .unwrap_or_default();
    if triggers.len() != 1 || actions.len() != 1 || conditions.len() > 1 {
        bail!("P3a subset requires one trigger/action and at most one state condition");
    }
    let t = triggers.pop().expect("length already checked");
    if t.trigger.is_some() && t.platform.is_some()
        || t.trigger.or(t.platform).as_deref() != Some("state")
        || !lab_entity(&t.entity_id, "binary_sensor")
    {
        bail!("P3a accepts only a synthetic binary_sensor state trigger");
    }
    let to =
        t.to.ok_or_else(|| anyhow::anyhow!("state trigger needs to"))?;
    if !matches!(to.as_str(), "on" | "off")
        || t.from
            .as_ref()
            .is_some_and(|s| !matches!(s.as_str(), "on" | "off"))
    {
        bail!("P3a requires a literal on/off state transition");
    }
    let c = conditions.pop();
    if c.as_ref().is_some_and(|c| {
        c.condition != "state"
            || !lab_entity(&c.entity_id, "light")
            || !matches!(c.state.as_str(), "on" | "off")
    }) {
        bail!("unsupported or nonlab HA condition");
    }
    let a = actions.pop().expect("length already checked");
    if a.action.is_some() && a.service.is_some() || !lab_entity(&a.target.entity_id, "light") {
        bail!("non-lab action target or conflicting action keys");
    }
    let service = a
        .action
        .or(a.service)
        .ok_or_else(|| anyhow::anyhow!("missing HA action"))?;
    if !matches!(service.as_str(), "light.turn_on" | "light.turn_off") {
        bail!("P3a supports only simulated lighting service intents");
    }
    Ok(Plan {
        source_entity: t.entity_id,
        from: t.from,
        to,
        condition_entity: c.as_ref().map(|c| c.entity_id.clone()),
        condition_state: c.map(|c| c.state),
        proposed_service: service,
        proposed_target: a.target.entity_id,
    })
}
/// Pure simulation: returning a string must never invoke a Moli service.
pub fn simulate(
    plan: &Plan,
    event_entity: &str,
    previous: &str,
    next: &str,
    condition_state: &str,
) -> Simulation {
    let triggered = event_entity == plan.source_entity
        && previous != next
        && next == plan.to
        && plan.from.as_deref().is_none_or(|from| from == previous)
        && plan
            .condition_state
            .as_deref()
            .is_none_or(|state| state == condition_state);
    Simulation {
        triggered,
        proposed_action: triggered
            .then(|| format!("{}:{}", plan.proposed_service, plan.proposed_target)),
        read_only: true,
        production_ready: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const MODERN: &str = r#"
id: "lab_motion_light"
alias: "Lab: synthetic motion"
mode: single
triggers:
  - trigger: state
    entity_id: binary_sensor.lab_motion
    from: "off"
    to: "on"
conditions:
  - condition: state
    entity_id: light.lab_room
    state: "off"
actions:
  - action: light.turn_on
    target:
      entity_id: light.lab_room
"#;
    const LEGACY: &str = r#"
trigger:
  - platform: state
    entity_id: binary_sensor.lab_motion
    from: "off"
    to: "on"
condition:
  - condition: state
    entity_id: light.lab_room
    state: "off"
action:
  - service: light.turn_on
    target:
      entity_id: light.lab_room
"#;
    #[test]
    fn official_modern_and_legacy_compile_to_identical_lab_plan() {
        let first = parse_plan(MODERN).unwrap();
        let second = parse_plan(LEGACY).unwrap();
        assert_eq!(first, second);
        let actual = simulate(&first, "binary_sensor.lab_motion", "off", "on", "off");
        assert!(actual.triggered);
        assert_eq!(
            actual.proposed_action.as_deref(),
            Some("light.turn_on:light.lab_room")
        );
        assert!(actual.read_only && !actual.production_ready);
    }
    #[test]
    fn only_real_state_transitions_qualify_and_condition_blocks() {
        let plan = parse_plan(MODERN).unwrap();
        for (prev, next, condition) in [
            ("on", "on", "off"),
            ("off", "off", "off"),
            ("off", "on", "on"),
            ("unknown", "on", "off"),
        ] {
            let simulation = simulate(&plan, "binary_sensor.lab_motion", prev, next, condition);
            assert!(!simulation.triggered && simulation.proposed_action.is_none());
        }
        let spoofed = simulate(&plan, "binary_sensor.lab_intruder", "off", "on", "off");
        assert!(!spoofed.triggered && spoofed.proposed_action.is_none());
    }
    #[test]
    fn disabled_actual_haos_list_can_only_compile_known_synthetic_mapping() {
        let source = include_str!("../../../scripts/fixtures/p8be_haos_automations.yaml");
        let plan = parse_disabled_backup_plan(source).unwrap();
        assert_eq!(plan.source_entity, "binary_sensor.lab_motion");
        assert_eq!(plan.proposed_target, "light.lab_room");
        assert_eq!(plan.proposed_service, "light.turn_on");
        let graph = crate::automation_moli::compile(&plan).unwrap();
        assert_eq!(graph.nodes.len(), 3);
        for invalid in [
            source.replace("initial_state: false", "initial_state: true"),
            source.replace("  initial_state: false\n", ""),
            source.replace("light.lab_room", "light.real_house"),
            source.replace("light.turn_on", "shell_command.attack"),
            source.replace("initial_state: false", "initial_state: !!bool false"),
            source.replace("triggers:", "triggers: []\n  dummy:"),
            format!("{source}\n{source}"),
            "[]".to_string(),
            source.replace("light.lab_room", "{{ dangerous }}"),
        ] {
            assert!(
                parse_disabled_backup_plan(&invalid).is_err(),
                "unsafe or out-of-scope HAOS YAML accepted: {invalid}"
            );
        }
    }
    #[test]
    fn malformed_or_nonlab_automations_rejected_before_execution() {
        for yaml in [
            MODERN.replace("mode: single", "mode: parallel"),
            MODERN.replace("light.lab_room", "light.real"),
            MODERN.replace("trigger: state", "trigger: time"),
            MODERN.replace("light.turn_on", "shell_command.attack"),
            MODERN.replace("off", "{{ states('sensor.foo') }}"),
            format!("{MODERN}\ntrigger: []"),
            format!("{MODERN}\nunknown_key: 1"),
            format!("{MODERN}\nmode: single"),
            format!("{MODERN}\ntriggers: []"),
            format!("{MODERN}\nservice: script.turn_on"),
            format!("{MODERN}\n# anchor\nalias: &evil ignore"),
        ] {
            assert!(parse_plan(&yaml).is_err(), "unsafe YAML accepted: {yaml}");
        }
        assert!(parse_plan(&" ".repeat(8193)).is_err());
    }
}
