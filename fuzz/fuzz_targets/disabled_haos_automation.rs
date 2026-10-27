#![no_main]
// Fuzz real Rusty Home P8bi disabled HAOS YAML source and Moli compiler.
// This never runs a Hub, service, device, network connection, or token.
use hars_native_kernel::{
    automation_moli::compile,
    automation_yaml::{parse_disabled_backup_plan, parse_disabled_backup_plans, simulate},
};
use libfuzzer_sys::fuzz_target;
fuzz_target!(|raw: &[u8]| {
    if raw.len() > 8192 { return; }
    let Ok(yaml) = std::str::from_utf8(raw) else { return; };
    // New P8cj: instrument the ENTIRE bounded disabled-only batch parser,
    // including hostile mixed lists and duplicate/unsafe member refusal.
    // No activation, approvals, device calls or Hub is instantiated.
    if let Ok(plans) = parse_disabled_backup_plans(yaml) {
        assert!(!plans.is_empty() && plans.len() <= 8);
        for plan in &plans {
            assert!(plan.source_entity.starts_with("binary_sensor.lab_"));
            assert!(plan.proposed_target.starts_with("light.lab_"));
            let preview = simulate(plan, &plan.source_entity, "off", "on", "off");
            assert!(preview.read_only && !preview.production_ready);
        }
    }
    if let Ok(plan) = parse_disabled_backup_plan(yaml) {
        if let Ok(graph) = compile(&plan) {
            assert_eq!(plan.source_entity, "binary_sensor.lab_motion");
            assert_eq!(plan.proposed_service, "light.turn_on");
            assert_eq!(plan.proposed_target, "light.lab_room");
            assert_eq!(graph.nodes.len(), 3);
            assert_eq!(graph.edges.len(), 2);
        }
    }
});
