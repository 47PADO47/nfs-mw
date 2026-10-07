use crate::{install, unwrapped};

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn every_car_geometry_parses() {
    let Some(dir) = install() else { return };
    let (mut cars, mut solids) = (0, 0);
    for car in nfsmw_data::car::list(&dir) {
        let rel = format!("CARS/{car}/GEOMETRY.BIN");
        let parsed = blackbox_solid::read_solids(&unwrapped(&dir, &rel)).unwrap_or_else(|e| panic!("{rel}: {e}"));
        assert!(!parsed.is_empty(), "{rel}: no solids");
        for s in &parsed {
            assert_eq!(s.name_hash, blackbox_hash::bstring_hash(&s.name), "{rel}: {}", s.name);
            crate::world::assert_indices_in_range(s, &rel);
        }
        cars += 1;
        solids += parsed.len();
    }
    eprintln!("{cars} cars, {solids} solids");
    assert!(cars >= 50);
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn bmw_m3_gtr_matches_documented_numbers() {
    let Some(dir) = install() else { return };
    let solids = blackbox_solid::read_solids(&unwrapped(&dir, "CARS/BMWM3GTR/GEOMETRY.BIN")).unwrap();
    // docs/formats/models.md: 97 solids, each with one 36-byte vertex buffer.
    assert_eq!(solids.len(), 97);
    assert!(solids.iter().all(|s| s.vertex_buffers.len() == 1 && s.vertex_buffers[0].stride == 36));
    let base = solids.iter().find(|s| s.name == "BMWM3GTR_BASE_A").unwrap();
    assert_eq!((base.groups.len(), base.vertices.len(), base.indices.len()), (7, 345, 1062));
    // docs/formats/cardata.md (SolidMarkers): the wheel carries the brake depth, the base the lights.
    let wheel = solids.iter().find(|s| s.name == "BMWM3GTR_KIT00_FRONT_TIRE_A").unwrap();
    let brake = wheel.marker(blackbox_hash::bstring_hash("FRONT_BRAKE")).expect("FRONT_BRAKE marker");
    assert!((brake.translation()[1] - 0.046).abs() < 1e-3, "{:?}", brake.translation());
    let headlight = base.marker(blackbox_hash::bstring_hash("LEFT_HEADLIGHT")).expect("LEFT_HEADLIGHT marker");
    let [x, y, z] = headlight.translation();
    assert!((x - 2.162).abs() < 1e-2 && (y - 0.581).abs() < 1e-2 && (z - 0.473).abs() < 1e-2, "{x} {y} {z}");
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn bmw_m3_gtr_assembles() {
    use nfsmw_data::car::{LoadOptions, slot};
    let Some(dir) = install() else { return };
    let options = LoadOptions { lod: 'A', all_parts: false, preset: None };
    let car = nfsmw_data::car::load(&dir, "BMWM3GTR", &options).unwrap();
    assert_eq!(car.car_type.as_deref(), Some("BMWM3GTR"));
    // docs/specs/car-assembly.md §7: stock paint METAL_L1_COLOR02, #4F4F4F.
    let paint = car.paint.as_ref().unwrap();
    assert_eq!((paint.name.as_str(), paint.hex().as_str()), ("METAL_L1_COLOR02", "#4F4F4F"));
    // Four wheels (the front tyre solid at every corner) and four brakes.
    let wheel = blackbox_hash::bstring_hash("BMWM3GTR_KIT00_FRONT_TIRE_A");
    let wheels: Vec<_> = car.placements.iter().filter(|p| p.solid == wheel).collect();
    assert_eq!(wheels.len(), 4);
    let brakes = car.placements.iter().filter(|p| p.slot == slot::FRONT_BRAKE || p.slot == slot::REAR_BRAKE);
    assert_eq!(brakes.count(), 4);
    // Nothing hidden in the front end is placed.
    let names: Vec<&str> = car.placements.iter().map(|p| car.solids[&p.solid].name.as_str()).collect();
    assert!(names.iter().all(|n| !n.contains("DAMAGE0") && !n.contains("DRIVER") && !n.contains("DECAL")), "{names:?}");
    assert!(names.contains(&"BMWM3GTR_KIT00_BODY_A") && names.contains(&"BMWM3GTR_BASE_A"));
    assert!((car.floor_height - 0.095).abs() < 1e-3, "{}", car.floor_height);
    assert!(car.textures.len() >= 15, "{} textures", car.textures.len());
}

/// docs/specs/car-assembly.md §1: the stock rule reproduces the CE_GTRSTREET preset exactly.
#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn stock_m3_gtr_matches_its_preset() {
    use blackbox_carparts::PresetRide;
    use nfsmw_data::car::{CarTables, stock_parts};
    let Some(dir) = install() else { return };
    let t = CarTables::load(&dir).unwrap();
    let car = t.car_type_for_folder("BMWM3GTR").unwrap();
    let stock = stock_parts(&t, car);
    let preset = t.preset("CE_GTRSTREET").unwrap();
    let mut listed = 0;
    for (slot, &hash) in preset.parts.iter().enumerate() {
        if hash == PresetRide::STOCK {
            continue;
        }
        let ours = stock[slot].map_or(PresetRide::EMPTY, |p| p.name_hash);
        assert_eq!(ours, hash, "slot {slot} ({})", nfsmw_data::car::LAYOUT.slots[slot].name);
        listed += usize::from(hash != PresetRide::EMPTY);
    }
    assert_eq!(listed, 81);
}

/// Every car type assembles with its wheels placed.
#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn every_car_type_assembles() {
    use nfsmw_data::car::{CarTables, LoadOptions, slot};
    let Some(dir) = install() else { return };
    let t = CarTables::load(&dir).unwrap();
    let options = LoadOptions { lod: 'A', all_parts: false, preset: None };
    let mut without_wheels = Vec::new();
    for folder in nfsmw_data::car::list(&dir) {
        if t.car_type_for_folder(&folder).is_none() {
            continue;
        }
        let car = nfsmw_data::car::load(&dir, &folder, &options).unwrap_or_else(|e| panic!("{folder}: {e:#}"));
        if car.placements.iter().filter(|p| p.slot == slot::FRONT_WHEEL || p.slot == slot::REAR_WHEEL).count() != 4 {
            without_wheels.push(folder);
        }
    }
    // COPHELI has no wheels; BMWM3 has no ecar record (no gameplay vehicle uses it).
    without_wheels.sort();
    assert_eq!(without_wheels, ["BMWM3", "COPHELI"]);
}
