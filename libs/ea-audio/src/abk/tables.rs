//! The module, player and sample tables of an `ABKC` bank.

use crate::bytes::{u8_at, u16_le, u32_le};
use crate::error::{Error, Result};

/// How a sample-table entry stores its sound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoundKind {
    /// In the bank's own `BNKl`; `index` selects the entry.
    Ram,
    /// Streamed from a companion file; `index` is an offset there.
    Streamed,
    /// Streamed, with a second offset for the looping part.
    StreamedLooped,
    Unknown(u8),
}

/// One 12-byte entry of a sample table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SampleEntry {
    pub kind: SoundKind,
    pub priority: u8,
    /// `BNKl` entry index for RAM sounds; an offset into the streamed file otherwise.
    pub index: u32,
    /// Offset of the looping part of a streamed sound.
    pub loop_offset: u32,
}

impl SampleEntry {
    /// Dummy entries (type 0, index 0) pad the tables and point at no sound.
    pub fn is_dummy(&self) -> bool {
        self.kind == SoundKind::Ram && self.index == 0
    }
}

/// A list of sounds a player can pick from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SampleTable {
    /// Offset of the table in the bank file.
    pub offset: u32,
    pub entries: Vec<SampleEntry>,
}

/// A player of a module; it plays the sounds of one sample table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Player {
    /// Index into [`super::Bank::sample_tables`].
    pub sample_table: usize,
}

/// A module groups players; the bank file has one to a few.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Module {
    pub players: Vec<Player>,
}

const ENTRY_SIZE: usize = 12;
const MODULE_HEADER: usize = 0x3C;

fn parse_table(data: &[u8], offset: usize) -> Result<SampleTable> {
    let count = u32_le(data, offset)? as usize;
    let end = count.checked_mul(ENTRY_SIZE).and_then(|n| n.checked_add(offset + 4));
    if end.is_none_or(|e| e > data.len()) {
        return Err(Error::Corrupt("sample table outside the bank"));
    }
    let entries = (0..count)
        .map(|k| {
            let at = offset + 4 + ENTRY_SIZE * k;
            let kind = match u8_at(data, at)? {
                0 => SoundKind::Ram,
                1 => SoundKind::Streamed,
                2 => SoundKind::StreamedLooped,
                other => SoundKind::Unknown(other),
            };
            Ok(SampleEntry {
                kind,
                priority: u8_at(data, at + 1)?,
                index: u32_le(data, at + 4)?,
                loop_offset: u32_le(data, at + 8)?,
            })
        })
        .collect::<Result<_>>()?;
    Ok(SampleTable { offset: offset as u32, entries })
}

/// Read all modules and the distinct sample tables their players point at.
pub fn parse(data: &[u8], module_count: usize, first_module: usize) -> Result<(Vec<Module>, Vec<SampleTable>)> {
    let mut modules = Vec::with_capacity(module_count);
    let mut tables: Vec<SampleTable> = Vec::new();
    let mut at = first_module;
    for _ in 0..module_count {
        let players = u8_at(data, at + 0x24)? as usize;
        let controllers = u8_at(data, at + 0x27)? as usize;
        let module_data = u32_le(data, at + 0x2C)? as usize;
        let mut parsed = Vec::with_capacity(players);
        for j in 0..players {
            let player = u32_le(data, at + MODULE_HEADER + 4 * j)? as usize;
            let table_offset = u32_le(data, module_data.saturating_add(player).saturating_add(4))? as usize;
            let index = match tables.iter().position(|t| t.offset as usize == table_offset) {
                Some(i) => i,
                None => {
                    tables.push(parse_table(data, table_offset)?);
                    tables.len() - 1
                }
            };
            parsed.push(Player { sample_table: index });
        }
        modules.push(Module { players: parsed });
        at += MODULE_HEADER + 4 * (players + controllers);
    }
    Ok((modules, tables))
}

/// Number of modules (`u16` at 0x0A) and the module-table offset (`u32` at 0x1C).
pub fn module_info(data: &[u8]) -> Result<(usize, usize)> {
    Ok((u16_le(data, 0x0A)? as usize, u32_le(data, 0x1C)? as usize))
}
