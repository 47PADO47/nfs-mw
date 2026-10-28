//! A siren's flash pattern, read from the car's bundle and baked into a digit string a manifest can carry.

use chameleon_bundle::Bundle;
use chameleon_effects::corona::pattern::{self, FlashPattern, Segment};

/// The most cycles of the pattern that are baked looking for the length after which the whole pattern repeats.
const MAX_CYCLES: u32 = 16;
/// The longest baked pattern, in steps (the manifest's limit).
const MAX_STEPS: usize = 4096;
/// The step of a pattern whose rates say none, seconds.
const DEFAULT_STEP: f32 = 0.125;
/// How close to a whole number of turns a segment has to come for the pattern to repeat there.
const REPEAT_SLACK: f32 = 0.02;

/// The pattern object `id` of the bundle with the sequences it names. `None` when it or a sequence is missing.
pub(super) fn read(bundle: &Bundle, id: u64) -> Option<FlashPattern> {
    let object = bundle.resource(bundle.find(id)?).ok()?;
    let mut segments = Vec::new();
    for entry in pattern::segments(&object.blocks[0]).ok()? {
        let reference = object.imports.iter().find(|import| import.slot() as usize == entry.reference_at)?;
        let sequence = bundle.resource(bundle.find(reference.id)?).ok()?;
        let steps = pattern::sequence(&sequence.blocks[0]).ok()?;
        segments.push(Segment { weight: entry.weight, rate: entry.rate, offset: entry.offset, steps });
    }
    (!segments.is_empty()).then_some(FlashPattern { segments })
}

/// The pattern sampled as one digit (0 to 9, the brightness in ninths) a step, and the step in seconds. The step is the
/// shortest any segment plays one of its own, and the pattern is baked over as many cycles as it takes to come back to where
/// it started (the segments turn their sequences by a different amount in a cycle), at most [`MAX_CYCLES`].
pub(super) fn bake(pattern: &FlashPattern) -> (String, f32) {
    let cycle = pattern.cycle();
    let step = pattern
        .segments
        .iter()
        .filter(|segment| segment.rate > 0.0 && !segment.steps.is_empty())
        .map(|segment| 1.0 / (segment.rate * segment.steps.len() as f32))
        .fold(f32::MAX, f32::min);
    let step = match step < f32::MAX {
        true => step.clamp(0.005, 2.0),
        false => DEFAULT_STEP,
    };
    let repeats = (1..=MAX_CYCLES)
        .find(|&k| {
            pattern.segments.iter().all(|segment| {
                let turns = segment.rate * cycle * k as f32;
                (turns - turns.round()).abs() < REPEAT_SLACK
            })
        })
        .unwrap_or(1);
    let steps = ((cycle * repeats as f32 / step).round() as usize).clamp(1, MAX_STEPS);
    let digits = (0..steps)
        .map(|n| {
            let level = pattern.at((n as f32 + 0.5) * step).clamp(0.0, 1.0);
            char::from_digit((level * 9.0).round() as u32, 10).unwrap_or('0')
        })
        .collect();
    (digits, step)
}
