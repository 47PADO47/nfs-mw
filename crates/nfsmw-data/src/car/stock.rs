//! Which part goes in each slot of a stock car, and preset overrides.
//! Spec: docs/specs/car-assembly.md §1.

use blackbox_carparts::{CarTypeInfo, Part, PartQuery, PresetRide};
use blackbox_hash::{bstring_hash, bstring_hash_continue};

use super::tables::{CarTables, LAYOUT, slot};

/// One part (or nothing) per slot.
pub type Slots = Vec<Option<Part>>;

/// Default vinyl colours and HUD colours of a stock car, by slot.
const VINYL_COLOURS: [&str; 4] = ["VINYL_L1_COLOR01", "VINYL_L1_COLOR03", "VINYL_L2_COLOR11", "VINYL_L1_COLOR01"];
const HUD_COLOURS: [&str; 3] = ["ORANGE", "ORANGE", "WHITE"];

/// The stock car of `car`.
pub fn stock_parts(t: &CarTables, car: &CarTypeInfo) -> Slots {
    let mut slots: Slots = (0..LAYOUT.slots.len())
        .map(|s| match s {
            slot::REAR_WHEEL | slot::VINYL_LAYER => None,
            s if slot::DECAL_TEXTURES.contains(&s) => None,
            slot::BASE_PAINT => find(t, car, s, Some(car.default_base_paint), None),
            s if slot::VINYL_COLOURS.contains(&s) => {
                let name = VINYL_COLOURS[s - slot::VINYL_COLOURS.start()];
                find(t, car, s, Some(bstring_hash(name)), None)
            }
            s if slot::HUD_COLOURS.contains(&s) => {
                let name = HUD_COLOURS[s - slot::HUD_COLOURS.start()];
                find(t, car, s, Some(bstring_hash(name)), None)
            }
            s => find(t, car, s, None, Some(0)),
        })
        .collect();
    fill_from_body(t, car, &mut slots);
    slots
}

/// The stock car with a preset's parts on top: a part hash replaces the stock part,
/// [`PresetRide::EMPTY`] clears the slot and [`PresetRide::STOCK`] keeps it.
pub fn preset_parts(t: &CarTables, car: &CarTypeInfo, preset: &PresetRide) -> Slots {
    let mut slots = stock_parts(t, car);
    for (s, &hash) in preset.parts.iter().enumerate().take(slots.len()) {
        match hash {
            PresetRide::STOCK => {}
            PresetRide::EMPTY => slots[s] = None,
            hash => slots[s] = find(t, car, s, Some(hash), None),
        }
    }
    fill_from_body(t, car, &mut slots);
    slots
}

/// The body decides the corner damage overlays and the door/quarter decal models:
/// parts named `<CAR>_KIT<nn>_<suffix>` with `nn` the body's `KITNUMBER`.
fn fill_from_body(t: &CarTables, car: &CarTypeInfo, slots: &mut Slots) {
    let kit = slots[slot::BODY].map(|body| kit_number(t, &body));
    let prefix = kit.map(|kit| bstring_hash(&format!("{}_KIT{kit:02}_", car.type_name)));
    for s in slot::DAMAGE0.chain(slot::KIT_DECALS) {
        let name = LAYOUT.slots[s].name;
        slots[s] = prefix.and_then(|prefix| {
            let suffix = if slot::KIT_DECALS.contains(&s) { format!("{name}_RECT_MEDIUM") } else { name.to_owned() };
            find(t, car, s, Some(bstring_hash_continue(prefix, suffix.as_bytes())), None)
        });
    }
}

/// A body part's kit number (0 = stock body).
pub fn kit_number(t: &CarTables, body: &Part) -> u32 {
    t.parts.attribute(body, "KITNUMBER").unwrap_or(0)
}

/// The first part for `slot` of `car`, searching the slot's types in order.
fn find(t: &CarTables, car: &CarTypeInfo, slot: usize, name_hash: Option<u32>, level: Option<u8>) -> Option<Part> {
    let part_id = LAYOUT.slots[slot].part_id;
    t.slot_types
        .search_types(slot, car.type_name_hash)
        .find_map(|type_hash| t.parts.find(&PartQuery { part_id, type_hash, name_hash, upgrade_level: level }).copied())
}
