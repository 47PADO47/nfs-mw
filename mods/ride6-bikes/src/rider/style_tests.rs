use serde_json::json;

use super::*;

#[test]
fn an_input_handler_class_gives_the_type_the_blueprint_would_use() {
    let name = |handler: &str| from_handler(handler).map(|set| set.name);
    assert_eq!(name("Default_InputHandler"), Some("race"));
    assert_eq!(name("2502T_InputHandler"), Some("race"));
    assert_eq!(name("Naked_InputHandler"), Some("naked"));
    assert_eq!(name("Baggers_InputHandler_C"), Some("naked"));
    assert_eq!(name("Motard_InputHandler"), Some("motard"));
    assert_eq!(name("Maxienduro_InputHandler"), Some("enduro"));
    assert_eq!(name("Scooter_InputHandler"), Some("scooter"));
    assert_eq!(name("ScooterGear_InputHandler"), Some("scooter"));
    assert_eq!(name("Test_InputHandler"), None);
}

#[test]
fn a_row_of_the_correction_table_overrides_the_handler() {
    let chosen =
        |over: Option<&str>, handler: Option<&str>| choose(over, handler).map(|(set, source)| (set.name, source));
    // The Suzuki GSX-S 1000 RM: a naked-bike handler in a sport-bike category, STREET override.
    assert_eq!(
        chosen(Some("EBikeTypeOverride::STREET"), Some("Default_InputHandler")),
        Some(("naked", Source::Override))
    );
    assert_eq!(
        chosen(Some("EBikeTypeOverride::NOOVERRIDE"), Some("Motard_InputHandler")),
        Some(("motard", Source::Handler))
    );
    // The Honda CRF 450R SMT: a motard handler, ENDURO override.
    assert_eq!(chosen(Some("ENDURO"), Some("Motard_InputHandler")), Some(("enduro", Source::Override)));
    assert_eq!(chosen(None, Some("Default_InputHandler")), Some(("race", Source::Handler)));
    assert_eq!(chosen(None, None), None);
    assert_eq!(chosen(Some("NOOVERRIDE"), None), None);
}

#[test]
fn the_handler_is_read_from_the_vehicle_elements_of_the_physics_file() {
    let vehicle = json!({
        "Properties": { "VehicleElements": [
            {"ObjectName": "x", "ObjectPath": "/PhysicsRide6/Physics/VehicleElements/ABS/Motard_ABSVehicleElement.0"},
            {"ObjectName": "y", "ObjectPath": "/PhysicsRide6/Physics/VehicleElements/InputHandler/Motard_InputHandler.0"}
        ]}
    });
    assert_eq!(input_handler(&vehicle).as_deref(), Some("Motard_InputHandler"));
    assert_eq!(input_handler(&json!({"Properties": {"VehicleElements": []}})), None);
    assert_eq!(input_handler(&json!({})), None);
}

#[test]
fn the_table_lists_each_set_with_its_graph_and_its_walk() {
    let race = entry(&graph_sets::RACE, 0, Some(2.2));
    assert_eq!(race["graph"], "graph_race.json");
    assert_eq!(race["bike_type"], 0);
    assert_eq!(race["reverse"]["loop_seconds"], json!(2.2f32));
    assert_eq!(race["reverse"]["rate"], json!([1.0, 4.0]));
    assert!(entry(&graph_sets::NAKED, 1, None).get("reverse").is_none());
    let text = table(&[(&graph_sets::RACE, race), (&graph_sets::NAKED, entry(&graph_sets::NAKED, 0, None))]);
    assert_eq!(text["version"], 1);
    assert_eq!(text["styles"]["naked"]["graph"], "graph_naked.json");
}
