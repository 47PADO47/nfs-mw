//! The paint packs: the colours and the liveries each vehicle of the vehicle list is offered in. Spec:
//! `docs/formats/chameleon-customization.md`.
//!
//! `GAMELOGIC/GAMEPLAY.BNDL` holds one Genesys object with a `Genesys.Gen.PaintPackGroup` per paint pack group, and the
//! group has one `Genesys.Gen.PaintPack` per choice the shop offers, in the order the shop lists them. A vehicle names its
//! group in its vehicle list entry. The structures sit inline in the object, and the pointers between them are offsets in
//! its main block.

use chameleon_bundle::{Bundle, ResourceKind};

use crate::error::{Error, Result};
use crate::genesys::{Object, TypeNames, find_object};

/// The name the engine gives the group's type.
const GROUP_TYPE: &str = "Genesys.Gen.PaintPackGroup";
/// Fields of a group: its id, the count of packs, and the table of offsets of the packs.
const GROUP_ID: usize = 0x0C;
const GROUP_COUNT: usize = 0x14;
const GROUP_TABLE: usize = 0x18;
/// Fields of a pack, six words from +0x0C.
const PACK_FIELDS: usize = 0x0C;
/// More packs than this is not a group (the install's largest has 9).
const MAX_PACKS: usize = 256;

/// One choice of a vehicle: a colour, or a livery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaintPack {
    /// The id of the `VehiclePaint` record of a plain colour (+0x0C), which the vehicle's bundle holds; 0 for a livery
    /// pack and for the two-colour packs of nine vehicles **[verified]**: all 273 plain colour packs of the install name a
    /// record of their vehicle's bundle. (The packs of three more vehicles name a plain colour and a pair.)
    pub paint: u32,
    /// +0x10: 0 on a pack with no pair of colours; else the id of a `Genesys.Gen.ColourGroup` of the vehicle's bundle, the
    /// pair of paints of a two-colour pack (`crate::colour_groups`, [`PaintPackGroup::two_colour_packs`]) **[verified]**:
    /// 62 packs of 12 vehicles name a group of their vehicle's bundle.
    pub two_paint: u32,
    /// +0x14: the pack's own number, in sequence within a group **[unconfirmed]**.
    pub id: u32,
    /// +0x18: an image id, the pack's swatch **[unconfirmed]**.
    pub swatch: u32,
    /// +0x1C: for a livery pack, the id of a Genesys object that is in the bundle of that livery and in no other
    /// **[verified]**: it names the bundle of the livery. 0 on a colour pack.
    pub livery: u32,
    /// +0x20: the string id of the pack's name in the text tables (`HERO`, `SILVER METALLIC`) **[verified]**.
    pub name: u32,
}

/// The packs of one paint pack group, in the order the shop lists them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaintPackGroup {
    /// The group's id, which a vehicle list entry names (+0x34 of the entry).
    pub id: u32,
    pub packs: Vec<PaintPack>,
}

impl PaintPackGroup {
    /// The pack of the livery whose bundle is `bundle`: the one whose [`PaintPack::livery`] object the bundle holds.
    pub fn livery_of(&self, bundle: &Bundle) -> Option<&PaintPack> {
        let object = ResourceKind::GenesysObject.id();
        let holds = |id: u32| bundle.entries().iter().any(|entry| entry.kind == object && entry.id as u32 == id);
        self.packs.iter().filter(|pack| pack.livery != 0).find(|pack| holds(pack.livery))
    }

    /// The ids of the `VehiclePaint` records of the plain colours, in the order the shop lists them.
    pub fn paint_order(&self) -> Vec<u32> {
        self.packs.iter().map(|pack| pack.paint).filter(|&paint| paint != 0).collect()
    }
}

/// Every paint pack group of the gameplay bundle `gameplay` (`GAMELOGIC/GAMEPLAY.BNDL`).
pub fn paint_packs(gameplay: &Bundle) -> Result<Vec<PaintPackGroup>> {
    let types = TypeNames::read(gameplay)?;
    let object = find_object(gameplay, &types, GROUP_TYPE)?;
    object.all(GROUP_TYPE).map(|at| read_group(&object, at)).collect()
}

fn read_group(object: &Object, at: usize) -> Result<PaintPackGroup> {
    let count = object.u32(at + GROUP_COUNT)? as usize;
    if count > MAX_PACKS {
        return Err(Error::Spec(format!("a paint pack group claims {count} packs")));
    }
    let mut packs = Vec::with_capacity(count);
    for n in 0..count {
        let pack = object.u32(at + GROUP_TABLE + 4 * n)? as usize;
        let field = |n: usize| object.u32(pack + PACK_FIELDS + 4 * n);
        packs.push(PaintPack {
            paint: field(0)?,
            two_paint: field(1)?,
            id: field(2)?,
            swatch: field(3)?,
            livery: field(4)?,
            name: field(5)?,
        });
    }
    Ok(PaintPackGroup { id: object.u32(at + GROUP_ID)?, packs })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put(block: &mut [u8], at: usize, words: &[u32]) {
        for (n, word) in words.iter().enumerate() {
            block[at + 4 * n..at + 4 * n + 4].copy_from_slice(&word.to_le_bytes());
        }
    }

    /// An object with one group at 0x40 of id 7, whose two packs are at 0x80 and 0xB0.
    fn object() -> Object {
        let mut main = vec![0u8; 0x100];
        put(&mut main, 0x40 + GROUP_ID, &[7]);
        put(&mut main, 0x40 + GROUP_COUNT, &[2, 0x80, 0xB0]);
        put(&mut main, 0x80 + PACK_FIELDS, &[0x12_4628, 0, 0x18_3ffe, 0x1b_8b59, 0, 0x1b_8ccf]);
        put(&mut main, 0xB0 + PACK_FIELDS, &[0, 0, 0x22_bf93, 0x22_bf98, 0x22_bf84, 0x22_6c7c]);
        Object { main, structs: vec![(GROUP_TYPE.to_owned(), 0x40)] }
    }

    #[test]
    fn a_group_reads_its_packs_in_order() {
        let object = object();
        let group = read_group(&object, 0x40).unwrap();
        assert_eq!(group.id, 7);
        assert_eq!(group.packs.len(), 2);
        assert_eq!(group.packs[0].paint, 0x12_4628);
        assert_eq!(group.packs[0].name, 0x1b_8ccf);
        assert_eq!((group.packs[1].livery, group.packs[1].name), (0x22_bf84, 0x22_6c7c));
        assert_eq!(group.paint_order(), vec![0x12_4628], "a livery pack is no colour");
    }

    #[test]
    fn a_group_that_claims_too_much_or_is_cut_short_is_refused() {
        let mut object = object();
        put(&mut object.main, 0x40 + GROUP_COUNT, &[u32::MAX]);
        assert!(read_group(&object, 0x40).is_err());
        put(&mut object.main, 0x40 + GROUP_COUNT, &[2, 0x80, 0x1000]);
        assert!(read_group(&object, 0x40).is_err(), "a pack past the end of the block");
    }
}
