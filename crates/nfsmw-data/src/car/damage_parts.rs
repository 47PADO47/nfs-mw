//! Installed police replacement parts. Spec: `docs/specs/cop-visible-damage.md`.
use super::{CarTables, LodAssembly, Slots};
use blackbox_carparts::{CarTypeInfo, Part};

#[derive(Default)]
pub struct DamageParts {
    pub enabled: bool,
    pub levels: Vec<LodAssembly>,
}

pub fn supported(name: &str) -> bool {
    [
        "COPMIDSIZE",
        "COPSUV",
        "COPSPORT",
        "COPGHOST",
        "COPGTO",
        "COPSUVL",
        "COPGTOGHOST",
        "COPSPORTHENCH",
        "COPSPORTGHOST",
    ]
    .iter()
    .any(|ty| name.eq_ignore_ascii_case(ty))
}

/// Start after the selected part, continuing through the slot's remaining types.
fn successor(tables: &CarTables, car: &CarTypeInfo, slot: usize, current: Part) -> Option<Part> {
    let types: Vec<_> = tables.slot_types.search_types(slot, car.type_name_hash).collect();
    let start_type = types.iter().position(|&ty| ty == tables.parts.type_name_hash(&current))?;
    let start_part = tables.parts.parts().iter().position(|&part| part == current)? + 1;
    types.iter().enumerate().skip(start_type).find_map(|(index, &ty)| {
        let begin = if index == start_type { start_part } else { 0 };
        tables.parts.parts()[begin..]
            .iter()
            .find(|part| part.part_id == super::LAYOUT.slots[slot].part_id && tables.parts.type_name_hash(part) == ty)
            .copied()
    })
}

pub(super) fn slots(tables: &CarTables, car: &CarTypeInfo, pristine: &Slots) -> Option<Slots> {
    if !supported(&car.type_name) {
        return None;
    }
    let mut damaged = pristine.clone();
    for slot in 0..=22 {
        damaged[slot] = pristine[slot].and_then(|part| successor(tables, car, slot, part));
    }
    Some(damaged)
}

/// Original slot membership after taking the maximum across all affected zones.
pub fn damaged_slots(levels: &[u8; 10]) -> u32 {
    let mut mask = 0;
    for (zone, &level) in levels.iter().enumerate() {
        if level == 0 || zone == 9 {
            continue;
        }
        for slot in [1, 2, 6, 7, 8, 9, 17, 18, 19, 20] {
            mask |= 1 << slot;
        }
        let extra: &[usize] = match zone {
            0 => &[3, 10, 11, 12],
            1 => &[15, 16],
            2 => &[3, 21, 22],
            3 => &[3, 13, 14],
            4 | 5 => &[10],
            6 | 7 => &[15],
            8 => &[3, 10, 11, 13, 14, 15, 21, 22],
            _ => &[],
        };
        for &slot in extra {
            mask |= 1 << slot;
        }
    }
    mask
}

/// The five window zones plus the rear defroster share their original damaged texture.
pub fn glass_mask(levels: &[u8; 10]) -> u16 {
    if damaged_slots(levels) == 0 {
        return 0;
    }
    (1 << 0) | (1 << 4) | (1 << 5) | (1 << 6) | (1 << 7)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn front_damage_selects_authored_panels_without_affecting_doors_or_rear_bumper() {
        let mut levels = [0; 10];
        levels[0] = 1;
        let mask = damaged_slots(&levels);
        for slot in [1, 2, 3, 6, 7, 8, 9, 10, 11, 12, 17, 18, 19, 20] {
            assert_ne!(mask & (1 << slot), 0);
        }
        assert_eq!(mask & ((1 << 13) | (1 << 15) | (1 << 16) | (1 << 21)), 0);
        levels[0] = 6;
        assert_eq!(damaged_slots(&levels), mask);
        levels = [0; 10];
        levels[9] = 6;
        assert_eq!((damaged_slots(&levels), glass_mask(&levels)), (0, 0));
        assert!(!supported("BMWM3GTR"));
    }
}
