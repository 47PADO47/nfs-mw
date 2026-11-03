//! The engine model of RIDE 6's bike engines: a media file of a Crankcase Audio REV source plugin (`PLUG`/`ADM3`).
//!
//! Every bike engine of RIDE 6 is made by that plugin (`docs/formats/ride6-audio.md`, section "REV engine models"): the
//! bank holds, per bike, two models (the engine and the exhaust event) and the plugin synthesises the sound from the
//! recordings in them. This module reads the table of recordings of a model; the recordings themselves are decoded by
//! `vgmstream` (`decode.rs`), which numbers them in the same order.
//!
//! Layout, measured on all 204 models of the 122 engine banks (the field names are ours):
//!
//! - bytes 0 to 3 `PLUG`; at `0x30` the tag `ADM3`; the `u32` at `0x40` is the offset of the first recording's bytes, and
//!   everything between `0x34` and it is the directory;
//! - the directory holds 48-byte records, each starting with the tag `SMP2`: `tag[4]`, `u32`, `u32`, `u32`, `hz f32`,
//!   `cycles u32`, `u32`, `sample rate u32`, `frames u32`, `mark u32`, `u32`, `offset u32`; the `u32` that follows is the next
//!   record's tag (`SMP2`), or 1 at the last record of a run;
//! - the records come in this order: a run of loops (**on load**), one long recording (**rev up**), a run of loops
//!   (**off load**), two long recordings (**coast down** and **idle**). A loop's `hz` rises along its run and is the
//!   frequency of the engine cycle (rpm / 120) it was recorded at; it holds `cycles` whole cycles, so its pitch is
//!   `cycles * rate / frames`; a long recording has `hz = 0` and `cycles = 0`.

use anyhow::{Result, bail, ensure};

/// The plugin number a bank gives the REV source plugin (`hirc::SoundRef::plugin` of an engine sound). Read from the banks:
/// every one of the 634 engine and exhaust events of RIDE 6 starts one such sound, and its media file is a `PLUG`/`ADM3`
/// model that `vgmstream` names "Crankcase ADMx".
pub const PLUGIN_ID: u32 = 0x01A0_1052;

/// One recording of a model.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RevStream {
    /// The number `vgmstream-cli -s` takes (1 is the first record).
    pub number: usize,
    /// The frequency tag of a loop; 0 for a long recording.
    pub tag_hz: f32,
    /// Whole engine cycles in a loop; 0 for a long recording.
    pub cycles: u32,
    pub sample_rate: u32,
    pub frames: u32,
}

impl RevStream {
    /// The pitch of a loop, engine cycles per second: its whole cycles over its length. The tag agrees with it except for
    /// the first loop of some models, whose tag is a rounded nominal value.
    pub fn cycle_hz(&self) -> f32 {
        if self.cycles == 0 || self.frames == 0 {
            return 0.0;
        }
        self.cycles as f32 * self.sample_rate as f32 / self.frames as f32
    }

    /// The engine speed of a loop, rpm: a four-stroke engine has one cycle per two turns, so rpm is 120 times the cycle
    /// frequency (the highest loops of 293 bikes fall at 0.97 of the redline in the median, `docs/formats/ride6-audio.md`).
    pub fn rpm(&self) -> f32 {
        self.cycle_hz() * RPM_PER_CYCLE_HZ
    }
}

/// rpm per cycle per second of a four-stroke engine: two turns of 60 seconds per minute.
pub const RPM_PER_CYCLE_HZ: f32 = 120.0;

/// A model's recordings by role.
#[derive(Debug, Clone, PartialEq)]
pub struct RevModel {
    pub on_load: Vec<RevStream>,
    pub rev_up: RevStream,
    pub off_load: Vec<RevStream>,
    pub coast_down: RevStream,
    pub idle: RevStream,
}

fn word(bytes: &[u8], at: usize) -> Result<u32> {
    let Some(slice) = bytes.get(at..at + 4) else { bail!("the model ends inside a record") };
    Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

/// Offset of the first recording and the directory's first byte.
const DIRECTORY_START: usize = 0x34;
const FIRST_OFFSET_AT: usize = 0x40;
const RECORD: usize = 48;

/// Read the directory of a model file.
pub fn parse(bytes: &[u8]) -> Result<RevModel> {
    ensure!(bytes.starts_with(b"PLUG"), "not a plugin media file (no PLUG tag)");
    ensure!(bytes.get(0x30..0x34) == Some(&b"ADM3"[..]), "not a REV model (no ADM3 tag)");
    let end = (word(bytes, FIRST_OFFSET_AT)? as usize).min(bytes.len());
    ensure!(end > DIRECTORY_START, "the directory is empty");
    let directory = &bytes[..end];
    let mut records = Vec::new();
    let mut at = DIRECTORY_START;
    while at + RECORD <= directory.len() {
        if &directory[at..at + 4] != b"SMP2" {
            at += 1;
            continue;
        }
        let number = records.len() + 1;
        let rate = word(directory, at + 28)?;
        ensure!((8_000..=192_000).contains(&rate), "record {number}: sample rate {rate}");
        records.push(RevStream {
            number,
            tag_hz: f32::from_bits(word(directory, at + 16)?),
            cycles: word(directory, at + 20)?,
            sample_rate: rate,
            frames: word(directory, at + 32)?,
        });
        at += RECORD;
    }
    ensure!(!records.is_empty(), "the model has no recordings");
    arrange(&records)
}

/// Split the records into the roles by their order: loops (rising pitch), long, loops, long, long.
fn arrange(records: &[RevStream]) -> Result<RevModel> {
    let mut runs: Vec<Vec<RevStream>> = Vec::new();
    let mut longs: Vec<RevStream> = Vec::new();
    let mut order = Vec::new();
    for record in records {
        if record.cycles == 0 {
            longs.push(*record);
            order.push('L');
            continue;
        }
        let new_run = match runs.last().and_then(|run| run.last()) {
            Some(last) if order.last() == Some(&'o') => record.tag_hz < last.tag_hz,
            _ => true,
        };
        if new_run {
            runs.push(Vec::new());
        }
        if let Some(run) = runs.last_mut() {
            run.push(*record);
        }
        order.push('o');
    }
    let pattern: String = order.iter().collect();
    let squeezed = {
        let mut text = String::new();
        for symbol in pattern.chars() {
            if !(symbol == 'o' && text.ends_with('o')) {
                text.push(symbol);
            }
        }
        text
    };
    ensure!(squeezed == "oLoLL", "unexpected recording order `{squeezed}` ({} records)", records.len());
    let mut runs = runs.into_iter();
    let (Some(on_load), Some(off_load)) = (runs.next(), runs.next()) else { bail!("expected two runs of loops") };
    ensure!(on_load.len() >= 2 && off_load.len() >= 2, "a run of loops has fewer than two recordings");
    Ok(RevModel { on_load, rev_up: longs[0], off_load, coast_down: longs[1], idle: longs[2] })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    fn record(hz: f32, cycles: u32, frames: u32) -> Vec<u8> {
        let mut out = b"SMP2".to_vec();
        for value in [0u32, 0x0005_0000, 1] {
            out.extend_from_slice(&value.to_le_bytes());
        }
        out.extend_from_slice(&hz.to_le_bytes());
        for value in [cycles, 0x0030_0001, 48_000, frames, frames / 2, 1, 0] {
            out.extend_from_slice(&value.to_le_bytes());
        }
        assert_eq!(out.len(), RECORD);
        out
    }

    /// A model file with the given records after a header and some other bytes in between.
    pub(crate) fn model(records: &[(f32, u32, u32)]) -> Vec<u8> {
        let mut out = b"PLUG".to_vec();
        out.resize(0x30, 0);
        out.extend_from_slice(b"ADM3");
        out.resize(0x40, 0);
        let body: Vec<u8> = records.iter().flat_map(|&(hz, cycles, frames)| record(hz, cycles, frames)).collect();
        let first = 0x44 + 16 + body.len() + 16;
        out.extend_from_slice(&(first as u32).to_le_bytes());
        out.extend_from_slice(&[7u8; 16]);
        out.extend(body);
        out.extend_from_slice(&[9u8; 16]);
        out.extend_from_slice(&[0u8; 64]);
        out
    }

    /// The usual shape: 3 loops, rev-up, 3 loops, coast-down, idle.
    pub(crate) fn usual() -> Vec<(f32, u32, u32)> {
        vec![
            (30.0, 18, 28_800),
            (60.0, 36, 28_800),
            (90.0, 54, 28_800),
            (0.0, 0, 200_000),
            (35.0, 21, 28_800),
            (55.0, 33, 28_800),
            (80.0, 48, 28_800),
            (0.0, 0, 300_000),
            (0.0, 0, 190_000),
        ]
    }

    #[test]
    fn a_model_is_split_into_its_roles_and_its_loops_have_a_pitch() {
        let parsed = parse(&model(&usual())).unwrap();
        assert_eq!(parsed.on_load.len(), 3);
        assert_eq!(parsed.off_load.len(), 3);
        assert_eq!(parsed.on_load[2].number, 3);
        assert_eq!((parsed.rev_up.number, parsed.rev_up.frames), (4, 200_000));
        assert_eq!(parsed.off_load[0].number, 5);
        assert_eq!((parsed.coast_down.number, parsed.idle.number), (8, 9));
        assert_eq!(parsed.idle.frames, 190_000);
        // 36 cycles in 28800 frames at 48 kHz is 60 Hz, a four-stroke at 7200 rpm.
        assert!((parsed.on_load[1].cycle_hz() - 60.0).abs() < 1e-3);
        assert!((parsed.on_load[1].rpm() - 7200.0).abs() < 0.1);
        assert_eq!(parsed.rev_up.cycle_hz(), 0.0);
    }

    #[test]
    fn the_pitch_follows_the_whole_cycles_not_a_rounded_tag() {
        let mut layout = usual();
        layout[0] = (30.4, 15, 24_000);
        let parsed = parse(&model(&layout)).unwrap();
        assert_eq!(parsed.on_load[0].tag_hz, 30.4);
        assert!((parsed.on_load[0].cycle_hz() - 30.0).abs() < 1e-3);
    }

    #[test]
    fn other_orders_and_other_files_are_refused() {
        assert!(parse(b"RIFF....").is_err());
        let mut no_adm = model(&usual());
        no_adm[0x30] = b'X';
        assert!(parse(&no_adm).is_err());
        let mut only_loops = usual();
        only_loops.truncate(3);
        assert!(parse(&model(&only_loops)).is_err());
        let mut extra_long = usual();
        extra_long.push((0.0, 0, 1000));
        assert!(parse(&model(&extra_long)).is_err());
        assert!(parse(&model(&[])).is_err());
        let mut bad_rate = model(&usual());
        let at = bad_rate.windows(4).position(|w| w == b"SMP2").unwrap() + 28;
        bad_rate[at..at + 4].copy_from_slice(&1u32.to_le_bytes());
        assert!(parse(&bad_rate).is_err());
    }
}
