//! P3b, strict home-assistant YAML -> actual native Moli graph (synthetic ONLY).
use crate::automation_yaml::{Plan, parse_plan};
use anyhow::{Result, bail};
use moli_automation::{
    Author, Automation, Automations, Config, Edge, Graph, Mode, Node, Op, Rule, Step,
};
use moli_core::{InstanceId, Origin, PointId, Value};
use moli_runtime::{Hub, HubOptions, spawn_driver};
use serde::Serialize;
use serde_json::json;
use std::{path::Path, sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;

pub const SYNTHETIC_YAML: &str = r#"
id: "lab_import"
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

/// Map only one explicitly known fixture to the existing Moli fake driver.
/// There is intentionally no configurable device resolver: unknown device IDs
/// or service intentions must fail before an executable graph exists.
pub fn compile(plan: &Plan) -> Result<Graph> {
    if plan.source_entity != "binary_sensor.lab_motion"
        || plan.proposed_target != "light.lab_room"
        || plan.proposed_service != "light.turn_on"
        || plan.from.as_deref() != Some("off")
        || plan.to != "on"
        || plan.condition_entity.as_deref() != Some("light.lab_room")
        || plan.condition_state.as_deref() != Some("off")
    {
        bail!("HA YAML P3b requires exact synthetic fixture with no guessed mappings");
    }
    let graph = Graph {
        nodes: vec![
            Node {
                id: "trigger".into(),
                step: Step::WhenState {
                    point: "hars-lab:sensor/on".into(),
                    to: Some(json!(true)),
                    from: Some(json!(false)),
                    for_s: 0,
                },
                x: 0.0,
                y: 0.0,
            },
            Node {
                id: "condition".into(),
                step: Step::If {
                    rules: vec![Rule::State {
                        point: "hars-lab:actuator/on".into(),
                        op: Op::Eq,
                        value: json!(false),
                    }],
                    all: true,
                },
                x: 1.0,
                y: 0.0,
            },
            Node {
                id: "action".into(),
                step: Step::Set {
                    point: "hars-lab:actuator/on".into(),
                    value: json!(true),
                },
                x: 2.0,
                y: 0.0,
            },
        ],
        edges: vec![
            Edge {
                from: "trigger".into(),
                port: "out".into(),
                to: "condition".into(),
            },
            Edge {
                from: "condition".into(),
                port: "yes".into(),
                to: "action".into(),
            },
        ],
    };
    // Round-trip confirms the typed graph remains consumable by upstream Moli.
    let encoded = serde_json::to_vec(&graph)?;
    let decoded: Graph = serde_json::from_slice(&encoded)?;
    if graph != decoded {
        bail!("graph serialization changed the compiled automation");
    }
    Ok(graph)
}

#[derive(Debug, Serialize)]
pub struct Proof {
    pub compiled_from_ha_yaml: bool,
    pub actual_native_moli_automation: bool,
    pub required_human_approval: bool,
    pub initially_inactive: bool,
    pub synthetic_actuator_confirmed: bool,
    pub approval_invalidated_by_edit: bool,
    pub no_python: bool,
    pub hardware_touched: bool,
    pub production_ready: bool,
}

/// One executable Rust-only test with actual upstream Moli automation engine
/// and two invented devices; NO real service/broker/network/HAOS.
pub async fn prove(folder: &Path) -> Result<Proof> {
    let plan = parse_plan(SYNTHETIC_YAML)?;
    prove_plan(folder, &plan).await
}

/// Reuse the exact existing Moli real synthetic execution/approval proof
/// with a parsed, SHA-bound source emitted by genuine Supervisor backups.
pub async fn prove_plan(folder: &Path, plan: &Plan) -> Result<Proof> {
    let graph = compile(plan)?;
    let hub = Hub::new(HubOptions::default())?;
    let cancel = CancellationToken::new();
    let driver = spawn_driver(
        &hub,
        InstanceId::from("hars-lab"),
        Arc::new(crate::Synthetic),
        cancel.clone(),
    );
    crate::ready(&hub).await?;
    let (autos, engine) =
        Automations::start(&hub, Config::default(), Some(folder), cancel.clone()).await?;
    let candidate = Automation {
        id: String::new(),
        name: "Imported HA YAML synthetic proof".into(),
        enabled: false,
        mode: Mode::Single,
        graph,
        author: Author::Import,
        approved: None,
        approved_version: None,
        note: Some("Lab proof only; do not import into a real household".into()),
        created: 0,
        updated: 0,
    };
    let saved = autos.save(candidate, crate::human()).await?;
    if saved.is_live() {
        bail!("unapproved imported YAML became active");
    }
    let approved = autos
        .approve(&saved.id, &saved.fingerprint(), crate::human())
        .await?;
    if !approved.is_live() {
        bail!("explicit lab approval did not activate synthetic automation");
    }
    let mut changed = approved.clone();
    let action = changed
        .graph
        .nodes
        .iter_mut()
        .find(|node| node.id == "action")
        .ok_or_else(|| anyhow::anyhow!("compiled graph missing action"))?;
    action.step = Step::Set {
        point: "hars-lab:actuator/on".into(),
        value: json!(false),
    };
    if changed.is_approved() {
        bail!("a mutated imported action retained previous approval");
    }
    tokio::time::sleep(Duration::from_millis(70)).await;
    hub.command(
        &PointId::from("hars-lab:sensor/on"),
        Value::Bool(true),
        Origin::Ui,
        None,
    )
    .await?;
    crate::wait_for(&hub, "hars-lab:actuator/on", true).await?;
    cancel.cancel();
    let _ = tokio::time::timeout(Duration::from_secs(3), driver).await;
    let _ = tokio::time::timeout(Duration::from_secs(3), engine).await;
    drop(autos);
    Ok(Proof {
        compiled_from_ha_yaml: true,
        actual_native_moli_automation: true,
        required_human_approval: true,
        initially_inactive: true,
        synthetic_actuator_confirmed: true,
        approval_invalidated_by_edit: true,
        no_python: true,
        hardware_touched: false,
        production_ready: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automation_yaml::parse_plan;
    use moli_automation::{Rule, Step};
    const YAML: &str = r#"
id: "lab_import"
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
    #[test]
    fn yaml_compiles_to_conditioned_synthetic_graph() {
        let plan = parse_plan(YAML).unwrap();
        let graph = compile(&plan).unwrap();
        assert_eq!(graph.nodes.len(), 3);
        assert_eq!(graph.edges.len(), 2);
        let trigger = graph.node("trigger").unwrap();
        assert!(matches!(trigger.step, Step::WhenState { .. }));
        let condition = graph.node("condition").unwrap();
        assert!(
            matches!(&condition.step, Step::If { rules, .. } if matches!(rules[0], Rule::State { .. }))
        );
        assert_eq!(graph.next("condition", "yes")[0].id, "action");
        assert!(graph.next("condition", "no").is_empty());
        assert!(matches!(
            graph.node("action").unwrap().step,
            Step::Set { .. }
        ));
    }
    #[test]
    fn nonmapped_lab_entities_or_actions_refused_not_guessed() {
        for invalid in [
            YAML.replace("binary_sensor.lab_motion", "binary_sensor.lab_different"),
            YAML.replace("light.lab_room", "light.lab_other"),
            YAML.replace("light.turn_on", "light.turn_off"),
            YAML.replace("to: \"on\"", "to: \"off\""),
        ] {
            assert!(compile(&parse_plan(&invalid).unwrap()).is_err());
        }
    }
}
