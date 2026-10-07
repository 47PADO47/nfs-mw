//! Synthetic car tables in the NFS: Most Wanted layout.

use blackbox_chunk::ids;
use blackbox_hash::bstring_hash;

fn chunk(id: u32, payload: &[u8]) -> Vec<u8> {
    let mut v = id.to_le_bytes().to_vec();
    v.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    v.extend_from_slice(payload);
    v
}

fn le32(values: &[u32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

/// A string table; returns the bytes and each string's offset in 4-byte units.
fn strings(list: &[&str]) -> (Vec<u8>, Vec<u32>) {
    let mut bytes = vec![0u8; 4];
    let mut offsets = Vec::new();
    for s in list {
        offsets.push(bytes.len() as u32 / 4);
        bytes.extend_from_slice(s.as_bytes());
        bytes.push(0);
        bytes.resize(bytes.len().next_multiple_of(4), 0);
    }
    (bytes, offsets)
}

#[allow(clippy::too_many_arguments)]
fn part(
    name: u32,
    part_id: u8,
    level: u8,
    selector: u8,
    type_index: u8,
    name_off: u16,
    attrs: u16,
    model: u16,
) -> Vec<u8> {
    let mut r = Vec::new();
    r.extend_from_slice(&(name as u16).to_le_bytes());
    r.extend_from_slice(&((name >> 16) as u16).to_le_bytes());
    r.extend_from_slice(&[part_id, level << 5, selector, type_index]);
    for v in [name_off, attrs, model] {
        r.extend_from_slice(&v.to_le_bytes());
    }
    r
}

fn model_table(templated: bool, middle: u16, entries: [u32; 5]) -> Vec<u8> {
    let mut r = vec![u8::from(templated), 0];
    r.extend_from_slice(&middle.to_le_bytes());
    r.extend(le32(&entries));
    r
}

/// Three parts: CAR's stock body, CAR's kit-01 body and a BBS rim in WHEELS.
pub fn parts_pack() -> Vec<u8> {
    let (strings, o) = strings(&["_KIT00", "_KIT01", "_BODY", "BODY_00", "BODY_01", "WHEEL", "_STYLE01_18_25", ""]);
    let none = u32::MAX;
    let models = [
        model_table(true, o[0] as u16, [o[2], none, none, none, none]),
        model_table(true, o[1] as u16, [o[2], none, none, none, none]),
        model_table(true, o[6] as u16, [o[7], none, none, none, none]),
    ]
    .concat();
    let attributes = le32(&[
        bstring_hash("KITNUMBER"),
        0,
        bstring_hash("KITNUMBER"),
        1,
        bstring_hash("BRAND_NAME"),
        bstring_hash("BBS"),
    ]);
    // Lists (i16): [1, 0] at 0, [1, 1] at 2, [1, 2] at 4.
    let lists: Vec<u8> = [1i16, 0, 1, 1, 1, 2].iter().flat_map(|v| v.to_le_bytes()).collect();
    let parts = [
        part(0x1111_0001, 23, 0, 1, 0, o[3] as u16, 0, 0),
        part(0x1111_0002, 23, 1, 1, 0, o[4] as u16, 2, 1),
        part(0x2222_0001, 67, 1, 2, 1, o[5] as u16, 4, 2),
    ]
    .concat();
    let mut header = vec![0u8; 60];
    header[0x08..0x0C].copy_from_slice(&6u32.to_le_bytes());
    for (at, n) in [(0x20, 3u32), (0x28, 2), (0x30, 3), (0x38, 3)] {
        header[at..at + 4].copy_from_slice(&n.to_le_bytes());
    }
    let body = [
        chunk(ids::CAR_PART_HEADER, &header),
        chunk(ids::CAR_PART_STRING_TABLE, &strings),
        chunk(ids::CAR_PART_ATTRIBUTE_LISTS, &lists),
        chunk(ids::CAR_PART_ATTRIBUTES_TABLE, &[attributes, vec![0; 8]].concat()),
        chunk(ids::CAR_PART_MODEL_TABLE, &models),
        chunk(
            ids::CAR_PART_TYPE_NAME_TABLE,
            &[le32(&[bstring_hash("CAR"), bstring_hash("WHEELS")]), vec![0; 4]].concat(),
        ),
        chunk(ids::CAR_PART_PARTS_TABLE, &[parts, vec![0; 8]].concat()),
    ]
    .concat();
    chunk(ids::CAR_PART_PACK, &body)
}

pub fn slot_types() -> Vec<u8> {
    let own = u32::MAX;
    let mut defaults: Vec<u32> = (0..139).flat_map(|_| [own, 0]).collect();
    defaults[44 * 2 + 1] = bstring_hash("SPOILER");
    let overrides = [bstring_hash("911TURBO"), 44, bstring_hash("SPOILER_PORSCHES"), 0];
    chunk(ids::CAR_PART_SLOT_TYPES, &[le32(&defaults), le32(&overrides)].concat())
}

pub fn car_types() -> Vec<u8> {
    let mut r = vec![0u8; 0xD0];
    r[..3].copy_from_slice(b"CAR");
    r[0x10..0x13].copy_from_slice(b"CAR");
    r[0x50..0x54].copy_from_slice(&bstring_hash("CAR").to_le_bytes());
    r[0xC7] = 1;
    r[0xCC..0xD0].copy_from_slice(&0xC7F2_884Eu32.to_le_bytes());
    // The payload starts 8 bytes into a 0x10-aligned file position: 8 bytes of 0x11 padding.
    chunk(ids::CAR_TYPE_INFOS, &[vec![0x11; 8], r].concat())
}

pub fn preset() -> Vec<u8> {
    let mut r = vec![0u8; 0x290];
    r[0x08..0x0B].copy_from_slice(b"CAR");
    r[0x28..0x32].copy_from_slice(b"CAR_STREET");
    r[0x60 + 23 * 4] = 1;
    chunk(ids::PRESET_RIDES, &r)
}
