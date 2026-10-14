//! The stepping helpers the option rows share: the volume nudge, the render-scale presets and the wrap-around cycle.
use crate::settings::{Percent, RenderScale};

/// Render scales (percent) the row cycles through: FSR 1's quality modes and a few supersampling steps.
const RENDER_SCALES: [u16; 9] = [50, 59, 67, 77, 85, 100, 125, 150, 200];
/// A slider press moves the volume by this many percent.
const VOLUME_STEP: u8 = 10;

pub(super) fn nudge(p: Percent, forward: bool) -> Percent {
    let v = if forward { p.0.saturating_add(VOLUME_STEP).min(100) } else { p.0.saturating_sub(VOLUME_STEP) };
    Percent(v)
}

/// The next preset above (`forward`) or below the scale, wrapping around; a scale between presets (set from the
/// console or the config file) moves to the closest preset in that direction.
pub(super) fn next_scale(current: RenderScale, forward: bool) -> RenderScale {
    let now = current.percent();
    let next = match forward {
        true => RENDER_SCALES.iter().copied().find(|p| *p > now).unwrap_or(RENDER_SCALES[0]),
        false => {
            RENDER_SCALES.iter().rev().copied().find(|p| *p < now).unwrap_or(RENDER_SCALES[RENDER_SCALES.len() - 1])
        }
    };
    RenderScale::new(next).unwrap_or(current)
}

pub(super) fn cycle(at: usize, len: usize, forward: bool) -> usize {
    if forward { (at + 1) % len } else { (at + len - 1) % len }
}
