//! Installed moment-stream selectors. Spec: `speedbreaker-moment-audio.md`.
use blackbox_attrib::{CollectionRef, Database, Value};
use ea_audio::speech::SpeechIndex;

#[derive(Clone, Copy, Debug)]
pub struct MomentSound {
    pub stream: u32,
    pub param: u32,
    pub volume_slot: usize,
    pub positioned: bool,
}

impl MomentSound {
    pub fn speedbreaker(db: &Database, interactive: bool) -> Option<Self> {
        let name = match interactive {
            true => "purs_break_start_music",
            false => "purs_break_start_fx",
        };
        Self::named(db, name)
    }

    /// The authored gas-station blast, independent of generic collision events.
    pub fn petrol_explosion(db: &Database) -> Option<Self> {
        Self::named(db, "expl_gas_station")
    }

    fn named(db: &Database, name: &str) -> Option<Self> {
        let c = db.collection("aud_moment_strm", name)?;
        Some(Self {
            stream: word(c, "stream")?,
            param: word(c, "param")?,
            volume_slot: word(c, "VolSlot")? as usize,
            positioned: c.get("IsPositioned").and_then(Value::as_bool).unwrap_or(false),
        })
    }
}

fn word(c: CollectionRef<'_>, field: &str) -> Option<u32> {
    match c.get(field)? {
        Value::Raw { bytes, .. } => Some(u32::from_le_bytes(bytes.get(..4)?.try_into().ok()?)),
        value => value.as_u32(),
    }
}

/// Match the SoundFX phrase's two sample selectors, keeping flags with their own take.
pub fn moment_take(index: &[u8], sound: MomentSound) -> Result<(usize, usize), String> {
    let parsed = SpeechIndex::parse(index).map_err(|e| e.to_string())?;
    for (bank, data) in parsed.banks().iter().enumerate() {
        if data.header.event != 0x400e {
            continue;
        }
        let row = index.get(0x58 + bank * 16..0x58 + (bank + 1) * 16).ok_or("moment bank row missing")?;
        let size = u32::from_le_bytes(row[4..8].try_into().unwrap()) as usize;
        let at = u32::from_le_bytes(row[8..12].try_into().unwrap()) as usize;
        let header =
            index.get(at..at.checked_add(size).ok_or("moment header overflow")?).ok_or("moment header missing")?;
        let fields = usize::from(header[4] & 0x7f);
        if fields != 2 {
            return Err("SoundFX bank needs two sample selector fields".into());
        }
        let masks = [sound.stream, sound.param];
        let mut matches = Vec::new();
        for take in 0..data.header.takes() {
            let flags = header.get(14 + take * 4..16 + take * 4).ok_or("moment selectors missing")?;
            if selectors_match(flags, &masks) {
                matches.push(take);
            }
        }
        if matches.len() == 1 {
            return Ok((bank, matches[0]));
        }
        return Err(format!("moment selectors have {} matching takes; selection policy unavailable", matches.len()));
    }
    Err("SoundFX moment bank missing".into())
}

fn selectors_match(flags: &[u8], masks: &[u32]) -> bool {
    flags.len() == masks.len() && flags.iter().zip(masks).all(|(&bit, &mask)| bit < 32 && mask & (1 << bit) != 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stream_and_parameter_are_independent_masks() {
        assert!(selectors_match(&[0, 18], &[1, 1 << 18]));
        assert!(!selectors_match(&[0, 16], &[1, 1 << 18]));
        assert!(!selectors_match(&[1, 18], &[1, 1 << 18]));
        assert!(!selectors_match(&[0, 32], &[1, u32::MAX]));
    }
}
