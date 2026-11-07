use super::*;

pub const SAMPLE: &str = r#"{
  "schema": 1, "vehicle": "TEST_BIKE", "tyre_sizes": {"front": "120-60-17", "back": "160-60-17"},
  "bones": {"BONE_F_VIS_WHEEL": {"position": [0.7, 0.2, 0.0], "rotation": [0,0,0,1]},
            "BONE_B_VIS_WHEEL": {"position": [-0.7, 0.2, 0.0], "rotation": [0,0,0,1]}},
  "parts": [
    {"id": "chassis", "role": "body", "export": "chassis1", "anchor": {"bone": "BONE_CHASSIS", "translation": [0,0,0], "rotation": [0,0,0,1]},
     "materials": {"mat_a": "/Game/x/mat_a.0"}},
    {"id": "front_tyre", "role": "front_wheel", "export": "f_tyre1", "anchor": {"bone": "BONE_F_VIS_WHEEL", "translation": [0.7,0.2,0], "rotation": [0,0,0,1]},
     "materials": {"mat_t": "/Game/x/Mat_T.0"}}
  ],
  "materials": {
    "mat_a": {"shading_model": "MSM_ClearCoat", "blend_mode": "BLEND_Masked",
              "tex": {"BaseColor - Texture": "/Game/x/livery_d.0", "PBR - Texture": "/Game/x/livery_l.0", "Normal - Texture": "/Engine/T_Default_N.0"},
              "switch": {"LegacyTexturesFormats - PBR - S_R_M_O": true}},
    "mat_t": {"blend_mode": "BLEND_Masked", "clip": 0.3333, "tex": {"BaseColorOpacity - Texture": "/Game/x/tyre_d.0"}, "switch": {"BaseColorOpacity - Source": true}}
  }
}"#;

#[test]
fn the_physics_file_is_looked_up_by_the_assets_name_when_the_table_has_one() {
    let table = PartTable::parse(SAMPLE).unwrap();
    assert_eq!(table.physics_name(), "TEST_BIKE");
    let text = SAMPLE.replacen(
        "\"vehicle\": \"TEST_BIKE\",",
        "\"vehicle\": \"TEST_BIKE\", \"physics_asset\": \"/PhysicsRide6/Physics/V/KTM/KTM_450SMR_2022.KTM_450SMR_2022\",",
        1,
    );
    assert_eq!(PartTable::parse(&text).unwrap().physics_name(), "KTM_450SMR_2022");
}

#[test]
fn a_table_reads_parts_materials_and_bones() {
    let table = PartTable::parse(SAMPLE).unwrap();
    assert_eq!(table.vehicle, "TEST_BIKE");
    assert_eq!(table.parts.len(), 2);
    assert_eq!(table.parts[1].kind, Kind::Tyre);
    assert_eq!(table.parts[1].role, Role::FrontWheel);
    assert_eq!(table.bone("BONE_B_VIS_WHEEL").unwrap(), [-0.7, 0.2, 0.0]);
    assert!(table.bone("NOPE").is_err());
    assert_eq!(table.chassis().unwrap().export, "chassis1");
    assert_eq!(table.tyre_sizes.get("back").map(String::as_str), Some("160-60-17"));
}

#[test]
fn a_material_takes_the_textures_its_switches_name_and_ignores_engine_defaults() {
    let table = PartTable::parse(SAMPLE).unwrap();
    let a = table.material(&table.parts[0], "mat_a").unwrap();
    assert_eq!(a.base_color.as_deref(), Some("livery_d"));
    assert_eq!(a.pbr.as_deref(), Some("livery_l"));
    assert_eq!(a.normal, None, "the engine's default normal map is no data");
    assert!(a.pbr_has_occlusion && !a.opacity_in_base);
    let t = table.material(&table.parts[1], "mat_t").unwrap();
    assert_eq!(t.base_color.as_deref(), Some("tyre_d"));
    assert!(t.opacity_in_base);
    assert_eq!(t.clip, Some(0.3333));
    assert!(table.material(&table.parts[1], "unknown").is_none());
}

#[test]
fn the_steering_rules_are_ride6s_unless_the_file_names_its_own() {
    let table = PartTable::parse(SAMPLE).unwrap();
    assert_eq!(table.front_end, Rules::ride6());
    let own = SAMPLE.replacen("\"schema\": 1,", "\"schema\": 1, \"front_end\": {\"steer_roots\": [\"FORK\"]},", 1);
    let table = PartTable::parse(&own).unwrap();
    assert_eq!(
        table.front_end,
        Rules { steer_roots: vec!["FORK".to_owned()], slide_roots: Vec::new(), slide_followers: Vec::new() }
    );
}

#[test]
fn schemas_other_than_one_are_refused() {
    assert!(PartTable::parse(r#"{"schema": 2}"#).is_err());
    assert!(PartTable::parse("[").is_err());
}

#[test]
fn the_anchor_rotates_then_moves() {
    // A quarter turn about z takes x to y.
    let half = std::f32::consts::FRAC_1_SQRT_2;
    let anchor = Anchor { bone: None, translation: [1.0, 0.0, 0.0], rotation: [0.0, 0.0, half, half] };
    let p = anchor.point([1.0, 0.0, 0.0]);
    assert!((p[0] - 1.0).abs() < 1e-5 && (p[1] - 1.0).abs() < 1e-5 && p[2].abs() < 1e-5, "{p:?}");
    let d = anchor.direction([0.0, 1.0, 0.0]);
    assert!((d[0] + 1.0).abs() < 1e-5 && d[1].abs() < 1e-5, "{d:?}");
}

const LIVERIES: &str = r#"{
  "schema": 1, "vehicle": "TEST_BIKE",
  "parts": [{"id": "chassis", "role": "body", "export": "c", "anchor": {"bone": null, "translation": [0,0,0], "rotation": [0,0,0,1]},
             "materials": {"mat_livery_0_1_001": "/Game/x/mat_livery_0_1_001.0"}}],
  "materials": {
    "mat_livery_0_1_001": {"tex": {"BaseColor - Texture": "/Game/x/liv1_d.0"}},
    "mat_livery_0_1_002": {"tex": {"BaseColor - Texture": "/Game/x/liv2_d.0"}}},
  "liveries": [
    {"index": 2, "materials": {"mat_livery_0_1_001": "/Game/x/mat_livery_0_1_002.0"}},
    {"index": 1, "materials": {"mat_livery_0_1_001": "/Game/x/mat_livery_0_1_001.0"}}]
}"#;

#[test]
fn liveries_are_read_in_order_and_replace_the_chassis_livery_material() {
    let table = PartTable::parse(LIVERIES).unwrap();
    assert_eq!(table.liveries.iter().map(|l| l.index).collect::<Vec<_>>(), vec![1, 2]);
    assert_eq!(table.liveries[1].materials.get("mat_livery_0_1_001").map(String::as_str), Some("mat_livery_0_1_002"));
    assert_eq!(table.livery_material().map(|(name, _)| name), Some("mat_livery_0_1_001".to_owned()));
    let pictures = table.livery_pictures();
    let stems: Vec<_> = pictures.iter().map(|(n, m)| (*n, m.base_color.clone())).collect();
    assert_eq!(stems, vec![(1, Some("liv1_d".to_owned())), (2, Some("liv2_d".to_owned()))]);
}

#[test]
fn a_bike_whose_liveries_share_one_picture_offers_none() {
    let same = LIVERIES.replace("mat_livery_0_1_002.0", "mat_livery_0_1_001.0");
    assert!(PartTable::parse(&same).unwrap().livery_pictures().is_empty());
    let none = LIVERIES.replace("\"liveries\"", "\"others\"");
    assert!(PartTable::parse(&none).unwrap().livery_pictures().is_empty());
}

#[test]
fn a_picture_is_found_under_whichever_parameter_holds_it() {
    let text = SAMPLE.replacen(
        "\"mat_t\": {",
        "\"mat_old\": {\"tex\": {\"BaseColor0_Map\": \"/Game/x/rim_d.0\", \"Pbr0_Map\": \"/Game/x/rim_l.0\"}},
    \"mat_swapped\": {\"tex\": {\"BaseColor - Texture\": \"/Game/x/muf_d.0\"}, \"switch\": {\"BaseColorOpacity - Source\": true}},
    \"mat_t\": {",
        1,
    );
    let table = PartTable::parse(&text).unwrap();
    let rim = &table.materials["mat_old"];
    assert_eq!((rim.base_color.as_deref(), rim.pbr.as_deref()), (Some("rim_d"), Some("rim_l")));
    assert_eq!(table.materials["mat_swapped"].base_color.as_deref(), Some("muf_d"));
}

#[test]
fn a_surface_with_no_instance_takes_the_chassis_instance_of_its_kind() {
    let text = SAMPLE
        .replacen(
            "\"materials\": {\"mat_a\": \"/Game/x/mat_a.0\"}",
            "\"materials\": {\"mat_livery_0_1_001\": \"/Game/x/mat_livery_0_1_001.0\"}",
            1,
        )
        .replacen("\"mat_a\": {\"shading_model\"", "\"mat_livery_0_1_001\": {\"shading_model\"", 1);
    let table = PartTable::parse(&text).unwrap();
    let chassis = table.chassis().unwrap();
    let paint = table.material(chassis, "mat_livery_1").expect("the paint of the far levels");
    assert_eq!(paint.base_color.as_deref(), Some("livery_d"));
    assert!(table.material(chassis, "mat_unknown_kind").is_none());
    let tyre = &table.parts[1];
    assert_eq!(table.material(tyre, "mat_bike_lods711").and_then(|m| m.base_color.as_deref()), Some("tyre_d"));
}

#[test]
fn the_fallbacks_of_a_table_are_kept_for_the_manifest() {
    assert!(PartTable::parse(SAMPLE).unwrap().fallbacks.is_empty());
    let text = SAMPLE.replacen(
        "\"parts\": [",
        "\"fallbacks\": [{\"part\": \"chassis\", \"surface\": \"mat_light1\", \"levels\": [0], \"triangles\": 440, \
         \"rule\": \"family\", \"source\": \"A_B-2008\", \"material\": \"mat_light2\", \"picture\": \"lights2_d\"}], \"parts\": [",
        1,
    );
    let table = PartTable::parse(&text).unwrap();
    assert_eq!(table.fallbacks.len(), 1);
    assert_eq!(table.fallbacks[0].source.as_deref(), Some("A_B-2008"));
}
