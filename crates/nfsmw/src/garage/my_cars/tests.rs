use super::*;

fn selection(color: [u8; 3]) -> Selection {
    Selection { car: "CAR".into(), color: Some(color), ..Default::default() }
}

#[test]
fn two_slots_of_one_model_keep_independent_stable_identities() {
    let mut store = store::Store::default();
    let red = store.create(&selection([200, 10, 10]), "Red car").unwrap();
    let blue = store.create(&selection([10, 10, 200]), "Blue car").unwrap();
    assert_ne!(red, blue);
    assert_ne!(store.slots[&red].selection, store.slots[&blue].selection);
    store.slots.remove(&red);
    store.deleted.insert(red.clone());
    let third = store.create(&selection([10, 200, 10]), "Green car").unwrap();
    assert_ne!(third, red, "deleting a slot must not reuse its identity");
    assert!(store.save(&red, &selection([0; 3]), "Deleted").is_err());
}

#[test]
fn the_original_capacity_blocks_a_twenty_first_custom_slot() {
    let mut store = store::Store::default();
    for index in 0..20 {
        store.create(&selection([index; 3]), "Custom").unwrap();
    }
    assert!(store.create(&selection([0; 3]), "Full").is_err());
    store.slot_limit = 21;
    assert!(store.create(&selection([0; 3]), "Extended").is_ok(), "modded capacity is data-driven");
}

#[test]
fn migration_preserves_legacy_payload_and_delete_survives_reopen() {
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let root = std::env::temp_dir().join(format!("nfsmw-slot-store-{}-{stamp}", std::process::id()));
    let legacy = root.join("legacy.toml");
    let first = root.join("first/my-cars.toml");
    let second = root.join("second/my-cars.toml");
    let payload = crate::garage::presets::encode(&selection([50, 100, 150])).unwrap();
    crate::preset_store::save(&legacy, "CAR", &payload).unwrap();
    let original = std::fs::read(&legacy).unwrap();
    let mut one = store::Store::read(&first, Some(&legacy)).unwrap();
    assert_eq!(one.slots["legacy-CAR"].selection, payload);
    let added = one.create(&selection([100, 50, 150]), "Alias one").unwrap();
    one.slots.remove("legacy-CAR");
    one.deleted.insert("legacy-CAR".into());
    one.write(&first).unwrap();
    let reopened = store::Store::read(&first, Some(&legacy)).unwrap();
    assert!(!reopened.slots.contains_key("legacy-CAR"));
    assert!(reopened.deleted.contains("legacy-CAR"));
    assert!(reopened.slots.contains_key(&added));
    let two = store::Store::read(&second, Some(&legacy)).unwrap();
    assert!(!two.slots.contains_key(&added), "new slots belong to their alias");
    assert!(two.slots.contains_key("legacy-CAR"), "each alias can migrate the original setup");
    assert_eq!(std::fs::read(&legacy).unwrap(), original);
    std::fs::remove_dir_all(root).unwrap();
}
