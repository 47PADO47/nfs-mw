//! Streamed world emitter placements. See `docs/specs/world-emitter-placements.md`.

use anyhow::{Context, Result, bail};

const LIBRARY: u32 = 0x0003_bc00;
const HEADER: usize = 16;
const RECORD: usize = 80;
const TRIGGER: usize = 48;

#[derive(Debug, Clone, PartialEq)]
pub struct Trigger {
    pub radius: f32,
    pub position: [f32; 3],
    pub reset_time: f32,
    pub probability: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Placement {
    pub group: u32,
    pub section: u16,
    /// Row-vector matrix in game coordinates, stored as sixteen consecutive floats.
    pub matrix: [f32; 16],
    pub triggers: Vec<Trigger>,
}

fn word(bytes: &[u8], at: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(bytes.get(at..at + 4).context("truncated word")?.try_into()?))
}

fn half(bytes: &[u8], at: usize) -> Result<u16> {
    Ok(u16::from_le_bytes(bytes.get(at..at + 2).context("truncated halfword")?.try_into()?))
}

fn float(bytes: &[u8], at: usize) -> Result<f32> {
    Ok(f32::from_bits(word(bytes, at)?))
}

fn parse(bytes: &[u8]) -> Result<Vec<Placement>> {
    let version = word(bytes, 4)?;
    if version != 5 {
        bail!("unsupported emitter library version {version}");
    }
    let count = word(bytes, 8)? as usize;
    if count > bytes.len().saturating_sub(HEADER) / RECORD {
        bail!("emitter count exceeds payload");
    }
    let mut at = HEADER;
    let mut placements = Vec::with_capacity(count);
    for _ in 0..count {
        let record = bytes.get(at..at + RECORD).context("truncated emitter record")?;
        let count = usize::from(half(record, 10)?);
        let mut matrix = [0.0; 16];
        for (i, value) in matrix.iter_mut().enumerate() {
            *value = float(record, 16 + i * 4)?;
        }
        if matrix.iter().any(|v| !v.is_finite()) {
            bail!("nonfinite emitter matrix");
        }
        at += RECORD;
        let size = count.checked_mul(TRIGGER).context("trigger count overflow")?;
        let records = bytes.get(at..at + size).context("truncated emitter triggers")?;
        let mut triggers = Vec::with_capacity(count);
        for record in records.as_chunks::<TRIGGER>().0 {
            let trigger = Trigger {
                radius: float(record, 12)?,
                position: [float(record, 16)?, float(record, 20)?, float(record, 24)?],
                reset_time: float(record, 32)?,
                probability: half(record, 36)?,
            };
            if !trigger.radius.is_finite()
                || !trigger.reset_time.is_finite()
                || trigger.position.iter().any(|v| !v.is_finite())
            {
                bail!("nonfinite emitter trigger");
            }
            triggers.push(trigger);
        }
        at += size;
        placements.push(Placement { group: word(record, 0)?, section: half(record, 8)?, matrix, triggers });
    }
    Ok(placements)
}

/// Read all emitter libraries in a section whose root buffer is 16-byte aligned.
pub fn read(bytes: &[u8]) -> Result<Vec<Placement>> {
    let mut result = Vec::new();
    for chunk in blackbox_chunk::find_all(bytes, LIBRARY) {
        result.extend(
            parse(chunk.aligned_payload(16)).with_context(|| format!("emitter library at {:#x}", chunk.offset))?,
        );
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_multiple_records_and_skips_triggers() {
        let mut data = vec![0; HEADER + 2 * RECORD + TRIGGER];
        data[4..8].copy_from_slice(&5u32.to_le_bytes());
        data[8..12].copy_from_slice(&2u32.to_le_bytes());
        data[HEADER..HEADER + 4].copy_from_slice(&123u32.to_le_bytes());
        data[HEADER + 10..HEADER + 12].copy_from_slice(&1u16.to_le_bytes());
        let second = HEADER + RECORD + TRIGGER;
        data[second..second + 4].copy_from_slice(&456u32.to_le_bytes());
        let result = parse(&data).unwrap();
        assert_eq!((result[0].group, result[0].triggers.len(), result[1].group), (123, 1, 456));
        assert!(parse(&data[..second + RECORD - 1]).is_err());
    }

    #[test]
    fn rejects_unknown_version_and_invalid_counts() {
        let mut data = vec![0; HEADER];
        assert!(parse(&data).is_err());
        data[4..8].copy_from_slice(&5u32.to_le_bytes());
        data[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(parse(&data).is_err());
    }

    #[test]
    #[ignore = "needs the installed game selected by NFSMW_GAME_DIR"]
    fn installed_stream_placements_and_groups() {
        use std::io::{Read, Seek, SeekFrom};
        let root = std::path::PathBuf::from(std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR"));
        let dir = game_install::GameDir::open(&root).unwrap();
        let index = crate::world::WorldIndex::open(&dir, "L2RA").unwrap();
        let mut file = std::fs::File::open(root.join(&index.stream_file)).unwrap();
        let mut placements = Vec::new();
        for section in &index.sections {
            let mut bytes = vec![0; section.size as usize];
            file.seek(SeekFrom::Start(u64::from(section.file_offset))).unwrap();
            file.read_exact(&mut bytes).unwrap();
            placements.extend(read(&bytes).unwrap());
        }
        assert!(placements.len() >= 1500, "found {} placements", placements.len());
        assert!(placements.iter().any(|p| !p.triggers.is_empty()));
        assert!(placements.iter().any(|p| p.triggers.is_empty()));
        let db = crate::shared::attributes(&dir).unwrap();
        let mut unique = std::collections::BTreeMap::new();
        for placement in &placements {
            *unique.entry(placement.group).or_insert(0usize) += 1;
        }
        for (key, count) in unique {
            let group = db.collection_by_key(blackbox_attrib::vlt_hash("emittergroup"), key);
            if let Some(group) = group {
                eprintln!(
                    "group {key:08X} {} count={count} far={:?} emitters={}",
                    group.name().unwrap_or("unknown"),
                    group.get_f32("FarClip"),
                    crate::car::exhaust::group_emitters(&db, group).len()
                );
            }
        }
        let groups = placements
            .iter()
            .filter_map(|p| db.collection_by_key(blackbox_attrib::vlt_hash("emittergroup"), p.group))
            .filter(|group| {
                group.get_f32("FarClip").is_some_and(|f| f > 0.0)
                    && !crate::car::exhaust::group_emitters(&db, *group).is_empty()
            })
            .count();
        assert!(groups > 0, "no world placements resolved to usable emitter groups");
    }
}
