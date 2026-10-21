//! Native emergency flare timing; `docs/specs/emergency-car-lights.md`.

use std::f32::consts::{FRAC_PI_4, PI, TAU};

pub const RED: u32 = 0x1000;
pub const BLUE: u32 = 0x2000;
pub const WHITE: u32 = 0x4000;

const SHORT_PERIOD: f32 = 0.261_799_4;

/// Raw colour pulse before enabled/broken masks and marker-order accumulation.
pub fn colour_pulse(time: f32) -> [f32; 2] {
    if !time.is_finite() {
        return [0.0; 2];
    }
    let pulse = ((TAU / SHORT_PERIOD) * time).cos().powi(2);
    let gate = (TAU / FRAC_PI_4) * time;
    [gate, gate + PI].map(|angle| match angle.cos() > 0.2 {
        true => pulse,
        false => 0.0,
    })
}

/// First red/blue marker intensities and white at traversal ordinal zero.
#[cfg(test)]
pub fn intensities(time: f32, enabled: u32, broken: u32) -> [f32; 3] {
    let [red, blue] = colour_pulse(time);
    let live = enabled & !broken;
    [
        red * f32::from(u8::from(live & RED != 0)),
        blue * f32::from(u8::from(live & BLUE != 0)),
        white_intensity(time, 0, enabled, broken),
    ]
}

/// White modulation uses the ordinal across the car's entire flare list.
pub fn white_intensity(time: f32, flare_index: usize, enabled: u32, broken: u32) -> f32 {
    if !time.is_finite() || enabled & !broken & WHITE == 0 {
        return 0.0;
    }
    let pulse = ((TAU / SHORT_PERIOD) * time).cos().powi(2);
    let carrier = ((time + 1.5 * flare_index as f32) * 12.0).cos();
    (3.0 * pulse * carrier).sin().clamp(0.0, 1.0)
}

/// Raw multipliers: clamp after multiplying the headlight's base intensity.
/// Native white enable controls this mode even when the white flare is broken.
pub fn headlight_wigwag(time: f32, enabled: u32) -> [f32; 2] {
    if enabled & WHITE == 0 {
        return [1.0; 2];
    }
    if !time.is_finite() {
        return [0.0; 2];
    }
    let left = (12.0 * time).cos();
    [left, 1.0 - left]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emergency_colour_pulses_match_isolated_original_helper_samples() {
        // Independently executed PC helper. Its angle table differs slightly from mathematical
        // cosine; the measured 27-value matrix had maximum raw error below 0.000087.
        for (time, expected) in [
            (0.0, [1.000_007, 0.0]),
            (0.05, [0.131_332_55, 0.0]),
            (0.1, [0.543_662_8, 0.0]),
            (0.2, [0.0, 0.0]),
            (0.3, [0.0, 0.370_158_37]),
            (0.4, [0.0, 0.969_632_86]),
            (0.5, [0.0, 0.712_042_7]),
            (0.75, [0.435_941_13, 0.0]),
            (1.0, [0.0, 0.0]),
        ] {
            for (actual, expected) in colour_pulse(time).into_iter().zip(expected) {
                assert!((actual - expected).abs() < 0.000_1, "time {time}: {actual} vs {expected}");
            }
        }
    }

    #[test]
    fn emergency_flare_colours_follow_opposite_phases_and_broken_masks() {
        let all = RED | BLUE | WHITE;
        let start = intensities(0.0, all, 0);
        assert_eq!(&start[..2], &[1.0, 0.0]);
        assert!((start[2] - 0.141_12).abs() < 0.000_01);
        let other = intensities(PI / 8.0, all, 0);
        assert!(other[0] < 0.000_01 && other[1] > 0.999_99);
        assert_eq!(intensities(0.0, all, RED | WHITE), [0.0; 3]);
        assert_eq!(intensities(PI / 8.0, all, BLUE | WHITE), [0.0; 3]);
        assert_eq!(intensities(0.0, 0, 0), [0.0; 3]);
    }

    #[test]
    fn white_flare_uses_the_whole_list_ordinal_and_headlights_use_raw_carrier() {
        let first = white_intensity(0.0, 0, WHITE, 0);
        let second = white_intensity(0.0, 1, WHITE, 0);
        assert!((first - second).abs() > 0.5);
        assert_eq!(white_intensity(0.0, 0, WHITE, WHITE), 0.0);
        assert_eq!(headlight_wigwag(0.0, WHITE), [1.0, 0.0]);
        let [left, right] = headlight_wigwag(PI / 12.0, WHITE);
        assert!((left + 1.0).abs() < 0.000_01);
        assert!((right - 2.0).abs() < 0.000_01);
        assert_eq!(headlight_wigwag(0.0, 0), [1.0; 2]);
    }

    #[test]
    fn emergency_flare_invalid_time_is_finite_and_dark() {
        for time in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_eq!(intensities(time, RED | BLUE | WHITE, 0), [0.0; 3]);
            assert_eq!(headlight_wigwag(time, WHITE), [0.0; 2]);
        }
    }
}
