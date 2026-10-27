//! The car's ignition sound: the `Player Vehicle Ignition` sequence of its bundle.
//!
//! A car bundle holds a sound sequence (kind `0x15`, class `Genesys.Gen.Sequence`) whose name string is `Player Vehicle
//! Ignition`. It has no live value to follow (its range is 0 to 0), so it plays once from its start: one wave item, a
//! single sample over a window as long as the sample, and an effect item (class `RaceCarEffectSequenceItem`, what it does
//! is not decoded) over a later window. The ids of the sequences differ between cars: the car's own sample plays.

use chameleon_bundle::{Bundle, ResourceKind};
use chameleon_sound_graph::{ItemKind, SoundGraphs};

/// The name string of the sequence.
const NAME: &[u8] = b"Player Vehicle Ignition";

/// What a car's ignition sequence plays.
#[derive(Debug, Clone, PartialEq)]
pub struct Ignition {
    /// The id of the sequence.
    pub sequence: u64,
    /// The id of the sample (kind `0x81`) the wave item plays.
    pub sample: u64,
    /// The end of the wave item's window in seconds: the length the sequence gives the sound.
    pub seconds: f32,
    /// The gain of the wave item (its modulator's base; the item has no mixer channel).
    pub gain: f32,
    /// The window of the effect item, in seconds, when the sequence has one. What the effect is, is not decoded.
    pub effect_window: Option<(f32, f32)>,
}

/// The ignition sequence of the car in `bundle`. `None` when it has none, or its wave item plays no sample.
pub fn read_ignition(bundle: &Bundle) -> Option<Ignition> {
    let kind = ResourceKind::GenesysObject.id();
    let id = bundle
        .entries()
        .iter()
        .enumerate()
        .filter(|(_, entry)| entry.kind == kind)
        .find(|(index, _)| names_ignition(bundle, *index))
        .map(|(_, entry)| entry.id)?;
    let graphs = SoundGraphs::read(bundle);
    let sequence = graphs.sequences.iter().find(|sequence| sequence.id == id)?;
    let (mut sample, mut seconds, mut gain, mut effect_window) = (None, 0.0, 1.0, None);
    for item in &sequence.items {
        match &item.kind {
            ItemKind::Wave(wave) if sample.is_none() => {
                sample = wave.waves.first().copied();
                seconds = item.hi;
                gain = wave.gain.base;
            }
            ItemKind::Other(class) if class.contains("Effect") && effect_window.is_none() => {
                effect_window = Some((item.lo, item.hi));
            }
            _ => {}
        }
    }
    Some(Ignition { sequence: id, sample: sample?, seconds, gain, effect_window })
}

/// Whether the main block of the resource at `index` holds the sequence's name.
fn names_ignition(bundle: &Bundle, index: usize) -> bool {
    let Ok(resource) = bundle.resource(index) else { return false };
    let Some(block) = resource.blocks.first() else { return false };
    block.windows(NAME.len()).any(|window| window == NAME)
}
