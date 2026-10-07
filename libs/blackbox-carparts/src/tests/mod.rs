mod builder;

use blackbox_hash::bstring_hash;

use crate::layout::MOST_WANTED;
use crate::{PartQuery, PresetRide, read_car_types, read_parts_db, read_preset_rides, read_slot_types};

#[test]
fn parts_and_model_names() {
    let db = read_parts_db(&builder::parts_pack(), &MOST_WANTED).unwrap();
    let car = bstring_hash("CAR");
    assert_eq!(db.parts().len(), 3);
    assert_eq!(db.type_names(), [car, bstring_hash("WHEELS")]);

    // The stock body: first part id 23 at upgrade level 0, templated name CAR + _KIT00 + _BODY + _A.
    let body = db.find(&PartQuery::new(23, car).upgrade_level(0)).unwrap();
    assert_eq!(db.name(body), "BODY_00");
    assert_eq!(db.model_hash(body, 0), Some(bstring_hash("CAR_KIT00_BODY_A")));
    assert_eq!(db.model_hash(body, 1), None); // no model at LOD B
    assert_eq!(db.attribute(body, "KITNUMBER"), Some(0));

    // Level 1 is skipped by the level-0 query but found by name.
    let kit = db.find(&PartQuery::new(23, car).upgrade_level(1)).unwrap();
    assert_eq!(db.model_hash(kit, 0), Some(bstring_hash("CAR_KIT01_BODY_A")));
    assert_eq!(db.attribute(kit, "KITNUMBER"), Some(1));
    assert_eq!(db.by_name_hash(kit.name_hash), Some(kit));

    // An aftermarket rim: the name starts from its BRAND_NAME attribute.
    let rim = db.find(&PartQuery::new(67, bstring_hash("WHEELS"))).unwrap();
    assert_eq!(db.model_hash(rim, 0), Some(bstring_hash("BBS_STYLE01_18_25_A")));
    assert!(db.find(&PartQuery::new(67, car)).is_none());
}

#[test]
fn slot_types_resolve_the_car_type() {
    let data = builder::slot_types();
    let slots = read_slot_types(&data, &MOST_WANTED);
    assert_eq!(slots.defaults.len(), 139);
    let car = bstring_hash("CAR");
    let porsche = bstring_hash("911TURBO");
    let spoiler = MOST_WANTED.slot("SPOILER").unwrap();
    assert_eq!(slots.search_types(0, car).collect::<Vec<_>>(), [car]);
    assert_eq!(slots.search_types(spoiler, car).collect::<Vec<_>>(), [car, bstring_hash("SPOILER")]);
    assert_eq!(slots.search_types(spoiler, porsche).collect::<Vec<_>>(), [bstring_hash("SPOILER_PORSCHES")]);
}

#[test]
fn car_types_and_presets() {
    let types = read_car_types(&builder::car_types(), &MOST_WANTED);
    assert_eq!(types.len(), 1);
    let t = &types[0];
    assert_eq!((t.type_name.as_str(), t.base_model_name.as_str()), ("CAR", "CAR"));
    assert_eq!((t.type_name_hash, t.skinnable, t.default_base_paint), (bstring_hash("CAR"), true, 0xC7F2_884E));

    let presets = read_preset_rides(&builder::preset(), &MOST_WANTED);
    assert_eq!(presets.len(), 1);
    assert_eq!(presets[0].preset_name, "CAR_STREET");
    assert_eq!(presets[0].parts.len(), 139);
    assert_eq!(presets[0].parts[23], PresetRide::STOCK);
}

#[test]
fn slot_table() {
    let slots = MOST_WANTED.slots;
    assert_eq!(slots.len(), 139);
    assert_eq!(MOST_WANTED.slot("BODY"), Some(23));
    assert_eq!(MOST_WANTED.slot("FRONT_WHEEL"), Some(66));
    assert_eq!((slots[66].part_id, slots[67].part_id), (67, 67));
    assert_eq!((slots[76].part_id, slots[77].part_id, slots[79].part_id), (76, 79, 77));
    assert_eq!((slots[130].name, slots[130].part_id), ("DECAL_RIGHT_QUARTER_TEX7", 80));
    assert_eq!((slots[138].name, slots[138].part_id), ("MISC", 86));
}
