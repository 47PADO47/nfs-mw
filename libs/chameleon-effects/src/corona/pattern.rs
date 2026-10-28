//! The flash pattern of a siren corona: a cycle of segments, each playing a sequence of intensity steps.
//! Format: `docs/formats/chameleon-corona.md`.
//!
//! A corona record of a police car points at a pattern object. The pattern lists segments, each with a weight (its share of
//! the cycle), a rate and an offset, and a reference to a sequence object: a list of intensities, one per step.

use crate::{Error, f32_at, u16_at, u32_at};

/// Where a pattern object keeps the offset of its first segment, and the segment count.
const ENTRIES_AT: usize = 0x10;
const COUNT_AT: usize = 0x14;
/// A segment: the reference to its sequence at +0x0C, then weight, rate and offset floats at +0x18, +0x1C, +0x20.
const ENTRY_LEN: usize = 0x24;
const SEQUENCE_REF: usize = 0x0C;
const MAX_SEGMENTS: usize = 16;
const MAX_STEPS: usize = 1024;

/// A segment of a pattern as the object holds it: the sequence object is only named.
#[derive(Debug, Clone, PartialEq)]
pub struct SegmentRef {
    /// The segment's share of the cycle: its weight over the sum of all weights.
    pub weight: f32,
    /// Sequence cycles a second, in time units of the cycle (the sequence plays `rate * t + offset` times through).
    pub rate: f32,
    pub offset: f32,
    /// Offset in the pattern object of the sequence reference, to look up in its import table.
    pub reference_at: usize,
}

/// A segment with its sequence read.
#[derive(Debug, Clone, PartialEq)]
pub struct Segment {
    pub weight: f32,
    pub rate: f32,
    pub offset: f32,
    /// Intensity of each step, 0 to 1.
    pub steps: Vec<f32>,
}

/// The segments of a pattern object (without their sequences).
pub fn segments(main: &[u8]) -> Result<Vec<SegmentRef>, Error> {
    let short = Error::Truncated("flash pattern");
    let entries = u32_at(main, ENTRIES_AT).ok_or(short.clone())? as usize;
    let count = usize::from(u16_at(main, COUNT_AT).ok_or(short.clone())?);
    if count > MAX_SEGMENTS {
        return Err(Error::Truncated("flash pattern: too many segments"));
    }
    let mut out = Vec::with_capacity(count);
    for n in 0..count {
        let at = entries + ENTRY_LEN * n;
        out.push(SegmentRef {
            weight: f32_at(main, at + 0x18).ok_or(short.clone())?,
            rate: f32_at(main, at + 0x1C).ok_or(short.clone())?,
            offset: f32_at(main, at + 0x20).ok_or(short.clone())?,
            reference_at: at + SEQUENCE_REF,
        });
    }
    Ok(out)
}

/// The steps of a sequence object: the offset of the first at +0x10, the count at +0x14, a float each.
pub fn sequence(main: &[u8]) -> Result<Vec<f32>, Error> {
    let short = Error::Truncated("flash sequence");
    let first = u32_at(main, ENTRIES_AT).ok_or(short.clone())? as usize;
    let count = usize::from(u16_at(main, COUNT_AT).ok_or(short.clone())?);
    if count > MAX_STEPS {
        return Err(Error::Truncated("flash sequence: too many steps"));
    }
    (0..count).map(|n| f32_at(main, first + 4 * n).ok_or(short.clone())).collect()
}

/// A whole pattern, ready to play.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FlashPattern {
    pub segments: Vec<Segment>,
}

impl FlashPattern {
    /// The length of a cycle in time units: the sum of the weights.
    pub fn cycle(&self) -> f32 {
        self.segments.iter().map(|segment| segment.weight).sum()
    }

    /// The intensity at time `t` (seconds, taking one time unit as a second **[unconfirmed]**): the segment is the one
    /// whose share of the cycle holds `t`, and its step is the one `rate * t + offset` turns of the sequence reach.
    pub fn at(&self, t: f32) -> f32 {
        let total = self.cycle();
        if self.segments.is_empty() || total <= 0.0 {
            return 0.0;
        }
        let mut left = (t / total).rem_euclid(1.0) * total;
        let mut chosen = &self.segments[self.segments.len() - 1];
        for segment in &self.segments {
            left -= segment.weight;
            if left <= 0.0 {
                chosen = segment;
                break;
            }
        }
        if chosen.steps.is_empty() {
            return 0.0;
        }
        let turns = (chosen.rate * t + chosen.offset).rem_euclid(1.0);
        let step = ((turns * chosen.steps.len() as f32) as usize).min(chosen.steps.len() - 1);
        chosen.steps[step]
    }

    /// The pattern sampled every `step` seconds over one cycle.
    pub fn sample(&self, step: f32) -> Vec<f32> {
        let count = (self.cycle() / step).round() as usize;
        (0..count).map(|n| self.at((n as f32 + 0.5) * step)).collect()
    }
}
