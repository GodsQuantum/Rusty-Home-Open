#![no_main]
// Fuzz real Rusty Home P8bi disabled HAOS YAML source and Moli compiler.
// This never runs a Hub, service, device, network connection, or token.
use hars_native_kernel::{
    automation_moli::compile,
    automation_yaml::parse_disabled_backup_plan,
};
use libfuzzer_sys::fuzz_target;
fuzz_target!(|raw: &[u8]| {
    if raw.len() > 8192 { return; }
    let Ok(yaml) = std::str::from_utf8(raw) else { return; };
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
