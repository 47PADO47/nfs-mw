//! The sequence model: what a sequence and its items are once read, and how an item's window is evaluated.

use crate::value::{Fade, Modulator};

/// A wave item: loops (or one-shots) played in a window of the sequence's live value.
#[derive(Debug, Clone, PartialEq)]
pub struct WaveItem {
    /// The sample resources it can play; one is picked when `random`.
    pub waves: Vec<u64>,
    /// The mixer channel it plays on.
    pub mixer: u64,
    pub gain: Modulator,
    pub pitch: Modulator,
    /// The pitch modulator's `base` is the live value at which the sample plays at its own speed: the playback rate is
    /// the live value over it (the transmission whine).
    pub tracks_driver: bool,
    pub random: bool,
}

/// A Ginsu item: an engine recording played by the live value as its frequency.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GinsuItem {
    /// The id of the `Ginsu` resource.
    pub resource: u64,
    pub mixer: u64,
}

/// A mixer item: the gain of a mixer channel as a function of the live value.
#[derive(Debug, Clone, PartialEq)]
pub struct MixerItem {
    pub channel: u64,
    /// The modulators that set the channel's gain (the values with no DSP parameter id).
    pub gains: Vec<Modulator>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ItemKind {
    Wave(WaveItem),
    Ginsu(GinsuItem),
    Mixer(MixerItem),
    /// An item of another class (gear-change effects, LFOs, bus gains), by class name.
    Other(String),
}

/// One item: it is active while the live value is between `lo` and `hi`, fades in at `lo` and out at `hi`.
#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub lo: f32,
    pub hi: f32,
    pub fade_in: Fade,
    pub fade_out: Fade,
    pub kind: ItemKind,
}

impl Item {
    /// The item's gain at the live value `value` (0 outside its window).
    pub fn window(&self, value: f32) -> f32 {
        if value < self.lo || value > self.hi {
            return 0.0;
        }
        self.fade_in.gain(value - self.lo).min(self.fade_out.gain(self.hi - value))
    }
}

/// A sequence: items played along the range of a live value.
#[derive(Debug, Clone, PartialEq)]
pub struct Sequence {
    pub id: u64,
    /// The live value that drives it (`Players.LocalPlayer.GetPlayerAudibleRPM`).
    pub driver: String,
    /// The range of the live value; it is clamped into it.
    pub lo: f32,
    pub hi: f32,
    pub items: Vec<Item>,
}

impl Sequence {
    /// Whether the driver's name contains `name`.
    pub fn driven_by(&self, name: &str) -> bool {
        self.driver.contains(name)
    }

    /// The live value clamped into the sequence's range.
    pub fn clamp(&self, value: f32) -> f32 {
        value.clamp(self.lo.min(self.hi), self.lo.max(self.hi))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(lo: f32, hi: f32, fade_in: f32, fade_out: f32) -> Item {
        Item {
            lo,
            hi,
            fade_in: Fade { shape: 1.0, span: fade_in },
            fade_out: Fade { shape: 1.0, span: fade_out },
            kind: ItemKind::Other(String::new()),
        }
    }

    #[test]
    fn an_item_is_silent_outside_its_window_and_fades_at_both_ends() {
        let whine = item(20.0, 90.0, 20.0, 20.0);
        assert_eq!((whine.window(10.0), whine.window(95.0)), (0.0, 0.0));
        assert!((whine.window(30.0) - 0.5).abs() < 1e-5);
        assert_eq!(whine.window(55.0), 1.0);
        assert!((whine.window(80.0) - 0.5).abs() < 1e-5);
    }

    #[test]
    fn a_sequence_clamps_the_live_value_into_its_range() {
        let sequence =
            Sequence { id: 1, driver: "x.GetPlayerAudibleRPM".into(), lo: 1790.0, hi: 7500.0, items: Vec::new() };
        assert_eq!((sequence.clamp(800.0), sequence.clamp(9000.0), sequence.clamp(3000.0)), (1790.0, 7500.0, 3000.0));
        assert!(sequence.driven_by("AudibleRPM") && !sequence.driven_by("Speed"));
    }
}
