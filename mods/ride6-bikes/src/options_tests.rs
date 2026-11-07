use super::*;

pub const CATALOGUE: &str = r#"{
 "schema": 1,
 "assets": {
  "exmanifold162_974_001": {"kind": "header", "meshes": {"body": "x"}},
  "muffler974_001": {"kind": "muffler", "meshes": {"body": "x"}},
  "rim738_001": {"kind": "rim", "meshes": {"front": "x", "back": "y"}},
  "brake700_001": {"kind": "brake", "meshes": {"front": "x", "back": "y"}},
  "barpad701_001": {"kind": "barpad", "meshes": {"body": "x"}},
  "lamp1": {"kind": "lamp", "meshes": {}}
 },
 "bikes": {
  "Ducati_Panigale-V4-S-2025": {
   "exhaust": [{"id": "MUF_Akrapovic_Completo_Standard", "header": "exmanifold162_974_001", "muffler": "muffler974_001",
     "anchors": {"header": {"bone": "BONE_CHASSIS", "translation": [0, 0.5, 0], "rotation": [0, 0, 0, 1]},
                 "muffler": {"bone": "BONE_MUFFLER_001", "translation": [0, 0, 0.1], "rotation": [0, 0, 0, 1]}}}],
   "rims": [{"id": "RI_OZ_Piega-R_Livery-01", "asset": "rim738_001",
     "anchors": {"front": {"bone": "BONE_F_VIS_WHEEL", "translation": [1, 0, 0], "rotation": [0, 0, 0, 1]},
                 "back": {"bone": "BONE_B_VIS_WHEEL", "translation": [-1, 0, 0], "rotation": [0, 0, 0, 1]}}}],
   "brakes": [{"level": 1, "asset": "brake700_001",
     "anchors": {"front": {"bone": "BONE_FL_BRAKE", "translation": [1, 0, 0], "rotation": [0, 0, 0, 1]},
                 "back": {"bone": "BONE_B_BRAKE", "translation": [-1, 0, 0], "rotation": [0, 0, 0, 1]}}}],
   "barpad": [],
   "upgrade": {"tables": ["TuningComponentData_Upgrade_1000cc_4T"], "base": {}, "full": {"peak_torque": 1.1, "mass": 0.97}}
  },
  "Plain_Bike-2000": {"exhaust": [], "rims": [], "brakes": [], "barpad": [], "upgrade": null}
 }
}"#;

#[test]
fn the_catalogue_is_read_with_its_anchors() {
    let catalogue = Catalogue::parse(CATALOGUE).unwrap();
    assert_eq!(catalogue.assets.len(), 5, "an asset of an unknown kind is left out");
    assert_eq!(catalogue.assets["rim738_001"].kind, Kind::Rim);
    let bike = catalogue.bike("Ducati_Panigale-V4-S-2025").unwrap();
    assert_eq!(bike.exhaust[0].id, "MUF_Akrapovic_Completo_Standard");
    assert_eq!(bike.exhaust[0].header_anchor.bone.as_deref(), Some("BONE_CHASSIS"));
    assert_eq!(bike.exhaust[0].muffler_anchor.translation, [0.0, 0.0, 0.1]);
    assert_eq!(bike.rims[0].front.translation, [1.0, 0.0, 0.0]);
    assert_eq!(bike.brakes[0].level, 1);
    assert!(bike.pads.is_empty() && bike.has_parts());
}

#[test]
fn an_upgrade_that_changes_nothing_is_not_one() {
    let catalogue = Catalogue::parse(CATALOGUE).unwrap();
    let upgrade = catalogue.bike("Ducati_Panigale-V4-S-2025").unwrap().upgrade.unwrap();
    assert_eq!(upgrade, Upgrade { peak_torque: 1.1, mass: 0.97, brake_force: 1.0 });
    assert!(upgrade.is_any());
    assert!(!Upgrade { peak_torque: 1.0, mass: 1.0, brake_force: 1.0 }.is_any());
    let plain = catalogue.bike("Plain_Bike-2000").unwrap();
    assert!(plain.upgrade.is_none() && !plain.has_parts());
    assert!(catalogue.bike("Unknown").is_none());
}

#[test]
fn the_assets_a_set_of_bikes_uses_are_listed_once() {
    let catalogue = Catalogue::parse(CATALOGUE).unwrap();
    let used = catalogue.used_by(["Ducati_Panigale-V4-S-2025", "Plain_Bike-2000", "Unknown"].into_iter());
    assert_eq!(used, ["brake700_001", "exmanifold162_974_001", "muffler974_001", "rim738_001"]);
}

#[test]
fn a_wrong_schema_is_refused() {
    assert!(Catalogue::parse(r#"{"schema": 2}"#).is_err());
    assert!(Catalogue::parse("nope").is_err());
}
