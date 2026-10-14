//! The log trailer's damage specs, read from the install (`TRAILERLOG` and its parent `trailers` in `pvehicle`).
//! Spec: `docs/specs/trailer-logs.md`.

use blackbox_attrib::Value;
use nfsmw_data::car::physics::{DAMAGE_ZONES, PhysicsData};
use nfsmw_data::world::sequences::SequenceCatalog;

/// The `VisualScale` (the first float of each `DZ_*` record) of a `pvehicle` damage record, zone by zone.
fn visual_scales(data: &PhysicsData, name: &str) -> Option<[f32; 10]> {
    let damage = data.database().collection("pvehicle", &name.to_ascii_lowercase())?.follow("damagespecs")?;
    Some(DAMAGE_ZONES.map(|zone| match damage.get(zone) {
        Some(Value::Raw { bytes, .. }) => f32::from_le_bytes(bytes[..4].try_into().unwrap()),
        _ => f32::NAN,
    }))
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn trailerlog_damage_scales_leave_the_side_zones_at_zero() {
    let Some(dir) = crate::install() else { return };
    let mut data = PhysicsData::load(&dir).unwrap();
    data.set_sequences(SequenceCatalog::read(&crate::unwrapped(&dir, "TRACKS/L2RA.BUN")).unwrap());
    // `DZ_*` order: FRONT, REAR, LEFT, RIGHT, LFRONT, RFRONT, LREAR, RREAR, TOP, BOTTOM.
    let expected = [0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0];
    assert_eq!(visual_scales(&data, "TRAILERLOG"), Some(expected));
    assert_eq!(visual_scales(&data, "trailers"), Some(expected));
    // The event spec is present, so the damage path runs for the log trailer.
    let car = data.car("TRAILERLOG").unwrap();
    let spec = car.events.as_deref().expect("TRAILERLOG has no event spec");
    assert_eq!(spec.strength, 20000.0);
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn platform_crib_solids_parse_with_the_0x18_layout() {
    let Some(dir) = crate::install() else { return };
    let data = dir.read("FRONTEND/PLATFORMS/PlatformCrib.BIN").unwrap();
    let solids = blackbox_solid::read_solids(&data).unwrap();
    // Five solids (version 0x18 records), each with geometry.
    assert_eq!(solids.len(), 5);
    assert!(solids.iter().all(|s| !s.vertices.is_empty() && !s.indices.is_empty()));
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn damagespecs_layout_offsets_of_the_dz_fields() {
    let Some(dir) = crate::install() else { return };
    let data = PhysicsData::load(&dir).unwrap();
    let class = data.database().class("damagespecs").expect("no damagespecs class");
    let names = [
        "DZ_FRONT",
        "DZ_REAR",
        "DZ_LEFT",
        "DZ_RIGHT",
        "DZ_LFRONT",
        "DZ_RFRONT",
        "DZ_LREAR",
        "DZ_RREAR",
        "DZ_TOP",
        "DZ_BOTTOM",
    ];
    for name in names {
        let key = blackbox_attrib::vlt_hash(name);
        match class.field_by_key(key) {
            Some(f) => println!("{name}: offset {:#x} size {} in_layout {}", f.offset, f.size, f.in_layout()),
            None => println!("{name}: not a field of the class"),
        }
    }
    for f in class.layout_fields() {
        println!("layout field key {:#010x} offset {:#x} size {}", f.key, f.offset, f.size);
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn damage_both_floats_of_a_car_and_the_trailer() {
    let Some(dir) = crate::install() else { return };
    let data = PhysicsData::load(&dir).unwrap();
    for name in ["BMWM3GTR", "TRAILERLOG"] {
        let Some(damage) =
            data.database().collection("pvehicle", &name.to_ascii_lowercase()).and_then(|v| v.follow("damagespecs"))
        else {
            println!("{name}: no damagespecs");
            continue;
        };
        for zone in DAMAGE_ZONES {
            if let Some(Value::Raw { bytes, .. }) = damage.get(zone) {
                let f: Vec<f32> = bytes.as_chunks::<4>().0.iter().map(|c| f32::from_le_bytes(*c)).collect();
                println!("{name} {zone}: {f:?}");
            }
        }
    }
}
