//! Master channel outputs: levels to the slots of a sound object. Spec: `docs/specs/dynamic-mixer.md` §7.

use super::{MasterNode, Mixer};
use crate::map::{OutputKind, PresetWord};
use crate::shape::{SILENCE_DB, pitch_ratio, q15_from_db};

/// Lowest and highest pitch offset in cents (-4 octaves... -4800, +2400).
const PITCH_MIN: i32 = -0x12C0;
const PITCH_MAX: i32 = 0x960;
/// A fully open filter.
pub const FILTER_OPEN: i32 = 25_000;

pub(super) fn write(m: &mut Mixer) {
    for i in 0..m.masters.len() {
        let node = &m.masters[i];
        let level = m.graph.master[i];
        let attached = m.blocks[node.block].attached;
        if !attached {
            // Only the first word is written, with the "off" value of its kind.
            if let Some(first) = node.words.first() {
                m.blocks[node.block].slots[usize::from(first.slot)] = off_value(node.kind) as u16;
            }
            continue;
        }
        for word in &node.words {
            let value = word_value(m, node, word, level);
            m.blocks[node.block].slots[usize::from(word.slot)] = (value & 0xFFFF) as u16;
        }
    }
}

fn off_value(kind: OutputKind) -> i32 {
    match kind {
        OutputKind::Pitch => 0,
        OutputKind::Filter => FILTER_OPEN,
        OutputKind::Volume | OutputKind::Depth | OutputKind::Other => 0,
    }
}

fn word_value(m: &Mixer, node: &MasterNode, word: &PresetWord, level: i32) -> i32 {
    let offset = level.saturating_add(word.offset);
    let Some(&spatial) = node.spatial.get(usize::from(word.spatial)) else {
        return plain(node.kind, offset);
    };
    if word.azimuth {
        return m.graph.spatial_azimuth[spatial] & 0xFFFF;
    }
    match node.kind {
        OutputKind::Volume | OutputKind::Depth => {
            q15_from_db(m.graph.spatial_db[spatial].saturating_add(offset).clamp(SILENCE_DB, 0))
        }
        // The rolloff's Doppler term is not computed (0); the original zeroes a value below the range here.
        OutputKind::Pitch if offset > PITCH_MAX => PITCH_MAX,
        OutputKind::Pitch if offset < PITCH_MIN => 0,
        OutputKind::Pitch => offset,
        OutputKind::Filter => offset.clamp(SILENCE_DB, 0),
        OutputKind::Other => level,
    }
}

/// The conversion without a 3D control.
fn plain(kind: OutputKind, x: i32) -> i32 {
    match kind {
        OutputKind::Volume | OutputKind::Depth => q15_from_db(x.clamp(SILENCE_DB, 0)),
        OutputKind::Pitch => x.clamp(PITCH_MIN, PITCH_MAX),
        OutputKind::Filter => (pitch_ratio(x.clamp(SILENCE_DB, 0)) * FILTER_OPEN as f32) as i32,
        OutputKind::Other => x.clamp(0, FILTER_OPEN),
    }
}
