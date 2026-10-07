//! Reading a `CarPartPack` chunk.

use blackbox_chunk::{Chunk, ids};

use super::{ModelTable, Part, PartsDb};
use crate::bytes::{u16_at, u32_at};
use crate::layout::{CarDataLayout, ModelTableLayout, PartLayout};
use crate::{Error, Result};

/// The first `CarPartPack` in `data`.
pub fn read_parts_db(data: &[u8], layout: &CarDataLayout) -> Result<PartsDb> {
    let l = &layout.parts;
    let pack = blackbox_chunk::find(data, ids::CAR_PART_PACK).ok_or(Error::Missing("CarPartPack"))?;
    let child = |id, what| pack.child(id).map(|c: Chunk<'_>| c.payload).ok_or(Error::Missing(what));

    let header = child(ids::CAR_PART_HEADER, "CarPartPack header")?;
    if header.len() < l.header_num_parts + 4 {
        return Err(Error::Malformed { what: "CarPartPack header", detail: format!("{} bytes", header.len()) });
    }
    let version = u32_at(header, l.header_version);
    if version != l.version {
        return Err(Error::UnsupportedVersion { found: version, expected: l.version });
    }
    let count = |o| u32_at(header, o) as usize;
    // Tables carry trailing padding, so the header's counts bound them.
    let records = |payload: &'static str, bytes: &[u8], len: usize, n: usize| -> Result<Vec<Vec<u8>>> {
        let need = len * n;
        bytes.get(..need).map(|b| b.chunks_exact(len).map(<[u8]>::to_vec).collect()).ok_or_else(|| Error::Malformed {
            what: payload,
            detail: format!("{} bytes for {n} records of {len}", bytes.len()),
        })
    };

    let attributes = records(
        "attributes table",
        child(ids::CAR_PART_ATTRIBUTES_TABLE, "attributes table")?,
        l.attribute_len,
        count(l.header_num_attributes),
    )?;
    let type_names = records(
        "type-name table",
        child(ids::CAR_PART_TYPE_NAME_TABLE, "type-name table")?,
        4,
        count(l.header_num_type_names),
    )?;
    let model_tables = records(
        "model table",
        child(ids::CAR_PART_MODEL_TABLE, "model table")?,
        l.model_table.len,
        count(l.header_num_model_tables),
    )?;
    let parts = records(
        "parts table",
        child(ids::CAR_PART_PARTS_TABLE, "parts table")?,
        l.part.len,
        count(l.header_num_parts),
    )?;

    Ok(PartsDb {
        string_unit: l.string_unit,
        strings: child(ids::CAR_PART_STRING_TABLE, "string table")?.to_vec(),
        attribute_lists: child(ids::CAR_PART_ATTRIBUTE_LISTS, "attribute lists")?.to_vec(),
        attributes: attributes.iter().map(|r| (u32_at(r, 0), u32_at(r, 4))).collect(),
        model_tables: model_tables.iter().map(|r| model_table(r, &l.model_table)).collect(),
        type_names: type_names.iter().map(|r| u32_at(r, 0)).collect(),
        parts: parts.iter().map(|r| part(r, &l.part)).collect(),
    })
}

fn part(r: &[u8], l: &PartLayout) -> Part {
    let packed = r[l.group_and_level];
    Part {
        name_hash: u32::from(u16_at(r, l.name_hash + 2)) << 16 | u32::from(u16_at(r, l.name_hash)),
        part_id: r[l.part_id],
        group: packed & 0x1F,
        upgrade_level: packed >> 5,
        base_selector: r[l.base_selector] as i8,
        type_index: r[l.type_index],
        name_offset: u16_at(r, l.name_offset),
        attribute_list: u16_at(r, l.attribute_list),
        model_table: u16_at(r, l.model_table),
    }
}

fn model_table(r: &[u8], l: &ModelTableLayout) -> ModelTable {
    let middle = u16_at(r, l.middle_string);
    ModelTable {
        templated: r[l.templated] != 0,
        middle: (middle != u16::MAX).then_some(middle),
        entries: (0..l.lods).map(|i| u32_at(r, l.entries + i * 4)).collect(),
    }
}
