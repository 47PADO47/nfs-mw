//! Colourful tire smoke: the colour of a puff for the chosen mode (`smoke_color_mode`, `smoke_color`), by pure functions that
//! the smoke of the game and a mod's tire smoke variant both call while they build their quads. Nothing here allocates or
//! keeps state: a puff carries at most one number (its level, see [`level_of`]).

use glam::Vec3;

use crate::settings::{SmokeColorMode, SmokeTint};

/// The speed, in metres a second, at which "By Speed" has reached the chosen colour.
pub const FULL_SPEED: f32 = 60.0;
/// How fast the rainbow turns: cycles a second, and cycles for each metre along the ground.
const CYCLES_PER_SECOND: f32 = 0.35;
const CYCLES_PER_METRE: f32 = 0.12;
/// How saturated and how bright the rainbow is.
const RAINBOW_SATURATION: f32 = 0.85;

/// The number a puff is born with for the mode, 0 to 1: the speed over [`FULL_SPEED`] for "By Speed", the wheel's smoke (its
/// slip) for "By Slip", 0 for the others, which do not use it.
pub fn level_of(mode: SmokeColorMode, speed: f32, slip: f32) -> f32 {
    match mode {
        SmokeColorMode::Speed => (speed / FULL_SPEED).clamp(0.0, 1.0),
        SmokeColorMode::Slip => slip.clamp(0.0, 1.0),
        _ => 0.0,
    }
}

/// Where in the rainbow a puff is at `clock` seconds, at `position` (render space, `z` up): the hue turns with time and runs
/// along the ground, so a trail shows the colours one after another.
pub fn rainbow_phase(clock: f32, position: Vec3) -> f32 {
    clock * CYCLES_PER_SECOND + (position.x + position.y) * CYCLES_PER_METRE
}

/// The colour of a puff whose own colour is `base`: `Off` keeps it; `Solid` is the chosen colour; `Rainbow` is a hue of
/// `phase` (its fractional part, one turn of the wheel); `Speed` and `Slip` go from `base` to the chosen colour with `level`.
pub fn smoke_rgb(tint: SmokeTint, base: [u8; 3], phase: f32, level: f32) -> [u8; 3] {
    match tint.mode {
        SmokeColorMode::Off => base,
        SmokeColorMode::Solid => tint.color.0,
        SmokeColorMode::Rainbow => hsv(phase.rem_euclid(1.0), RAINBOW_SATURATION, 1.0),
        SmokeColorMode::Speed | SmokeColorMode::Slip => mix(base, tint.color.0, level.clamp(0.0, 1.0)),
    }
}

fn mix(from: [u8; 3], to: [u8; 3], t: f32) -> [u8; 3] {
    std::array::from_fn(|i| (f32::from(from[i]) + (f32::from(to[i]) - f32::from(from[i])) * t).round() as u8)
}

/// A colour of the wheel: hue `h` in turns (0 to 1), saturation and value 0 to 1.
fn hsv(h: f32, s: f32, v: f32) -> [u8; 3] {
    let sector = h * 6.0;
    let (index, fraction) = (sector.floor() as i32 % 6, sector.fract());
    let (p, q, t) = (v * (1.0 - s), v * (1.0 - s * fraction), v * (1.0 - s * (1.0 - fraction)));
    let (r, g, b) = match index {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    [r, g, b].map(|channel| (channel.clamp(0.0, 1.0) * 255.0).round() as u8)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::SmokeColor;

    const GREY: [u8; 3] = [172, 174, 177];

    fn tint(mode: SmokeColorMode, rgb: [u8; 3]) -> SmokeTint {
        SmokeTint { mode, color: SmokeColor(rgb) }
    }

    #[test]
    fn off_keeps_the_smokes_own_colour_whatever_the_chosen_one() {
        let off = tint(SmokeColorMode::Off, [255, 0, 0]);
        assert_eq!(smoke_rgb(off, GREY, 0.3, 0.9), GREY);
    }

    #[test]
    fn solid_is_the_chosen_colour() {
        let solid = tint(SmokeColorMode::Solid, [10, 200, 30]);
        assert_eq!(smoke_rgb(solid, GREY, 0.7, 0.2), [10, 200, 30]);
    }

    #[test]
    fn speed_and_slip_go_from_the_base_to_the_chosen_colour() {
        for mode in [SmokeColorMode::Speed, SmokeColorMode::Slip] {
            let t = tint(mode, [0, 100, 255]);
            assert_eq!(smoke_rgb(t, [100, 100, 100], 0.0, 0.0), [100, 100, 100], "level 0 is the base");
            assert_eq!(smoke_rgb(t, [100, 100, 100], 0.0, 1.0), [0, 100, 255], "level 1 is the chosen colour");
            assert_eq!(smoke_rgb(t, [100, 100, 100], 0.0, 0.5), [50, 100, 178]);
            assert_eq!(smoke_rgb(t, [100, 100, 100], 0.0, 7.0), [0, 100, 255], "a level past 1 is clamped");
        }
    }

    #[test]
    fn the_level_a_puff_is_born_with_follows_the_mode() {
        assert_eq!(level_of(SmokeColorMode::Speed, 30.0, 0.9), 0.5);
        assert_eq!(level_of(SmokeColorMode::Speed, 600.0, 0.0), 1.0);
        assert_eq!(level_of(SmokeColorMode::Slip, 30.0, 0.25), 0.25);
        assert_eq!(level_of(SmokeColorMode::Slip, 30.0, 4.0), 1.0);
        for mode in [SmokeColorMode::Off, SmokeColorMode::Solid, SmokeColorMode::Rainbow] {
            assert_eq!(level_of(mode, 50.0, 1.0), 0.0);
        }
    }

    #[test]
    fn the_rainbow_walks_the_wheel_and_repeats_every_turn() {
        let rainbow = tint(SmokeColorMode::Rainbow, [1, 2, 3]);
        let at = |phase: f32| smoke_rgb(rainbow, GREY, phase, 0.0);
        assert_eq!(at(0.0), [255, 38, 38], "red at the start");
        assert_eq!(at(1.0 / 3.0), [38, 255, 38], "green a third round");
        assert_eq!(at(2.0 / 3.0), [38, 38, 255], "blue two thirds round");
        assert_eq!(at(0.25), at(1.25), "one turn later is the same colour");
        assert_eq!(at(-0.75), at(0.25), "a negative phase wraps");
        assert_ne!(at(0.1), at(0.2));
    }

    #[test]
    fn the_rainbow_runs_along_the_ground_and_with_time() {
        let here = rainbow_phase(0.0, Vec3::ZERO);
        assert!(rainbow_phase(1.0, Vec3::ZERO) > here, "time turns it");
        assert!(rainbow_phase(0.0, Vec3::new(5.0, 0.0, 0.0)) > here, "a puff further along is a later hue");
        assert_eq!(rainbow_phase(0.0, Vec3::new(0.0, 0.0, 9.0)), here, "height does not");
    }
}
