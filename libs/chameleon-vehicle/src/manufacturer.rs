//! The manufacturers of the vehicle list (`VEHICLES/VEHICLELIST.BNDL`). Spec: `docs/formats/chameleon-manufacturers.md`.
//!
//! The list resource's main block starts with a 0x20-byte header: the entry count at +0x04, the offset of the entries at
//! +0x0C, the number of manufacturers at +0x08 and the offset of their table at +0x10. The table holds one record of five
//! words per manufacturer. The names are string ids of the language tables (`UI/LANGUAGE/NNNN.BNDL`).

use chameleon_bundle::{Bundle, ResourceKind};

/// Bytes of one manufacturer record.
const STRIDE: usize = 20;
/// More manufacturers than this is not a vehicle list (the install has 36).
const MAX_MANUFACTURERS: usize = 1024;

/// One manufacturer of the vehicle list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Manufacturer {
    /// The id the vehicle list entries name in their `ManufacturerId` field (+0x14 of an entry).
    pub id: u32,
    /// The string id of the manufacturer's name in the language tables.
    pub name: u32,
    /// The string id of a second text of the record (the region).
    pub region: u32,
    /// The numeric key of the logo's texture (a [`chameleon_bundle::IdClass::GameChanger`] id), 0 when there is none.
    pub logo: u32,
    /// The key of a second picture of the record, 0 when there is none.
    pub picture: u32,
}

/// The manufacturers of the vehicle list bundle `data`. `None` when it is not a vehicle list.
pub fn manufacturers(data: &[u8]) -> Option<Vec<Manufacturer>> {
    let bundle = Bundle::parse(data).ok()?;
    let kind = ResourceKind::VehicleList.id();
    let index = bundle.entries().iter().position(|entry| entry.kind == kind)?;
    parse_manufacturers(&bundle.resource(index).ok()?.blocks[0])
}

/// The manufacturers of the list resource's main block.
pub fn parse_manufacturers(block: &[u8]) -> Option<Vec<Manufacturer>> {
    let word = |at: usize| -> Option<u32> {
        let bytes = block.get(at..at.checked_add(4)?)?;
        Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    };
    let count = word(8)? as usize;
    let first = word(0x10)? as usize;
    let end = first.checked_add(count.checked_mul(STRIDE)?)?;
    if count > MAX_MANUFACTURERS || end > block.len() {
        return None;
    }
    (0..count)
        .map(|n| {
            let at = first + n * STRIDE;
            Some(Manufacturer {
                id: word(at)?,
                name: word(at + 4)?,
                region: word(at + 8)?,
                logo: word(at + 12)?,
                picture: word(at + 16)?,
            })
        })
        .collect()
}

/// The text with string id `id` in the language table `block` (the main block of a `LocalisedText` resource: language,
/// string count, offset of the sorted ids, offset of the `(text offset, UTF-16 units with the NUL)` records). `None` when
/// the id is not there or the block is cut short. The first of several entries with one id wins.
pub fn localised_text(block: &[u8], id: u32) -> Option<String> {
    let word = |at: usize| -> Option<usize> {
        let bytes = block.get(at..at.checked_add(4)?)?;
        Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize)
    };
    let (count, ids, records) = (word(4)?, word(8)?, word(12)?);
    let (mut low, mut high) = (0, count);
    while low < high {
        let mid = low + (high - low) / 2;
        if word(ids.checked_add(mid.checked_mul(4)?)?)? < id as usize {
            low = mid + 1;
        } else {
            high = mid;
        }
    }
    if low >= count || word(ids.checked_add(low.checked_mul(4)?)?)? != id as usize {
        return None;
    }
    let record = records.checked_add(low.checked_mul(8)?)?;
    let (text, units) = (word(record)?, word(record + 4)?);
    let raw = block.get(text..text.checked_add(units.checked_sub(1)?.checked_mul(2)?)?)?;
    let units: Vec<u16> = raw.as_chunks::<2>().0.iter().map(|pair| u16::from_le_bytes(*pair)).collect();
    String::from_utf16(&units).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put(block: &mut Vec<u8>, values: &[u32]) {
        for value in values {
            block.extend_from_slice(&value.to_le_bytes());
        }
    }

    /// A list block with `records` after one empty-entries header.
    fn list(records: &[[u32; 5]]) -> Vec<u8> {
        let mut block = Vec::new();
        put(&mut block, &[0, 0, records.len() as u32, 0x20, 0x20, 0, 0, 0]);
        for record in records {
            put(&mut block, record);
        }
        block
    }

    #[test]
    fn records_are_read_in_order() {
        let found = parse_manufacturers(&list(&[[1, 2, 3, 4, 5], [6, 7, 8, 0, 0]])).unwrap();
        assert_eq!(found.len(), 2);
        assert_eq!(found[0], Manufacturer { id: 1, name: 2, region: 3, logo: 4, picture: 5 });
        assert_eq!(found[1].logo, 0);
    }

    #[test]
    fn a_table_that_is_cut_short_or_claims_too_much_is_refused() {
        let mut block = list(&[[1, 2, 3, 4, 5]]);
        block.truncate(block.len() - 1);
        assert!(parse_manufacturers(&block).is_none());
        let mut huge = list(&[]);
        huge[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(parse_manufacturers(&huge).is_none());
        assert!(parse_manufacturers(&[0; 8]).is_none());
    }

    /// A language block with the given `(id, text)` strings, ids ascending.
    fn language(strings: &[(u32, &str)]) -> Vec<u8> {
        let count = strings.len();
        let (ids, records) = (16, 16 + 4 * count);
        let mut block = Vec::new();
        put(&mut block, &[1, count as u32, ids as u32, records as u32]);
        for (id, _) in strings {
            put(&mut block, &[*id]);
        }
        let mut at = records + 8 * count;
        let mut texts = Vec::new();
        for (_, text) in strings {
            let units: Vec<u16> = text.encode_utf16().chain([0]).collect();
            put(&mut block, &[at as u32, units.len() as u32]);
            at += units.len() * 2;
            texts.extend(units.iter().flat_map(|unit| unit.to_le_bytes()));
        }
        block.extend(texts);
        block
    }

    #[test]
    fn a_string_is_found_by_its_id() {
        let block = language(&[(5, "ALFA ROMEO"), (9, "AUDI"), (40, "MERCEDES-BENZ")]);
        assert_eq!(localised_text(&block, 9).as_deref(), Some("AUDI"));
        assert_eq!(localised_text(&block, 40).as_deref(), Some("MERCEDES-BENZ"));
        assert_eq!(localised_text(&block, 5).as_deref(), Some("ALFA ROMEO"));
        assert_eq!(localised_text(&block, 6), None);
        assert_eq!(localised_text(&block, 99), None);
        assert_eq!(localised_text(&[0; 8], 1), None);
    }
}
