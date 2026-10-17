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

/// The sharpening levels the row cycles through (off, medium, high) as the sharpness percent each one sets.
const SHARPENING: [u8; 3] = [0, 50, 80];
/// The sharpness above which the row reads High: halfway between Medium and High.
const SHARPENING_HIGH_FROM: u8 = 66;

/// The level a sharpness percent reads as: 0 is Off, up to 65 Medium, above that High. A value set from the console
/// or the config file between the levels reads as the nearest one.
fn sharpening_level(p: Percent) -> usize {
    match p.0 {
        0 => 0,
        1..SHARPENING_HIGH_FROM => 1,
        _ => 2,
    }
}

pub(super) fn sharpening_name(p: Percent) -> &'static str {
    ["Off", "Medium", "High"][sharpening_level(p)]
}

/// The next sharpening level (`forward`) or the one before it, wrapping around.
pub(super) fn next_sharpening(current: Percent, forward: bool) -> Percent {
    Percent(SHARPENING[cycle(sharpening_level(current), SHARPENING.len(), forward)])
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sharpening_cycles_off_medium_high_and_reads_values_between_as_the_nearest_level() {
        let names: Vec<_> = [0, 1, 50, 65, 66, 80, 100].map(|p| sharpening_name(Percent(p))).to_vec();
        assert_eq!(names, ["Off", "Medium", "Medium", "Medium", "High", "High", "High"]);
        let mut p = Percent(0);
        let mut seen = Vec::new();
        for _ in 0..4 {
            p = next_sharpening(p, true);
            seen.push(p.0);
        }
        assert_eq!(seen, [50, 80, 0, 50]);
        assert_eq!(next_sharpening(Percent(0), false), Percent(80), "back from Off wraps to High");
        assert_eq!(next_sharpening(Percent(90), true), Percent(0), "a value above High counts as High");
    }
}
