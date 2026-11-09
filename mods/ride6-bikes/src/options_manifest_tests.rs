use std::collections::BTreeSet;

use blackbox_components::VehicleDef;
use blackbox_modding::content::{parse_car, parse_catalog};

use super::*;
use crate::build_pieces::PIECES;
use crate::options::{Exhaust, Pad};
use crate::partfile::Anchor;

fn anchor(x: f32) -> Anchor {
    Anchor::rigid(None, [x, 0.0, 0.0], [0.0, 0.0, 0.0, 1.0])
}

fn pair(id: &str, level: u32, asset: &str) -> Pair {
    Pair { id: id.to_owned(), level, asset: asset.to_owned(), front: anchor(1.0), back: anchor(-1.0) }
}

fn options() -> BikeOptions {
    BikeOptions {
        exhaust: vec![Exhaust {
            id: "MUF_Akrapovic_Completo_Standard".to_owned(),
            header: "exmanifold162_974_001".to_owned(),
            muffler: "muffler974_001".to_owned(),
            header_anchor: anchor(0.0),
            muffler_anchor: anchor(0.1),
            muffler_twins: Vec::new(),
        }],
        rims: vec![pair("RI_OZ_Piega-R_Livery-01", 0, "rim738_001")],
        brakes: vec![pair("", 1, "brake700_001")],
        pads: vec![Pad {
            id: "PAD_Renthal_Fatbar-Pad_Blue".to_owned(),
            asset: "barpad701_003".to_owned(),
            anchor: anchor(0.0),
        }],
        upgrade: Some(Upgrade { peak_torque: 1.12, mass: 0.95, brake_force: 1.075 }),
    }
}

fn plan() -> Plan {
    let have: Vec<&Piece> = PIECES.iter().collect();
    Plan::of("ktm450", &options(), &have, Mounts::default())
}

fn part<'a>(plan: &'a Plan, id: &str) -> &'a Part {
    plan.parts.iter().find(|part| part.id == id).unwrap_or_else(|| panic!("no part {id}"))
}

#[test]
fn a_bike_with_everything_has_a_slot_for_each() {
    let plan = plan();
    let ids: Vec<&str> = plan.slots.iter().map(|slot| slot.id).collect();
    assert_eq!(
        ids,
        ["exhaust", "rims", "brake_discs", "bar_pad", "perf_engine", "perf_chassis", "perf_brakes", "road_equipment"]
    );
    let exhaust = &plan.slots[0];
    assert_eq!(exhaust.accepts, ["r6:exhaust.stock", "r6:exhaust.muf_akrapovic_completo_standard"]);
    assert_eq!(exhaust.default, "r6:exhaust.stock");
    assert_eq!(plan.slots[4].group, "performance");
    assert_eq!(plan.slots[4].accepts.len(), 4, "stock and three levels of the engine kit");
    assert_eq!(plan.slots[5].accepts.len(), 3);
}

#[test]
fn the_stock_parts_are_the_solids_of_the_car_and_the_others_the_packs() {
    let plan = plan();
    let stock = part(&plan, "exhaust.stock");
    assert_eq!((stock.solid.as_str(), stock.pack.as_deref(), stock.role), ("KTM450_KIT00_EXHAUST", None, "body"));
    assert!(stock.matrix.is_none());
    let header = part(&plan, "exhaust.header.muf_akrapovic_completo_standard");
    assert_eq!(header.solid, "EXMANIFOLD162_974_001");
    assert_eq!(header.pack.as_deref(), Some("PARTS/R6/EXMANIFOLD162_974_001"));
    assert!(header.matrix.is_some());
    let rim = part(&plan, "rim.front.rim738_001");
    assert_eq!((rim.solid.as_str(), rim.role), ("RIM738_001_FRONT_RIM", "wheel_front"));
    assert_eq!(part(&plan, "rim.back.rim738_001").role, "wheel_rear");
    assert_eq!(part(&plan, "disc.back.brake700_001").solid, "BRAKE700_001_BACK_BRAKE");
    assert_eq!(part(&plan, "barpad.stock").role, "steered");
    assert_eq!(part(&plan, "barpad.pad_renthal_fatbar-pad_blue").solid, "BARPAD701_003");
}

#[test]
fn the_ladders_climb_in_equal_steps_to_the_full_multiplier() {
    let plan = plan();
    let component = |id: &str| plan.components.iter().find(|(known, _)| known == id).map(|(_, c)| c.clone()).unwrap();
    let first = component("r6:engine.l1.10400");
    assert_eq!((first.tier, first.muls.len()), (1, 1));
    assert!((first.muls[0].1 - 1.04).abs() < 1e-9 && first.muls[0].0 == "bike.engine.peak_torque_nm");
    assert_eq!(component("r6:engine.l3.11200").tier, 3);
    let brakes = component("r6:brakes.l2.10750");
    assert_eq!(
        brakes.muls.iter().map(|(key, _)| *key).collect::<Vec<_>>(),
        ["bike.brakes.front_force_n", "bike.brakes.rear_force_n"]
    );
    assert!((step(1.2, 3, 3) - 1.2).abs() < 1e-12 && (step(0.9, 1, 2) - 0.95).abs() < 1e-12);
}

#[test]
fn a_bike_gets_only_the_slots_its_data_and_its_stock_parts_allow() {
    let have: Vec<&Piece> = PIECES.iter().filter(|piece| piece.tag != "BARPAD" && piece.tag != "REAR_RIM").collect();
    let plan = Plan::of("ktm450", &options(), &have, Mounts::default());
    let ids: Vec<&str> = plan.slots.iter().map(|slot| slot.id).collect();
    assert_eq!(ids, ["exhaust", "brake_discs", "perf_engine", "perf_chassis", "perf_brakes", "road_equipment"]);
    let none = BikeOptions {
        upgrade: Some(Upgrade { peak_torque: 1.0, mass: 1.0, brake_force: 1.0 }),
        ..BikeOptions::default()
    };
    let have: Vec<&Piece> = PIECES.iter().filter(|piece| !matches!(piece.tag, "MIRRORS" | "HOMOLOGATION")).collect();
    assert!(Plan::of("x", &none, &have, Mounts::default()).slots.is_empty());
}

fn road(tags: &[&str]) -> Plan {
    let have: Vec<&Piece> = PIECES.iter().filter(|piece| tags.contains(&piece.tag)).collect();
    Plan::of("pani", &BikeOptions::default(), &have, Mounts::default())
}

#[test]
fn a_bike_with_road_equipment_gets_one_optional_slot_that_is_shown_by_default() {
    let plan = road(&["MIRRORS", "HOMOLOGATION", "EXHAUST"]);
    assert_eq!(plan.slots.len(), 1, "no catalogue row is needed for the road equipment");
    let slot = &plan.slots[0];
    assert_eq!((slot.id, slot.group, slot.optional), ("road_equipment", "road", true));
    assert_eq!((slot.accepts.as_slice(), slot.default.as_str()), (["r6:road.on".to_owned()].as_slice(), "r6:road.on"));
    let mirrors = part(&plan, "road.mirrors");
    assert_eq!((mirrors.solid.as_str(), mirrors.pack.as_deref(), mirrors.role), ("PANI_KIT00_MIRRORS", None, "body"));
    assert_eq!(part(&plan, "road.homologation").solid, "PANI_KIT00_HOMOLOGATION");
    assert!(
        plan.parts.iter().all(|part| !part.id.starts_with("exhaust")),
        "the exhaust has no slot without a catalogue row"
    );
    let shows = &plan.components[0].1.shows;
    assert_eq!(shows, &["road.mirrors", "road.homologation"], "one component for every bike");
}

#[test]
fn a_bike_with_only_the_mirrors_binds_only_them() {
    let plan = road(&["MIRRORS"]);
    assert_eq!(plan.slots.len(), 1);
    assert_eq!(plan.parts.len(), 1);
    assert!(road(&["EXHAUST", "FRONT_RIM"]).slots.is_empty(), "a bike with none has no slot");
}

#[test]
fn the_road_slot_is_written_optional_and_the_game_reads_it_back() {
    let plan = road(&["MIRRORS", "HOMOLOGATION"]);
    let text = plan.text();
    assert!(text.contains("group = \"road\"") && text.contains("optional = true"), "{text}");
    let car = parse_car("pani", &text).unwrap();
    let components = car.components.unwrap();
    assert!(components.slots[0].optional && components.slots[0].group == "road");
    let catalog = parse_catalog(&catalog_text([&plan].into_iter())).unwrap();
    assert!(catalog.get("r6:road.on").is_some());
    let vehicle = VehicleDef { id: "pani".to_owned(), parts: BTreeSet::new(), slots: components.slots };
    vehicle.check(&catalog).unwrap();
}

#[test]
fn names_and_ids_are_made_from_the_rows() {
    assert_eq!(display("MUF_Akrapovic_Completo_Standard"), "Akrapovic Completo Standard");
    assert_eq!(display("RI_OZ_Piega-R_Livery-01"), "OZ Piega R");
    assert_eq!(slug("MUF_Akrapovic_Kit-RacingLine"), "muf_akrapovic_kit-racingline");
    let long = slug(&"X".repeat(60));
    assert!(long.len() <= 40 && long.starts_with("xxxxxxxx") && long.contains('-'));
    assert_ne!(slug(&format!("{}A", "x".repeat(60))), slug(&format!("{}B", "x".repeat(60))));
}

#[test]
fn the_text_is_read_back_by_the_game_and_its_slots_fit_the_catalog() {
    let plan = plan();
    let car = parse_car("ktm450", &plan.text()).unwrap();
    let components = car.components.unwrap();
    assert_eq!(components.slots.len(), 8);
    assert_eq!(components.parts["rim.front.rim738_001"].matrix.unwrap().len(), 12);
    let catalog = parse_catalog(&catalog_text([&plan].into_iter())).unwrap();
    let vehicle = VehicleDef { id: "ktm450".to_owned(), parts: BTreeSet::new(), slots: components.slots };
    vehicle.check(&catalog).unwrap();
    // The text of two bikes defines a shared component once.
    let twice = catalog_text([&plan, &plan].into_iter());
    assert_eq!(twice.matches("id = \"r6:exhaust.stock\"").count(), 1);
}

#[test]
fn a_twin_silencer_is_one_part_for_each_can_and_the_component_shows_them_all() {
    let mut twin = options();
    twin.exhaust[0].muffler_twins = vec![anchor(0.3)];
    let have: Vec<&Piece> = PIECES.iter().collect();
    let plan = Plan::of("ktm450", &twin, &have, Mounts::default());
    let slug = "muf_akrapovic_completo_standard";
    let first = plan.parts.iter().find(|part| part.id == format!("exhaust.muffler.{slug}")).unwrap();
    let second = plan.parts.iter().find(|part| part.id == format!("exhaust.muffler2.{slug}")).unwrap();
    assert_eq!(first.solid, "MUFFLER974_001");
    assert_eq!(second.solid, "MUFFLER974_001_G2", "the second can is the pack's solid of the second group");
    assert_eq!(first.pack, second.pack);
    assert!(second.matrix.is_some_and(|m| (m[3] - 0.3).abs() < 1e-6), "mounted by its own anchor");
    let component = &plan.components.iter().find(|(id, _)| id == &format!("r6:exhaust.{slug}")).unwrap().1;
    assert_eq!(component.shows.len(), 3);
    assert!(component.shows.contains(&format!("exhaust.muffler2.{slug}")));
}

#[test]
fn a_brake_level_comes_with_the_discs_of_the_kit_of_the_same_level() {
    let plan = plan();
    let ladder = plan.slots.iter().find(|slot| slot.id == "perf_brakes").unwrap();
    // The bike has a kit of level 1 only: its level 1 brings those discs, its level 2 brings none.
    assert_eq!(ladder.bundles["r6:brakes.l1.10375"], ["r6:brake_discs.brake700_001"]);
    assert!(!ladder.bundles.contains_key("r6:brakes.l2.10750"));
    let other = plan.slots.iter().filter(|slot| slot.id != "perf_brakes");
    assert!(other.into_iter().all(|slot| slot.bundles.is_empty()), "only the brake ladder is linked");
    // The manifest says so in the offer, and the game's parser and checker accept it.
    let text = plan.text();
    assert!(
        text.contains(r#"{ component = "r6:brakes.l1.10375", bundle = ["r6:brake_discs.brake700_001"] }"#),
        "{text}"
    );
    let car = parse_car("ktm450", &text).unwrap();
    let slots = car.components.unwrap().slots;
    let offer = slots.iter().find(|slot| slot.id == "perf_brakes").unwrap().offer("r6:brakes.l1.10375").unwrap();
    assert_eq!(offer.bundle, ["r6:brake_discs.brake700_001"]);
    let catalog = parse_catalog(&catalog_text([&plan].into_iter())).unwrap();
    let vehicle = VehicleDef { id: "ktm450".to_owned(), parts: BTreeSet::new(), slots };
    vehicle.check(&catalog).unwrap();
}

#[test]
fn a_bike_without_the_disc_slot_has_a_brake_ladder_that_brings_nothing() {
    let have: Vec<&Piece> = PIECES.iter().filter(|piece| !piece.tag.ends_with("_DISC")).collect();
    let plan = Plan::of("ktm450", &options(), &have, Mounts::default());
    let ladder = plan.slots.iter().find(|slot| slot.id == "perf_brakes").unwrap();
    assert!(ladder.bundles.is_empty());
}
