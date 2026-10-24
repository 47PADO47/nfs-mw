//! Native vinyl and gauge palette parts. Contract: `docs/specs/garage-game-ui.md`.

use blackbox_carparts::CarTypeInfo;

use super::super::{CarTables, LAYOUT};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColorChoice {
    pub hash: u32,
    pub rgb: [u8; 3],
    pub upgrade_level: u8,
}

/// Vinyl remap channels 0..2, then gauge backing, needle and characters.
pub(super) fn read(tables: &CarTables, car: &CarTypeInfo) -> [Vec<ColorChoice>; 6] {
    std::array::from_fn(|index| {
        let slot = [79, 80, 81, 133, 134, 135][index];
        let compatible: Vec<_> = tables.slot_types.search_types(slot, car.type_name_hash).collect();
        compatible
            .into_iter()
            .flat_map(|type_hash| {
                tables.parts.parts().iter().filter(move |part| tables.parts.type_name_hash(part) == type_hash)
            })
            .filter_map(|part| {
                if part.part_id != LAYOUT.slots[slot].part_id {
                    return None;
                }
                if index < 3 && tables.parts.attribute(part, "BRAND_NAME") != Some(0x03e8_71f1) {
                    return None;
                }
                let channels = ["RED", "GREEN", "BLUE"].map(|name| tables.parts.attribute(part, name));
                let [Some(red), Some(green), Some(blue)] = channels else { return None };
                Some(ColorChoice {
                    hash: part.name_hash,
                    rgb: [red, green, blue].map(|value| value.min(255) as u8),
                    upgrade_level: part.upgrade_level,
                })
            })
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires an installed game"]
    fn installed_vinyl_and_gauge_palettes_keep_native_slot_parts() {
        let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
        let tables = CarTables::load(&dir).unwrap();
        let car = tables.car_type_for_folder("TT").unwrap();
        let palettes = read(&tables, car);
        for (index, palette) in palettes.iter().enumerate() {
            assert!(!palette.is_empty(), "native color palette {index} must not use body-paint fallback");
            let slot = [79, 80, 81, 133, 134, 135][index];
            for choice in palette {
                let part = tables.parts.by_name_hash(choice.hash).unwrap();
                assert_eq!(part.part_id, LAYOUT.slots[slot].part_id);
                if index < 3 {
                    assert_eq!(tables.parts.attribute(part, "BRAND_NAME"), Some(0x03e8_71f1));
                }
                assert_eq!(
                    choice.rgb,
                    ["RED", "GREEN", "BLUE"].map(|name| { tables.parts.attribute(part, name).unwrap().min(255) as u8 })
                );
            }
        }
        println!("Native vinyl/gauge palette counts: {:?}", palettes.each_ref().map(Vec::len));
    }
}
