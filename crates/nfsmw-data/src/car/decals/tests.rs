use super::*;
use crate::car::customization::{Catalog, Category, Inventory, Selection};

#[test]
fn native_zones_and_texture_positions_cover_exactly_the_save_slots() {
    let slots: Vec<_> = DecalZone::ALL.into_iter().flat_map(|zone| (0..8).map(move |p| zone.texture_slot(p))).collect();
    assert_eq!(slots, (83..=130).collect::<Vec<_>>());
    assert_eq!(DecalZone::ALL.map(DecalZone::model_slot), [70, 71, 72, 73, 74, 75]);
}

#[test]
fn unavailable_positions_and_body_kit_changes_are_rejected() {
    let mut catalog = Catalog::default();
    catalog.stock[Category::Body as usize] = Some(10);
    let zone = &mut catalog.decals[DecalZone::LeftDoor as usize];
    zone.positions = 6;
    zone.body_positions = HashMap::from([(10, 6), (11, 1)]);
    zone.choices.push(Choice { hash: 42, label: "Decal".into(), language_hash: None, carbon: false });
    catalog.choices[Category::Body as usize].push(Choice {
        hash: 11,
        label: "Kit".into(),
        language_hash: None,
        carbon: false,
    });
    let mut selection = Selection::default();
    selection.decals[2][5] = Some(42);
    assert!(!selection.is_stock());
    assert!(catalog.validate(&selection).is_ok());
    selection.parts[Category::Body as usize] = Some(11);
    assert!(catalog.validate(&selection).is_err());
    selection.decals[2][5] = None;
    selection.decals[2][0] = Some(42);
    assert!(catalog.validate(&selection).is_ok());
    selection.decals[2][0] = Some(99);
    assert!(catalog.validate(&selection).is_err());
    selection.decals = [[None; 8]; 6];
    selection.decals[0][0] = Some(42);
    assert!(catalog.validate(&selection).is_err());
}

#[test]
fn numbers_use_separate_native_positions_and_digit_catalogs() {
    let mut catalog = Catalog::default();
    catalog.decals[2].number_positions = [true; 2];
    catalog.number_choices[0].push(Choice { hash: 12, label: "1".into(), language_hash: None, carbon: false });
    catalog.number_choices[1].push(Choice { hash: 23, label: "2".into(), language_hash: None, carbon: false });
    let mut selection = Selection::default();
    selection.decals[2][6] = Some(12);
    selection.decals[2][7] = Some(23);
    assert!(catalog.validate(&selection).is_ok());
    selection.decals[2][6] = Some(23);
    assert!(catalog.validate(&selection).is_err());
    assert_eq!(placeholder(DecalZone::LeftDoor, 6), bstring_hash("DUMMY_NUMBER_LEFT"));
    assert_eq!(placeholder(DecalZone::RightDoor, 7), bstring_hash("DUMMY_NUMBER_RIGHT"));
    assert_eq!(placeholder(DecalZone::FrontWindow, 6), bstring_hash("DUMMY_DECAL7"));
}

#[test]
#[ignore = "requires an installed game"]
fn installed_decals_bind_authored_surfaces_and_keep_other_zones_hidden() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let inventory = Inventory::load(&dir).unwrap();
    let options = crate::car::LoadOptions { lod: 'A', all_parts: false, preset: None };
    let mut checked = 0;
    for folder in ["COBALTSS", "GALLARDO", "CARRERAGT"] {
        let catalog = inventory.catalog(&dir, folder).unwrap();
        for zone in DecalZone::ALL {
            let available = &catalog.decals[zone as usize];
            let Some(choice) = available.choices.first() else { continue };
            let mut selection = Selection::default();
            for p in 0..available.positions {
                selection.decals[zone as usize][p] = Some(choice.hash);
            }
            catalog.validate(&selection).unwrap();
            let car = crate::car::load_customized_driving(&dir, folder, &options, &selection).unwrap();
            assert!(car.placements.iter().any(|p| p.slot == zone.model_slot()));
            assert!(
                car.placements
                    .iter()
                    .filter(|p| slot::DECAL_MODELS.contains(&p.slot))
                    .all(|p| p.slot == zone.model_slot())
            );
            for placement in car.placements.iter().filter(|p| p.slot == zone.model_slot()) {
                let solid = &car.solids[&placement.solid];
                for group in &solid.groups {
                    let Some(texture) = group.diffuse_texture(solid) else { continue };
                    if car.swaps.hidden.contains(&texture) {
                        assert!(
                            ["DUMMY_NUMBER_LEFT", "DUMMY_NUMBER_RIGHT"]
                                .into_iter()
                                .any(|name| texture == bstring_hash(name))
                        );
                        continue;
                    }
                    assert!(car.textures.contains_key(&texture));
                }
            }
            checked += 1;
        }
    }
    assert!(checked >= 10, "only {checked} installed decal zones checked");
    println!("{checked} installed decal zones use their authored models and installed textures");
}

#[test]
#[ignore = "requires an installed game"]
fn installed_numbers_bind_both_digits_on_both_doors() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let catalog = Catalog::load(&dir, "COBALTSS").unwrap();
    assert!(catalog.number_choices.iter().all(|choices| choices.len() >= 10));
    let mut selection = Selection::default();
    for zone in [2, 3] {
        for number in 0..2 {
            selection.decals[zone][number + 6] = Some(catalog.number_choices[number][number + 1].hash);
        }
    }
    catalog.validate(&selection).unwrap();
    let options = crate::car::LoadOptions { lod: 'A', all_parts: false, preset: None };
    let car = crate::car::load_customized_driving(&dir, "COBALTSS", &options, &selection).unwrap();
    for zone in [DecalZone::LeftDoor, DecalZone::RightDoor] {
        let placement = car.placements.iter().find(|p| p.slot == zone.model_slot()).unwrap();
        let solid = &car.solids[&placement.solid];
        for number in 0..2 {
            assert!(!solid.texture_hashes.contains(&placeholder(zone, number + 6)));
        }
    }
}
