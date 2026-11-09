use super::*;
use crate::content::{ContentError, parse_car};

const BIKE: &str = r#"
[[slot]]
id = "exhaust"
group = "looks"
accepts = ["r6:exhaust.stock", { component = "r6:exhaust.race", price = 5 }]
default = "r6:exhaust.stock"

[parts."exhaust.muffler.stock"]
solid = "FASTBIKE_KIT00_MUFFLER"

[parts."exhaust.muffler.race"]
solid = "MUF_974_001"
pack = "PARTS/R6/MUF974_001"
role = "wheel_front"
matrix = [1, 0, 0, 0, 0, 1, 0, 0.1, 0, 0, 1, 0]
"#;

/// The tables the loader reads.
fn tables(text: &str) -> Result<Option<Components>, ContentError> {
    let raw: toml::Table = toml::from_str(text).unwrap();
    let slots = raw.get("slot").map(|value| value.clone().try_into().unwrap());
    let parts = raw.get("parts").map(|value| value.clone().try_into().unwrap());
    check(slots, parts)
}

#[test]
fn a_car_without_the_tables_has_no_components() {
    assert_eq!(tables("").unwrap(), None);
}

#[test]
fn the_slots_and_the_parts_are_read_in_order() {
    let found = tables(BIKE).unwrap().unwrap();
    assert_eq!(found.slots.len(), 1);
    assert_eq!(found.slots[0].id, "exhaust");
    assert_eq!(found.slots[0].accepts[1].price, Some(5));
    let stock = &found.parts["exhaust.muffler.stock"];
    assert_eq!(
        stock,
        &PartBinding {
            solid: "FASTBIKE_KIT00_MUFFLER".into(),
            pack: None,
            role: PartRole::Body,
            matrix: None,
            base: false,
            markers: Vec::new()
        }
    );
    let race = &found.parts["exhaust.muffler.race"];
    assert_eq!(race.pack.as_deref(), Some("PARTS/R6/MUF974_001"));
    assert_eq!(race.role, PartRole::WheelFront);
    assert_eq!(race.matrix.unwrap()[7], 0.1);
}

#[test]
fn a_car_manifest_carries_its_components() {
    let car = parse_car("fastbike", BIKE).unwrap();
    assert_eq!(car.components.unwrap().parts.len(), 2);
    assert!(parse_car("fastbike", &BIKE.replace("[[slot]]", "[[nothing]]")).is_err());
    assert_eq!(parse_car("fastbike", "").unwrap().components, None);
}

#[test]
fn a_matrix_has_twelve_finite_numbers() {
    let eleven = BIKE.replace("[1, 0, 0, 0, 0, 1, 0, 0.1, 0, 0, 1, 0]", "[1, 0, 0, 0, 0, 1, 0, 0.1, 0, 0, 1]");
    assert!(matches!(tables(&eleven), Err(ContentError::Field { field: "parts.matrix", .. })));
    let nan = BIKE.replace("0.1, 0, 0, 1, 0]", "nan, 0, 0, 1, 0]");
    assert!(matches!(tables(&nan), Err(ContentError::Field { field: "parts.matrix", .. })));
}

#[test]
fn a_solid_name_is_plain() {
    let bad = BIKE.replace("MUF_974_001", "muf 974");
    assert!(matches!(tables(&bad), Err(ContentError::InvalidModel(_))));
    let pack = BIKE.replace("PARTS/R6/MUF974_001", "PARTS/../R6");
    assert!(matches!(tables(&pack), Err(ContentError::InvalidModel(_))));
}

#[test]
fn an_id_is_lower_case_with_dots() {
    assert!(is_component_id("r6:exhaust.muf_akrapovic-completo"));
    assert!(!is_component_id("R6:Exhaust"));
    assert!(!is_component_id(""));
    assert!(!is_component_id(&"a".repeat(65)));
    let bad = BIKE.replace("id = \"exhaust\"", "id = \"Exhaust\"");
    assert!(matches!(tables(&bad), Err(ContentError::InvalidText(_))));
}

#[test]
fn a_catalog_file_is_read_and_a_repeated_id_refused() {
    let file = r#"
[[component]]
id = "r6:exhaust.stock"
effects = [{ kind = "part", show = ["exhaust.muffler.stock"] }]

[[component]]
id = "r6:engine.1"
tier = 1
effects = [{ kind = "param", key = "bike.engine.peak_torque_nm", op = "mul", value = 1.03 }]
"#;
    let mut catalog = parse_catalog(file).unwrap();
    assert_eq!(catalog.iter().count(), 2);
    assert_eq!(catalog.get("r6:engine.1").unwrap().tier, 1);
    let again = parse_catalog(file).unwrap();
    assert!(matches!(merge_catalog(&mut catalog, again), Err(ContentError::Parse(_))));
    assert!(parse_catalog("[[component]]\nid = \"Bad Id\"\n").is_err());
    assert!(parse_catalog("[[component]]\nid = \"ok\"\nunknown = 1\n").is_err());
}

#[test]
fn a_base_part_is_a_part_the_car_is_drawn_with() {
    let text = format!("{BIKE}\n[parts.\"body.2\"]\nsolid = \"FASTBIKE_KIT00_BODY2\"\nbase = true\n");
    let found = tables(&text).unwrap().unwrap();
    assert!(found.parts["body.2"].base);
    assert!(!found.parts["exhaust.muffler.stock"].base, "a part is not a base part unless it says so");
}

#[test]
fn a_part_carries_tail_pipe_markers_with_a_unit_direction() {
    let text = format!(
        "{BIKE}
[parts.\"exhaust.muffler2.race\"]
solid = \"MUF_974_001_G2\"
markers = [
  {{ name = \"LEFT_EXHAUST\", position = [-0.9, 0.12, 0.8], direction = [-2, 0, 0] }},
  {{ name = \"EXHAUST\", position = [0, 0, 0.3], direction = [0, 0, 1] }},
]
"
    );
    let found = tables(&text).unwrap().unwrap();
    let markers = &found.parts["exhaust.muffler2.race"].markers;
    assert_eq!(markers.len(), 2);
    assert_eq!(
        markers[0],
        PartMarker { name: "LEFT_EXHAUST".into(), position: [-0.9, 0.12, 0.8], direction: [-1.0, 0.0, 0.0] }
    );
    assert!(found.parts["exhaust.muffler.race"].markers.is_empty(), "a part with no list has none");
}

#[test]
fn a_marker_must_be_a_tail_pipe_with_finite_numbers_and_a_direction() {
    let with = |marker: &str| {
        format!(
            "{BIKE}
[parts.\"x.y\"]
solid = \"A\"
markers = [{marker}]
"
        )
    };
    let good = r#"{ name = "EXHAUST", position = [0, 0, 0], direction = [0, 0, 1] }"#;
    assert!(tables(&with(good)).is_ok());
    let field = |text: String| matches!(tables(&text), Err(ContentError::Field { field: "parts.markers", .. }));
    assert!(field(with(r#"{ name = "head light", position = [0, 0, 0], direction = [0, 0, 1] }"#)));
    assert!(field(with(r#"{ name = "", position = [0, 0, 0], direction = [0, 0, 1] }"#)));
    assert!(field(with(r#"{ name = "EXHAUST", position = [nan, 0, 0], direction = [0, 0, 1] }"#)));
    assert!(field(with(r#"{ name = "EXHAUST", position = [0, 0, 0], direction = [0, 0, 0] }"#)));
    assert!(field(with(&[good; 9].join(", "))));
    let no_direction = with(r#"{ name = "EXHAUST", position = [0, 0, 0] }"#);
    assert!(parse_car("fastbike", &no_direction).is_err(), "a direction is required");
}
