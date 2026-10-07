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
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn car_loader_finds_most_textures() {
    let Some(dir) = install() else { return };
    let car = nfsmw_data::car::load(&dir, "BMWM3GTR", 'A', false).unwrap();
    assert_eq!(car.solids.len(), 25);
    assert!(car.textures.len() >= 15, "{} textures", car.textures.len());
}
