use super::*;
use crate::frontend::look::{CarLook, CarRecordRef};
use nfsmw_career::stable::CustomizationRecord;
use nfsmw_data::car::customization::Category;

#[test]
#[ignore = "requires installed career customization and car geometry; set NFSMW_GAME_DIR"]
fn career_batch13_backdrop_restores_the_captured_owned_slots_parts() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let model = catalog(&dir, "COBALTSS").unwrap();
    let parts = model.cars[0].parts.as_ref().unwrap();
    let body = parts.choices[Category::Body as usize]
        .iter()
        .find(|choice| Some(choice.hash) != parts.stock[Category::Body as usize])
        .expect("installed body kit")
        .hash;
    let tables = CarTables::load(&dir).unwrap();
    let index = tables.parts.parts().iter().position(|part| part.name_hash == body).unwrap();
    let mut modified = CustomizationRecord::default();
    assert!(modified.set_part(Category::Body.slot(), Some(index.try_into().unwrap())));
    let look = |record| {
        Look::SafeHouse(CarLook {
            car: "COBALTSS".into(),
            preset: None,
            record: Some(CarRecordRef {
                handle: 27,
                alias: Some("UI13PREVIEW".into()),
                customization: Some(record),
                ..Default::default()
            }),
        })
    };
    let selected = look(modified.clone());
    // Another slot's record is not consulted by the worker, even when it changes after the request.
    let stock = look(CustomizationRecord::default());
    let restored =
        crate::scenes::world::career_preview_selection(&dir, &model, "COBALTSS", "UI13PREVIEW", 27, &modified, None)
            .unwrap();
    assert_eq!(restored.parts.parts[Category::Body as usize], Some(body));
    let (car, _) = load(&dir, &selected).expect("owned record").unwrap();
    let (original, _) = load(&dir, &stock).expect("owned stock record").unwrap();
    assert_eq!(car.name, "COBALTSS");
    let solids = |car: &CarModel| {
        let mut keys: Vec<_> = car.placements.iter().map(|placement| placement.solid).collect();
        keys.sort_unstable();
        keys
    };
    assert_ne!(solids(&car), solids(&original), "the preview must assemble the saved kit, not its base car");
}

#[test]
#[ignore = "requires installed career customization; set NFSMW_GAME_DIR"]
fn career_batch13_saved_rgb_extension_is_scoped_to_the_owned_slot_and_record() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let model = catalog(&dir, "COBALTSS").unwrap();
    let record = CustomizationRecord::default();
    let selection =
        crate::garage::Selection { car: "COBALTSS".into(), color: Some([17, 83, 211]), ..Default::default() };
    let root = std::env::temp_dir().join(format!(
        "career-preview-{}-{}",
        std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    ));
    let profiles = crate::career::ProfileDir::new(&root);
    let path = profiles.mod_data_dir("UI13").unwrap().join("garage.toml");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let fingerprint: String =
        save_store::digest::md5(record.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect();
    let mut payload = toml::Table::new();
    payload.insert("record".into(), fingerprint.into());
    payload.insert("selection".into(), crate::garage::presets::encode(&selection).unwrap().into());
    let store = crate::preset_store::updated(&path, "0000001b", &toml::to_string(&payload).unwrap()).unwrap();
    std::fs::write(&path, store).unwrap();
    let restore = |handle, record: &CustomizationRecord| {
        crate::scenes::world::career_preview_selection(
            &dir,
            &model,
            "COBALTSS",
            "UI13",
            handle,
            record,
            Some(&profiles),
        )
        .unwrap()
    };
    assert_eq!(restore(27, &record).color, selection.color);
    assert_eq!(restore(28, &record).color, None, "a neighbouring owned slot does not inherit this paint");
    let mut changed = record.clone();
    changed.set_tuning([[0.0; 7]; 3], 1);
    assert_eq!(restore(27, &changed).color, None, "stale extension must not override a changed native record");
    std::fs::remove_dir_all(root).unwrap();
}
