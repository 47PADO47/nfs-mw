//! Against a real NFS: Most Wanted install (set `NFSMW_GAME_DIR`). Expected numbers are the ones
//! in `docs/formats/attributes.md`.

use std::collections::BTreeMap;

use crate::hash::vlt_hash;
use crate::{CollectionRef, Database, Value};

fn game_file(path: &str) -> Option<Vec<u8>> {
    let dir = std::env::var_os("NFSMW_GAME_DIR")?;
    let path = std::path::Path::new(&dir).join(path);
    Some(std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display())))
}

/// `(class, fields, collections in attributes.bin)`.
const CLASSES: &[(&str, usize, usize)] = &[
    ("acceltrans", 5, 28),
    ("aivehicle", 9, 23),
    ("aud_moment_strm", 6, 45),
    ("aud_stitch_loop", 2, 2),
    ("audioimpact", 7, 131),
    ("audioscrape", 2, 7),
    ("audiosystem", 20, 9),
    ("brakes", 3, 90),
    ("camerainfo", 9, 69),
    ("chassis", 19, 97),
    ("chopperspecs", 24, 3),
    ("collisionreactions", 4, 31),
    ("controller", 152, 21),
    ("damagespecs", 16, 14),
    ("ecar", 49, 100),
    ("effects", 20, 198),
    ("emitterdata", 45, 205),
    ("emittergroup", 4, 128),
    ("emitteruv", 4, 4),
    ("engine", 7, 89),
    ("engineaudio", 40, 70),
    ("explosion", 6, 1),
    ("fecooling", 11, 1),
    ("frontend", 55, 0),
    ("fuelcell_effect", 2, 4),
    ("fuelcell_emitter", 19, 8),
    ("gameplay", 220, 0),
    ("induction", 7, 69),
    ("infractions", 1, 8),
    ("junkman", 7, 1),
    ("light_flares_cg", 8, 22),
    ("milestonetypes", 4, 30),
    ("music", 5, 27),
    ("nos", 8, 6),
    ("ocean", 5, 1),
    ("presetride", 7, 25),
    ("pursuitescalation", 4, 1),
    ("pursuitlevels", 57, 21),
    ("pursuitsupport", 4, 21),
    ("pvehicle", 66, 121),
    ("rigidbodyspecs", 23, 22),
    ("shiftpattern", 24, 25),
    ("simsurface", 24, 47),
    ("smackable", 30, 181),
    ("speech", 28, 133),
    ("speechtune", 39, 1),
    ("system", 3, 1),
    ("timeofdaylighting", 13, 8),
    ("tires", 9, 95),
    ("trafficpattern", 5, 10),
    ("transmission", 9, 89),
    ("turbosfx", 6, 18),
    ("visuallook", 8, 5),
    ("visuallookeffect", 7, 7),
    ("visuallooktransition", 7, 1),
    ("visualrgbtweaker", 3, 1),
    ("world", 33, 1),
];

fn attributes() -> Option<Database> {
    let mut db = Database::open(&game_file("GLOBAL/attributes.bin")?).expect("attributes.bin");
    db.names_mut().extend(CLASSES.iter().map(|c| c.0));
    Some(db)
}

#[test]
#[ignore = "needs a game install; set NFSMW_GAME_DIR"]
fn real_install_attributes_bin() {
    let Some(db) = attributes() else { return };
    assert_eq!(db.vaults().len(), 1);
    assert_eq!(db.vaults()[0].name(), "db");
    assert_eq!(db.classes().len(), 57);
    assert_eq!(db.collection_count(), 2376);
    assert_eq!(db.types().len(), 94);
    for &(name, fields, collections) in CLASSES {
        let class = db.class(name).unwrap_or_else(|| panic!("class {name}"));
        assert_eq!(class.fields.len(), fields, "{name} fields");
        let searchable = class.layout_fields().filter(|f| f.flags.searchable()).count();
        assert_eq!(searchable, class.layout_count as usize, "{name} searchable layout fields");
        assert_eq!(db.collections_of(name).count(), collections, "{name} collections");
    }
    // In-layout arrays with the 16-byte alignment flag: ecar's TireOffsets (Vector4[4]).
    for c in db.collections_of("ecar") {
        let offsets = c.get("TireOffsets").and_then(Value::as_array).expect("TireOffsets");
        // (x, y, z, w) in metres (w looks like the tire radius); read 8 bytes early, the header would show.
        let plausible = |v: [f32; 4]| v[..3].iter().all(|x| x.abs() < 10.0) && (0.0..1.0).contains(&v[3]);
        assert!(offsets.len() == 4 && offsets.iter().all(|v| v.as_vector4().is_some_and(plausible)));
    }
    // Every parent is loaded, and every RefSpec to a class with collections here resolves.
    let (mut refs, mut resolved) = (0, 0);
    for c in db.collections() {
        assert!(c.data().parent.is_none() || c.parent().is_some(), "parent of 0x{:08X}", c.key());
        for value in c.attributes().iter().flat_map(|a| a.value.as_array().unwrap_or(std::slice::from_ref(&a.value))) {
            if let Some(r) = value.as_ref_spec().filter(|r| r.collection != 0) {
                refs += 1;
                resolved += usize::from(db.resolve(r).is_some());
            }
        }
    }
    println!("{refs} RefSpecs, {resolved} resolve within attributes.bin");
    assert!(resolved * 10 > refs * 9, "{resolved} of {refs} RefSpecs resolve");
}

/// AxlePair (front, rear): a game-specific 8-byte record, read raw.
fn axle_pair(c: &CollectionRef<'_>, field: &str) -> Option<[f32; 2]> {
    let raw = c.get(field)?.as_raw()?;
    let f = |o: usize| f32::from_le_bytes(raw[o..o + 4].try_into().unwrap());
    Some([f(0), f(4)])
}

#[test]
#[ignore = "needs a game install; set NFSMW_GAME_DIR"]
fn real_install_bmw_m3_gtr() {
    let Some(db) = attributes() else { return };
    let car = db.collection("pvehicle", "bmwm3gtr").expect("pvehicle/bmwm3gtr");
    let names = db.names();
    let lineage: Vec<_> = car.lineage().map(|c| names.display(c.key())).collect();
    println!("pvehicle/bmwm3gtr: lineage {lineage:?}, vault {}", car.vault().name());
    let mass = car.get_f32("MASS").expect("MASS");
    let model = car.get_string_key("MODEL").expect("MODEL");
    println!("  MASS {mass} kg, MODEL {:?}, TENSOR_SCALE {:?}", model.string, car.get_vector4("TENSOR_SCALE"));
    assert!((1000.0..2000.0).contains(&mass), "MASS {mass}");
    assert_eq!(model.hash32, vlt_hash(model.string.as_deref().unwrap_or_default()));

    for part in ["chassis", "engine", "tires", "transmission", "brakes", "induction", "nos"] {
        let levels = car.get(part).unwrap_or_else(|| panic!("{part}"));
        let refs: Vec<_> = (0..).map_while(|i| levels.item(i)?.as_ref_spec()).collect();
        assert!(!refs.is_empty(), "{part}");
        for r in &refs {
            assert_eq!(r.class, vlt_hash(part), "{part} refers to another class");
            assert!(db.resolve(*r).is_some(), "{part} 0x{:08X} does not resolve", r.collection);
        }
        let shown: Vec<_> = refs.iter().map(|r| names.display(r.collection)).collect();
        println!("  {part}: {shown:?}");
    }

    let engine = car.follow("engine").unwrap();
    let torque = engine.get("TORQUE").and_then(Value::as_array).expect("engine TORQUE");
    let (redline, max_rpm) = (engine.get_f32("RED_LINE").unwrap(), engine.get_f32("MAX_RPM").unwrap());
    println!("  engine: TORQUE {torque:?}, RED_LINE {redline}, MAX_RPM {max_rpm}");
    assert!(!torque.is_empty() && torque.iter().all(|t| t.as_f32().is_some_and(|t| t > 0.0)));
    assert!((3000.0..=max_rpm).contains(&redline));

    let transmission = car.follow("transmission").unwrap();
    let ratios = transmission.get("GEAR_RATIO").and_then(Value::as_array).unwrap();
    println!("  transmission: GEAR_RATIO {ratios:?}, FINAL_GEAR {:?}", transmission.get_f32("FINAL_GEAR"));
    assert!(ratios.len() >= 6);

    let chassis = car.follow("chassis").unwrap();
    let tires = car.follow("tires").unwrap();
    println!(
        "  chassis: WHEEL_BASE {:?}, FRONT_WEIGHT_BIAS {:?}, RIDE_HEIGHT {:?}; tires: GRIP_SCALE {:?}, RIM_SIZE {:?}",
        chassis.get_f32("WHEEL_BASE"),
        chassis.get_f32("FRONT_WEIGHT_BIAS"),
        axle_pair(&chassis, "RIDE_HEIGHT"),
        axle_pair(&tires, "GRIP_SCALE"),
        axle_pair(&tires, "RIM_SIZE"),
    );
    assert!(chassis.get_f32("WHEEL_BASE").is_some_and(|w| (2.0..3.5).contains(&w)));
    assert!(
        axle_pair(&tires, "RIM_SIZE").is_some_and(|[f, r]| (15.0..=20.0).contains(&f) && (15.0..=20.0).contains(&r))
    );
}

/// A Lua 5.0 chunk header: `ESC "Lua"`, version 5.0, little-endian, sizes of int, size_t and
/// Instruction, instruction field widths, a 4-byte `lua_Number`, and the test number 3.14159265e7.
const LUA50_HEADER: &[u8] = b"\x1bLua\x50\x01\x04\x04\x04\x06\x08\x09\x09\x04\x3B\xAF\xEF\x4B";

#[test]
#[ignore = "needs a game install; set NFSMW_GAME_DIR"]
fn real_install_gameplay_and_frontend() {
    let Some(mut db) = attributes() else { return };
    db.names_mut().extend(["bytecode", "FilterBlocks"]);
    let gameplay = game_file("GLOBAL/gameplay.bin").unwrap();
    assert_eq!(db.load(&gameplay).unwrap(), 272);
    assert_eq!(db.collections_of("gameplay").count(), 6293);
    assert_eq!(db.load(&game_file("GLOBAL/FE_ATTRIB.bin").unwrap()).unwrap(), 1);
    assert_eq!(db.collections_of("frontend").count(), 446);
    assert_eq!(db.collection_count(), 2376 + 6293 + 446);
    // With all three packs loaded, every parent and every non-null RefSpec resolves.
    let mut refs = 0;
    for c in db.collections() {
        assert!(c.data().parent.is_none() || c.parent().is_some(), "parent of 0x{:08X}", c.key());
        for a in c.attributes() {
            let values = a.value.as_array().unwrap_or(std::slice::from_ref(&a.value));
            for r in values.iter().filter_map(Value::as_ref_spec).filter(|r| r.collection != 0) {
                assert!(db.resolve(r).is_some(), "RefSpec 0x{:08X}", r.collection);
                refs += 1;
            }
        }
    }
    assert_eq!(refs, 2029);

    // Blobs: all `gameplay` `bytecode` attributes of the `gpcore` vault, holding compiled Lua 5.0
    // chunks (state-graph handlers); all decompress.
    let mut blobs = BTreeMap::new();
    for c in db.collections() {
        for a in c.attributes() {
            for blob in a.value.as_array().unwrap_or(std::slice::from_ref(&a.value)).iter().filter_map(Value::as_blob) {
                let out = blob.decompress().unwrap_or_else(|e| panic!("blob of 0x{:08X}: {e}", c.key()));
                assert!(out.is_empty() || out.starts_with(LUA50_HEADER), "{}", db.names().display(c.key()));
                let key = (c.vault().name(), db.names().display(a.key), format!("{:?}", blob.wrapper()));
                let e = blobs.entry(key).or_insert((0, 0, 0));
                *e = (e.0 + 1, e.1 + blob.data.len(), e.2 + out.len());
            }
        }
    }
    println!("blobs (vault, field, wrapper) -> (count, stored bytes, unpacked bytes): {blobs:?}");
    let count = |wrapper: &str| blobs.iter().filter(|(k, _)| k.2 == wrapper).map(|(_, v)| v.0).sum::<usize>();
    assert_eq!((count("Huff"), count("Jdlz"), count("None")), (253, 5, 1));
    assert!(blobs.keys().all(|(vault, field, _)| *vault == "gpcore" && field == "bytecode"));

    // The RAWW-wrapped copy holds the same pack.
    let mut fresh = attributes().unwrap();
    assert_eq!(fresh.load(&game_file("GLOBAL/gameplay.lzc").unwrap()).unwrap(), 272);
    assert_eq!(fresh.collections_of("gameplay").count(), 6293);
}

/// The 2005 originals that mod tools leave as `*.bak` (skipped when absent).
#[test]
#[ignore = "needs a game install; set NFSMW_GAME_DIR"]
fn real_install_original_backups() {
    let Some(attributes) = game_file_if_present("GLOBAL/attributes.bak") else { return };
    let db = Database::open(&attributes).expect("attributes.bak");
    assert_eq!((db.classes().len(), db.collection_count()), (57, 2309));
    for (file, vaults, collections) in
        [("fe_attrib.bak", 1, 411), ("gameplay.bak", 272, 6293), ("gameplay.lzc.bak", 272, 6293)]
    {
        let Some(data) = game_file_if_present(&format!("GLOBAL/{file}")) else { continue };
        let mut db = db.clone();
        assert_eq!(db.load(&data).unwrap_or_else(|e| panic!("{file}: {e}")), vaults, "{file}");
        assert_eq!(db.collection_count() - 2309, collections, "{file}");
    }
}

fn game_file_if_present(path: &str) -> Option<Vec<u8>> {
    let path = std::path::Path::new(&std::env::var_os("NFSMW_GAME_DIR")?).join(path);
    std::fs::read(path).ok()
}
