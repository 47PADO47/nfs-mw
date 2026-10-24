use super::*;
use game_install::GameDir;
use nfsmw_data::car::{customization::Catalog, physics::PhysicsData};

fn installed() -> (CarTables, Model) {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let tables = CarTables::load(&dir).unwrap();
    let physics = PhysicsData::load(&dir).unwrap();
    let mut model = super::super::catalog(&dir, &physics).unwrap();
    model.cars.retain(|car| car.folder == "COBALTSS");
    model.cars.first_mut().unwrap().parts = Some(Catalog::load(&dir, "COBALTSS").unwrap().into());
    (tables, model)
}

fn nonstock_color(tables: &CarTables, slot: usize) -> [u8; 3] {
    let car = tables.car_type_for_folder("COBALTSS").unwrap();
    let stock = stock_parts(tables, car)[slot].map(|part| Paint::of(tables, &part).rgb);
    tables
        .parts
        .parts()
        .iter()
        .find(|part| {
            part.part_id == LAYOUT.slots[slot].part_id
                && tables
                    .slot_types
                    .search_types(slot, car.type_name_hash)
                    .any(|kind| kind == tables.parts.type_name_hash(part))
                && Some(Paint::of(tables, part).rgb) != stock
        })
        .map(|part| Paint::of(tables, part).rgb)
        .expect("an installed alternate swatch")
}

#[test]
#[ignore = "requires an installed game"]
fn installed_native_garage_roundtrip_keeps_colors_gauges_and_unrelated_bytes() {
    let (tables, model) = installed();
    let catalog = model.cars[0].parts.as_ref().unwrap();
    let mut selection = Selection { car: "COBALTSS".into(), ..Default::default() };
    for category in [Category::Body, Category::Wheels] {
        selection.parts.parts[category as usize] = Some(catalog.choices[category as usize].last().unwrap().hash);
    }
    for i in 0..3 {
        selection.parts.vinyl_colors[i] = Some(nonstock_color(&tables, 79 + i));
    }
    selection.gauges.skin = 4;
    selection.gauges.backing = Some(nonstock_color(&tables, 133));
    selection.gauges.needle = Some(nonstock_color(&tables, 134));
    selection.gauges.digits = Some(nonstock_color(&tables, 135));
    selection.performance = model.cars[0].performance.maximum();
    selection.tuning.profiles[2] = [-5, -3, -1, 0, 1, 3, 5];
    selection.tuning.active = 2;
    let mut bytes = *CustomizationRecord::default().as_bytes();
    bytes[278..280].copy_from_slice(&[17, 19]);
    bytes[400..408].copy_from_slice(&[9, 8, 7, 6, 5, 4, 3, 2]);
    let mut original = CustomizationRecord::parse(&bytes).unwrap();
    original.set_part(52, Some(123));
    original.set_part(82, Some(321));
    let saved = write(&original, &tables, &selection).unwrap();
    assert_eq!(&saved.as_bytes()[278..280], &[17, 19]);
    assert_eq!(&saved.as_bytes()[400..408], &[9, 8, 7, 6, 5, 4, 3, 2]);
    assert_eq!(saved.part(52), Some(123));
    assert_eq!(saved.part(82), Some(321), "unexposed fourth vinyl swatch is retained");
    assert_eq!(saved.part(slot::FRONT_WHEEL), saved.part(slot::REAR_WHEEL));
    let restored = read(&saved, &tables, "COBALTSS", &model).unwrap();
    // Importing a legacy RGB-only selection also learns its exact native swatch identities.
    selection.parts.vinyl_color_parts = restored.parts.vinyl_color_parts;
    selection.gauges.color_parts = restored.gauges.color_parts;
    assert_eq!(restored, selection);
    assert_eq!(write(&saved, &tables, &restored).unwrap(), saved, "native roundtrip must be idempotent");
    assert!(slot::KIT_DECALS.clone().any(|slot| saved.part(slot).is_some()), "new kit must update dependent models");
}

#[test]
#[ignore = "requires an installed game"]
fn installed_unchanged_native_rear_wheel_and_custom_rgb_survive_updates() {
    let (tables, model) = installed();
    let car = tables.car_type_for_folder("COBALTSS").unwrap();
    let wheels = &model.cars[0].parts.as_ref().unwrap().choices[Category::Wheels as usize];
    let mut record = CustomizationRecord::default();
    store_hash(&mut record, &tables, slot::FRONT_WHEEL, Some(wheels.first().unwrap().hash)).unwrap();
    store_hash(&mut record, &tables, slot::REAR_WHEEL, Some(wheels.last().unwrap().hash)).unwrap();
    let swatch = nonstock_color(&tables, 79);
    write_color(&mut record, &tables, car.type_name_hash, 79, None, Some(swatch), None).unwrap();
    let old_rear = record.part(slot::REAR_WHEEL);
    let old_swatch = record.part(79);
    let mut selection = read(&record, &tables, "COBALTSS", &model).unwrap();
    let rgb = [1, 2, 3];
    assert!(!tables.parts.parts().iter().any(|part| part.part_id == 77 && Paint::of(&tables, part).rgb == rgb));
    selection.parts.vinyl_colors[0] = Some(rgb);
    selection.parts.vinyl_color_parts[0] = None;
    let saved = write(&record, &tables, &selection).unwrap();
    assert_eq!(saved.part(slot::REAR_WHEEL), old_rear, "an unrelated rear-wheel identity must survive");
    assert_eq!(saved.part(79), old_swatch, "extension-only RGB must not invent a native swatch");
    let mut bad = saved.clone();
    let paint_index = tables.parts.parts().iter().position(|part| part.part_id == 76).unwrap();
    bad.set_part(slot::BODY, Some(paint_index as u16));
    assert!(read(&bad, &tables, "COBALTSS", &model).is_err(), "native indices must match their slot");
}
