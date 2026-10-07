//! Synthetic packs: a `car` class with every decoded value kind, an `engine` class, and a second
//! vault whose collections use the first vault's classes.

use super::fixture::{Data, Def, Entry, VaultBuilder, add_class, add_collection, pack};
use crate::hash::vlt_hash;
use crate::{Database, Error, RefSpec, Value};

const STRINGS: &[&str] = &["default", "bmw", "v8", "base car", "BASE", "BMWM3GTR", "race"];

fn def(name: &'static str, type_name: &'static str, offset: u16, size: u16, max: u16, flags: u8, align: u8) -> Def {
    Def { name, type_name, offset, size, max_count: max, flags, align_log2: align }
}

fn car_defs() -> Vec<Def> {
    vec![
        def("MASS", "EA::Reflection::Float", 0x00, 4, 1, 6, 2),
        def("HORN", "EA::Reflection::UInt8", 0x04, 1, 1, 6, 0),
        def("MODEL", "Attrib::StringKey", 0x08, 16, 1, 6, 3),
        def("TORQUE", "EA::Reflection::Float", 0x18, 4, 3, 7, 2),
        def("TENSOR", "Attrib::Types::Vector4", 0x30, 16, 1, 6, 4),
        def("GEARS", "EA::Reflection::Int32", 0, 4, 1, 4, 2),
        def("LABEL", "EA::Reflection::Text", 0, 4, 1, 4, 2),
        def("engine", "Attrib::RefSpec", 0, 12, 1, 4, 2),
        def("RATIOS", "EA::Reflection::Float", 0, 4, 4, 5, 2),
        def("SCRIPT", "Attrib::Blob", 0, 8, 1, 4, 2),
        def("WOOSH", "eDRIVE_BY_TYPE", 0, 4, 1, 4, 2),
        def("SPIN", "Attrib::Types::Vector4", 0, 16, 2, 5, 4),
        def("ACTIVE", "EA::Reflection::Bool", 0, 1, 1, 4, 0),
    ]
}

fn floats(values: &[f32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

/// `{u16 capacity, u16 count, u16 item size, u16 flags}`.
fn array_header(capacity: u16, count: u16, size: u16, flags: u16) -> Vec<u8> {
    [capacity, count, size, flags].iter().flat_map(|v| v.to_le_bytes()).collect()
}

/// A `car` layout block; its `MODEL` string pointer gets a `.bin` fix-up.
fn car_layout(b: &mut VaultBuilder, mass: f32, model: &str, torque: &[f32]) -> u32 {
    let mut block = vec![0u8; 0x40];
    block[0..4].copy_from_slice(&mass.to_le_bytes());
    block[4] = 2;
    block[0x08..0x10].copy_from_slice(&0x1122_3344_5566_7788u64.to_le_bytes());
    block[0x10..0x14].copy_from_slice(&vlt_hash(model).to_le_bytes());
    block[0x18..0x20].copy_from_slice(&array_header(3, torque.len() as u16, 4, 0));
    block[0x20..0x20 + 4 * torque.len()].copy_from_slice(&floats(torque));
    block[0x30..0x40].copy_from_slice(&floats(&[1.0, 2.0, 3.0, 0.0]));
    let at = b.bin_data(&block);
    let model_at = b.string(model);
    b.bin_ptr(at + 0x14, model_at);
    at
}

fn entry(field: &'static str, type_index: u16, node_flags: u8, data: Data) -> Entry {
    Entry { field, type_index, node_flags, data }
}

fn db_vault() -> (String, Vec<u8>, Vec<u8>) {
    let mut b = VaultBuilder::new("db", STRINGS);
    add_class(&mut b, "car", &car_defs(), 0x40);
    add_class(&mut b, "engine", &[def("TORQUE_PEAK", "EA::Reflection::Float", 0, 4, 1, 4, 2)], 0);

    let base = car_layout(&mut b, 1000.0, "BASE", &[100.0, 200.0]);
    let ratios = b.bin_data(&[array_header(2, 2, 4, 0), floats(&[3.5, 2.0])].concat());
    let reference = b.bin_data(&[vlt_hash("engine"), vlt_hash("v8"), 0].map(u32::to_le_bytes).concat());
    let label = b.string("base car");
    let types = ["EA::Reflection::Int32", "EA::Reflection::Text", "EA::Reflection::Float", "Attrib::RefSpec"];
    let types = [&types[..], &["eDRIVE_BY_TYPE", "EA::Reflection::Bool"]].concat();
    add_collection(
        &mut b,
        "car",
        "default",
        None,
        Some(base),
        &types,
        &[
            entry("GEARS", 0, 0x20, Data::Inline(6)),
            entry("LABEL", 1, 0x20, Data::InlinePtr(label)),
            entry("RATIOS", 2, 0x02, Data::Ptr(ratios)),
            entry("engine", 3, 0x00, Data::Ptr(reference)),
            entry("WOOSH", 4, 0x20, Data::Inline(7)),
            entry("ACTIVE", 5, 0x20, Data::Inline(1)),
        ],
    );

    let bmw = car_layout(&mut b, 1350.0, "BMWM3GTR", &[400.0, 450.0, 500.0]);
    let mut raww = b"RAWW\x01\x10\0\0".to_vec();
    raww.extend_from_slice(&[5u32, 21].map(u32::to_le_bytes).concat());
    raww.extend_from_slice(b"hello");
    let blob_data = b.bin_data(&raww);
    let blob = b.bin_data(&[raww.len() as u32, 0].map(u32::to_le_bytes).concat());
    b.bin_ptr(blob + 4, blob_data);
    let spin = b.bin_data(&[array_header(1, 1, 16, 0x8000), vec![0; 8], floats(&[1.0, 0.0, 0.0, 1.0])].concat());
    let types = ["EA::Reflection::Int32", "Attrib::Blob", "Attrib::Types::Vector4"];
    add_collection(
        &mut b,
        "car",
        "bmw",
        Some("default"),
        Some(bmw),
        &types,
        &[
            entry("GEARS", 0, 0x20, Data::Inline(5)),
            entry("SCRIPT", 1, 0x00, Data::Ptr(blob)),
            entry("SPIN", 2, 0x02, Data::Ptr(spin)),
        ],
    );

    let types = ["EA::Reflection::Float"];
    add_collection(
        &mut b,
        "engine",
        "v8",
        None,
        None,
        &types,
        &[entry("TORQUE_PEAK", 0, 0x20, Data::Inline(520f32.to_bits()))],
    );
    b.finish()
}

/// Collections only, of a class defined in `db` (like `gameplay.bin`).
fn extra_vault() -> (String, Vec<u8>, Vec<u8>) {
    let mut b = VaultBuilder::new("extra", &["race"]);
    let types = ["EA::Reflection::Int32"];
    add_collection(&mut b, "car", "race", Some("bmw"), None, &types, &[entry("GEARS", 0, 0x20, Data::Inline(4))]);
    b.finish()
}

fn database() -> Database {
    Database::open(&pack(&[db_vault(), extra_vault()])).unwrap()
}

#[test]
fn classes_and_collections() {
    let db = database();
    assert_eq!(db.vaults().len(), 2);
    assert_eq!(db.vault("db").unwrap().dependencies()[1].name, "db.bin");
    assert_eq!(db.classes().len(), 2);
    let car = db.class("car").unwrap();
    assert_eq!((car.fields.len(), car.layout_size, car.layout_count), (13, 0x40, 5));
    let torque = car.field("TORQUE").unwrap();
    assert!(torque.is_array() && torque.in_layout() && torque.max_count == 3 && torque.alignment == 4);
    let offsets: Vec<_> = car.layout_fields().map(|f| f.offset).collect();
    assert_eq!(offsets, [0x00, 0x04, 0x08, 0x18, 0x30]);
    assert_eq!(db.collection_count(), 4);
    assert_eq!(db.collections_of("car").count(), 3);
    let race = db.collection("car", "race").unwrap();
    assert_eq!(race.vault().name(), "extra");
    assert_eq!(race.lineage().map(|c| c.name().unwrap()).collect::<Vec<_>>(), ["race", "bmw", "default"]);
}

#[test]
fn values_and_inheritance() {
    let db = database();
    let bmw = db.collection("car", "bmw").unwrap();
    assert_eq!(bmw.get_f32("MASS"), Some(1350.0));
    assert_eq!(bmw.get("HORN"), Some(&Value::UInt8(2)));
    let model = bmw.get_string_key("MODEL").unwrap();
    assert_eq!((model.hash64, model.hash32), (Some(0x1122_3344_5566_7788), vlt_hash("BMWM3GTR")));
    assert_eq!(bmw.get_str("MODEL"), Some("BMWM3GTR"));
    let torque = bmw.get("TORQUE").unwrap().as_array().unwrap();
    assert_eq!(torque, [Value::Float(400.0), Value::Float(450.0), Value::Float(500.0)]);
    assert_eq!(bmw.get_f32("TORQUE"), Some(400.0)); // item 0
    assert_eq!(bmw.get_vector4("TENSOR"), Some([1.0, 2.0, 3.0, 0.0]));
    assert_eq!(bmw.get_i32("GEARS"), Some(5));

    // Inherited from `default`.
    assert_eq!(bmw.get_own("LABEL"), None);
    assert_eq!(bmw.get_str("LABEL"), Some("base car"));
    assert_eq!(bmw.get_at("RATIOS", 1), Some(&Value::Float(2.0)));
    assert_eq!(bmw.get_u32("WOOSH"), Some(7)); // an enum: raw bytes
    assert!(matches!(bmw.get("WOOSH"), Some(Value::Raw { bytes, .. }) if bytes == &[7, 0, 0, 0]));
    assert_eq!(bmw.get_bool("ACTIVE"), Some(true));

    // Two levels up, and nothing flows down.
    let race = db.collection("car", "race").unwrap();
    assert_eq!(race.get_i32("GEARS"), Some(4));
    assert_eq!(race.get_f32("MASS"), Some(1350.0));
    assert_eq!(race.get_str("LABEL"), Some("base car"));
    assert_eq!(db.collection("car", "default").unwrap().get("SCRIPT"), None);
}

#[test]
fn refs_blobs_and_aligned_arrays() {
    let db = database();
    let bmw = db.collection("car", "bmw").unwrap();
    assert_eq!(bmw.get_ref("engine"), Some(RefSpec { class: vlt_hash("engine"), collection: vlt_hash("v8") }));
    let v8 = bmw.follow("engine").unwrap();
    assert_eq!(v8.get_f32("TORQUE_PEAK"), Some(520.0));
    assert!(v8.parent().is_none());

    let blob = bmw.get("SCRIPT").unwrap().as_blob().unwrap();
    assert_eq!(blob.wrapper(), ea_compress::Wrapper::Raww);
    assert_eq!(&*blob.decompress().unwrap(), b"hello");

    // The 0x8000 flag: items start 16 bytes after the array.
    assert_eq!(bmw.get_vector4("SPIN"), Some([1.0, 0.0, 0.0, 1.0]));
}

#[test]
fn names_come_from_strings() {
    let db = database();
    let names = db.names();
    assert_eq!(names.get(vlt_hash("bmw")), Some("bmw"));
    assert_eq!(names.get(vlt_hash("extra.bin")), Some("extra.bin"));
    assert_eq!(names.get(vlt_hash("car")), None);
    assert_eq!(names.display(vlt_hash("car")), format!("0x{:08X}", vlt_hash("car")));
    let mut db = db;
    db.names_mut().add_lines("car\n");
    assert_eq!(db.collection("car", "bmw").unwrap().class().key, vlt_hash("car"));
    assert_eq!(db.names().get(vlt_hash("car")), Some("car"));
}

#[test]
fn bare_vault_and_wrapped_pack() {
    let (name, vlt, bin) = db_vault();
    let mut db = crate::Database::new();
    db.load_vault(&name, &vlt, &bin).unwrap();
    assert_eq!(db.collection_count(), 3);

    let plain = pack(&[db_vault()]);
    let mut raww = b"RAWW\x01\x10\0\0".to_vec();
    raww.extend_from_slice(&[plain.len() as u32, plain.len() as u32 + 16].map(u32::to_le_bytes).concat());
    raww.extend_from_slice(&plain);
    assert_eq!(Database::open(&raww).unwrap().collection_count(), 3);
}

#[test]
fn errors() {
    assert!(matches!(Database::open(b"not a pack at all"), Err(Error::NotAttrib)));
    let mut truncated = pack(&[db_vault()]);
    truncated.truncate(0x200);
    assert!(matches!(Database::open(&truncated), Err(Error::Pack(_))));

    // Collections before their class: refused, and the database is unchanged.
    let mut db = Database::new();
    let err = db.load(&pack(&[extra_vault()])).unwrap_err();
    assert!(matches!(err, Error::UnknownClass { class, .. } if class == vlt_hash("car")), "{err}");
    assert_eq!((db.vaults().len(), db.collection_count()), (0, 0));

    db.load(&pack(&[db_vault()])).unwrap();
    assert!(matches!(db.load(&pack(&[db_vault()])), Err(Error::Duplicate { what: "class", .. })));
    assert_eq!((db.vaults().len(), db.collection_count()), (1, 3));
}

#[test]
fn unknown_layouts_are_refused() {
    // A 0x20-byte ClassLoadData, as in the 2006+ layout.
    let mut b = VaultBuilder::new("modern", &[]);
    b.export(vlt_hash("car"), super::fixture::CLASS_LOAD, &[0; 0x20]);
    let err = Database::open(&pack(&[b.finish()])).unwrap_err();
    assert!(matches!(err, Error::UnsupportedLayout { .. }), "{err}");

    // 16-byte export entries.
    let mut b = VaultBuilder::new("modern", &[]);
    b.export_entry_len = 16;
    for i in 0..4 {
        b.export(i, super::fixture::COLLECTION_LOAD, &[0; 0x20]);
    }
    let err = Database::open(&pack(&[b.finish()])).unwrap_err();
    assert!(matches!(err, Error::UnsupportedLayout { .. }), "{err}");
}
