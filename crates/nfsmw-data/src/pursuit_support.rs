//! Context-selected support records; `docs/formats/pursuit-data.md` section 2.3.

use super::{HEAT_LEVELS, PursuitTable, level_collection};
use blackbox_attrib::{Database, Value};

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SupportRecord {
    pub strategy: u32,
    pub chance: f32,
    pub duration: f32,
    pub big_suv_chance: f32,
    pub priority_chance: f32,
    pub priority_time: f32,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SupportRow {
    pub minimum_delay: f32,
    pub air: Vec<SupportRecord>,
    pub heavy: Vec<SupportRecord>,
    pub leaders: Vec<SupportRecord>,
}

impl PursuitTable {
    pub fn support(self, db: &Database) -> Option<Vec<SupportRow>> {
        let table = match self {
            Self::OpenWorld => "supporttable",
            Self::Race => "supportracetable",
        };
        (1..=HEAT_LEVELS)
            .map(|heat| {
                let row = level_collection(db, table, heat)?;
                Some(SupportRow {
                    minimum_delay: row.get_f32("MinimumSupportDelay")?,
                    air: records(row.get("AirSupportOptions"), 12)?,
                    heavy: records(row.get("HeavySupportOptions"), 16)?,
                    leaders: records(row.get("LeaderSupportOptions"), 20)?,
                })
            })
            .collect()
    }
}

fn records(value: Option<&Value>, width: usize) -> Option<Vec<SupportRecord>> {
    let Some(value) = value else { return Some(Vec::new()) };
    value
        .as_array()?
        .iter()
        .map(|value| {
            let Value::Raw { bytes, .. } = value else { return None };
            decode_record(bytes, width)
        })
        .collect()
}

fn decode_record(bytes: &[u8], width: usize) -> Option<SupportRecord> {
    let bytes = bytes.get(..width)?;
    let word = |offset| u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
    let mut record = SupportRecord {
        strategy: word(0),
        chance: word(4) as f32,
        duration: f32::from_bits(word(8)),
        ..Default::default()
    };
    if width == 16 {
        record.big_suv_chance = word(12) as f32;
    }
    if width == 20 {
        record.priority_chance = word(12) as f32;
        record.priority_time = f32::from_bits(word(16));
    }
    Some(record)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heavy_chance_and_leader_priority_have_distinct_layouts() {
        let bytes: Vec<u8> = [
            5_u32.to_le_bytes(),
            25_u32.to_le_bytes(),
            30_f32.to_le_bytes(),
            80_u32.to_le_bytes(),
            120_f32.to_le_bytes(),
        ]
        .concat();
        let heavy = decode_record(&bytes, 16).unwrap();
        assert_eq!((heavy.chance, heavy.duration, heavy.big_suv_chance), (25.0, 30.0, 80.0));
        let leader = decode_record(&bytes, 20).unwrap();
        assert_eq!((leader.priority_chance, leader.priority_time, leader.big_suv_chance), (80.0, 120.0, 0.0));
        assert!(decode_record(&bytes[..11], 12).is_none());
    }
}
