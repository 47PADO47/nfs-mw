//! The colour groups of a car: the pairs of paints its two-colour paint packs name. Spec:
//! `docs/formats/chameleon-customization.md`.
//!
//! Every car bundle holds a `Genesys.Gen.VehicleColourPalette` object. For a car with a two-colour paint pack, the
//! palette also holds one `Genesys.Gen.ColourGroup` structure per pack (and a few more that no pack names), laid out
//! inline in the object like the other Genesys structures. A group has a first and a second paint, each a handle import
//! of a `VehiclePaint` record of the bundle, and its own id, which is what the pack's [`PaintPack::two_paint`] holds.

use std::collections::HashMap;

use chameleon_bundle::{Bundle, ResourceKind};

use crate::error::Result;
use crate::genesys::TypeNames;
use crate::paint::VehiclePaint;
use crate::paint_pack::{PaintPack, PaintPackGroup};

/// The name the engine gives the group's type.
const GROUP_TYPE: &str = "Genesys.Gen.ColourGroup";
/// Fields of a group: the handle of its first paint, the handle of its second paint, and its id.
const FIRST: usize = 0x0C;
const SECOND: usize = 0x14;
const ID: usize = 0x24;
/// The top bit of an import slot offset: the slot holds a handle, not a pointer.
const HANDLE: u32 = 0x8000_0000;

/// A pair of paints of a car, the colours of a two-colour pack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColourGroup {
    /// The group's id, which a pack names in [`PaintPack::two_paint`] **[verified]**: 62 packs of 12 vehicles name a group
    /// of their vehicle's bundle.
    pub id: u32,
    /// The `VehiclePaint` id (the paint's own number, `VehiclePaint::id`) of the first paint (+0x0C): the colour the
    /// body takes where the second does not **[verified]**; 0 when the slot holds no handle.
    pub first: u32,
    /// The id of the second paint (+0x14): the colour the surfaces that draw the second colour take **[verified]**.
    pub second: u32,
}

/// The pair of colours a two-colour pack gives its vehicle, linear RGB.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TwoColourPack {
    /// The string id of the pack's name in the text tables.
    pub name: u32,
    pub first: [f32; 3],
    pub second: [f32; 3],
}

/// Every colour group of the car bundle `bundle`, in the order they sit in the palette; none when it has none.
pub fn colour_groups(bundle: &Bundle) -> Result<Vec<ColourGroup>> {
    let types = TypeNames::read(bundle)?;
    let object = ResourceKind::GenesysObject.id();
    let mut out = Vec::new();
    for (index, entry) in bundle.entries().iter().enumerate() {
        if entry.kind != object || entry.import_count == 0 {
            continue;
        }
        let resource = bundle.resource(index)?;
        let mut structs = Vec::new();
        let mut handles = HashMap::new();
        for import in &resource.imports {
            let at = import.offset & !HANDLE;
            match types.name(import.id) {
                Some(GROUP_TYPE) => structs.push(at as usize),
                Some(_) => {}
                None => {
                    handles.insert(at as usize, import.id as u32);
                }
            }
        }
        structs.sort_unstable();
        out.extend(read_groups(&resource.blocks[0], &structs, &handles));
    }
    Ok(out)
}

/// The groups at `structs` (offsets in the object's main block `main`), their paint handles being `handles` (slot offset
/// to the key of the paint's resource id). A structure cut short by the end of the block is left out.
fn read_groups(main: &[u8], structs: &[usize], handles: &HashMap<usize, u32>) -> Vec<ColourGroup> {
    let word = |at: usize| -> Option<u32> {
        let bytes = main.get(at..at.checked_add(4)?)?;
        Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    };
    let paint = |at: usize| handles.get(&at).copied().unwrap_or(0);
    structs
        .iter()
        .filter_map(|&at| {
            Some(ColourGroup { id: word(at + ID)?, first: paint(at + FIRST), second: paint(at + SECOND) })
        })
        .collect()
}

impl PaintPackGroup {
    /// The two-colour packs of this group, one entry per pack in the shop's order: the pair of colours of the pack that
    /// names a group of `groups` whose paints are among `paints` (the car's `VehiclePaint` records); `None` for a pack that
    /// names none (a plain colour, a livery), or whose group has a paint the car lacks.
    pub fn two_colour_packs(&self, groups: &[ColourGroup], paints: &[VehiclePaint]) -> Vec<Option<TwoColourPack>> {
        self.packs.iter().map(|pack| two_colour(pack, groups, paints)).collect()
    }
}

fn two_colour(pack: &PaintPack, groups: &[ColourGroup], paints: &[VehiclePaint]) -> Option<TwoColourPack> {
    if pack.two_paint == 0 {
        return None;
    }
    let group = groups.iter().find(|group| group.id == pack.two_paint)?;
    let colour = |id: u32| paints.iter().find(|paint| paint.id == id).map(|paint| paint.diffuse);
    Some(TwoColourPack { name: pack.name, first: colour(group.first)?, second: colour(group.second)? })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put(block: &mut [u8], at: usize, value: u32) {
        block[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }

    /// A block with two groups, at 0x20 (id 0x10E35F) and 0x48 (id 0x10E360, no second paint).
    fn object() -> (Vec<u8>, Vec<usize>, HashMap<usize, u32>) {
        let mut main = vec![0u8; 0x80];
        put(&mut main, 0x20 + ID, 0x10_E35F);
        put(&mut main, 0x48 + ID, 0x10_E360);
        let handles = HashMap::from([(0x20 + FIRST, 0x12_462A), (0x20 + SECOND, 0x16_3181), (0x48 + FIRST, 0x12_462B)]);
        (main, vec![0x20, 0x48], handles)
    }

    fn paint(id: u32, diffuse: [f32; 3]) -> VehiclePaint {
        VehiclePaint { id, diffuse, flake: [0.0; 3], secondary: [0.0; 3], kind: 0, swatch: 0 }
    }

    fn pack(two_paint: u32, name: u32) -> PaintPack {
        PaintPack { paint: 0, two_paint, id: 1, swatch: 0, livery: 0, name }
    }

    #[test]
    fn a_group_reads_its_id_and_its_two_paints() {
        let (main, structs, handles) = object();
        let found = read_groups(&main, &structs, &handles);
        assert_eq!(found[0], ColourGroup { id: 0x10_E35F, first: 0x12_462A, second: 0x16_3181 });
        assert_eq!(found[1], ColourGroup { id: 0x10_E360, first: 0x12_462B, second: 0 }, "a missing handle is 0");
    }

    #[test]
    fn a_structure_past_the_end_of_the_block_is_left_out() {
        let (main, _, handles) = object();
        assert!(read_groups(&main, &[0x7C], &handles).is_empty());
    }

    #[test]
    fn a_pack_takes_the_colours_of_the_group_it_names() {
        let (main, structs, handles) = object();
        let groups = read_groups(&main, &structs, &handles);
        let paints = [paint(0x12_462A, [0.4, 0.3, 0.2]), paint(0x16_3181, [0.1, 0.1, 0.1])];
        let packs =
            PaintPackGroup { id: 1, packs: vec![pack(0x10_E35F, 7), pack(0, 8), pack(0x10_E360, 9), pack(5, 10)] };
        let found = packs.two_colour_packs(&groups, &paints);
        assert_eq!(found[0], Some(TwoColourPack { name: 7, first: [0.4, 0.3, 0.2], second: [0.1, 0.1, 0.1] }));
        assert_eq!(found[1], None, "a pack with no two-colour number is a plain colour");
        assert_eq!(found[2], None, "a group with a paint the car lacks");
        assert_eq!(found[3], None, "a number no group has");
    }
}
