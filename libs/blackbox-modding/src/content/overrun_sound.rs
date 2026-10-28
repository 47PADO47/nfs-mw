//! `[car_sound.overrun]`: the numbers of the accumulator backfire a mod car can use in place of the core rule "backfire
//! when the throttle is lifted at high engine speed" (docs/modding/car-sound.md, "The overrun backfire").
//!
//! The table only carries numbers; the controller that reads them is `blackbox_carsound::overrun` in the host. All eleven
//! keys are required, so a half-written table is refused rather than filled with values nobody chose. The ranges are the
//! ones the documentation promises; a value outside them (or not a number) refuses the car.

use serde::Deserialize;

use super::ContentError;

/// The accumulator backfire of one car. Rates, thresholds and probabilities are 0 to 1, the two times 0 to 10 seconds and
/// the charge rate 0 to 5 per second.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Overrun {
    /// Charge per second at full revs and full throttle (0 to 5).
    pub accumulator_rate: f32,
    /// The throttle falling through this arms a bang.
    pub throttle_arm: f32,
    /// The throttle falling through this fires the armed bang.
    pub throttle_fire: f32,
    /// The longest time between arming and firing for the bang to happen, seconds (0 to 10).
    pub window_seconds: f32,
    /// Seconds of a shut throttle before the flames start (0 to 10).
    pub flame_delay: f32,
    /// Flames need an engine speed (as a fraction of the redline) at or above this.
    pub flame_min_rpm: f32,
    /// A bang needs a strength above this to be heard.
    pub bang_audio_threshold: f32,
    /// A bang above this strength also lights the pipes.
    pub bang_effect_threshold: f32,
    /// A flame above this strength also lights the pipes.
    pub flame_effect_threshold: f32,
    /// Chance of a bang whose strength is high enough, with the driver's throttle shut.
    pub probability_closed: f32,
    /// The same chance with the driver's throttle fully open.
    pub probability_open: f32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawOverrun {
    accumulator_rate: f32,
    throttle_arm: f32,
    throttle_fire: f32,
    window_seconds: f32,
    flame_delay: f32,
    flame_min_rpm: f32,
    bang_audio_threshold: f32,
    bang_effect_threshold: f32,
    flame_effect_threshold: f32,
    probability_closed: f32,
    probability_open: f32,
}

/// Check the table: every number finite and inside its range.
pub(super) fn check(raw: Option<RawOverrun>) -> Result<Option<Overrun>, ContentError> {
    let Some(raw) = raw else { return Ok(None) };
    let unit = |field: &'static str, value: f32| -> Result<f32, ContentError> {
        if (0.0..=1.0).contains(&value) {
            return Ok(value);
        }
        Err(ContentError::Field { field, reason: "a number from 0 to 1" })
    };
    let within = |field: &'static str, value: f32, most: f32| -> Result<f32, ContentError> {
        if (0.0..=most).contains(&value) {
            return Ok(value);
        }
        Err(ContentError::Field { field, reason: "a number inside the documented range" })
    };
    Ok(Some(Overrun {
        accumulator_rate: within("car_sound.overrun.accumulator_rate", raw.accumulator_rate, 5.0)?,
        throttle_arm: unit("car_sound.overrun.throttle_arm", raw.throttle_arm)?,
        throttle_fire: unit("car_sound.overrun.throttle_fire", raw.throttle_fire)?,
        window_seconds: within("car_sound.overrun.window_seconds", raw.window_seconds, 10.0)?,
        flame_delay: within("car_sound.overrun.flame_delay", raw.flame_delay, 10.0)?,
        flame_min_rpm: unit("car_sound.overrun.flame_min_rpm", raw.flame_min_rpm)?,
        bang_audio_threshold: unit("car_sound.overrun.bang_audio_threshold", raw.bang_audio_threshold)?,
        bang_effect_threshold: unit("car_sound.overrun.bang_effect_threshold", raw.bang_effect_threshold)?,
        flame_effect_threshold: unit("car_sound.overrun.flame_effect_threshold", raw.flame_effect_threshold)?,
        probability_closed: unit("car_sound.overrun.probability_closed", raw.probability_closed)?,
        probability_open: unit("car_sound.overrun.probability_open", raw.probability_open)?,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::engine_sound::{CarEffects, RawEffects, check_effects};

    #[derive(Deserialize)]
    struct File {
        car_sound: Option<RawEffects>,
    }

    fn effects(text: &str) -> Result<Option<CarEffects>, ContentError> {
        let file: File = toml::from_str(text).map_err(|error| ContentError::Parse(error.to_string()))?;
        check_effects(file.car_sound)
    }

    const TABLE: &str = "[car_sound.overrun]\naccumulator_rate = 0.2\nthrottle_arm = 0.9\nthrottle_fire = 0.1\n\
        window_seconds = 2.0\nflame_delay = 0.2\nflame_min_rpm = 0.1\nbang_audio_threshold = 0.95\n\
        bang_effect_threshold = 0.8\nflame_effect_threshold = 1.0\nprobability_closed = 0.8\nprobability_open = 0.3\n";

    #[test]
    fn a_whole_table_is_read_and_is_a_table_on_its_own() {
        let got = effects(TABLE).unwrap().unwrap().overrun.unwrap();
        assert_eq!((got.accumulator_rate, got.throttle_arm, got.throttle_fire), (0.2, 0.9, 0.1));
        assert_eq!((got.window_seconds, got.flame_delay, got.flame_min_rpm), (2.0, 0.2, 0.1));
        assert_eq!((got.bang_audio_threshold, got.bang_effect_threshold, got.flame_effect_threshold), (0.95, 0.8, 1.0));
        assert_eq!((got.probability_closed, got.probability_open), (0.8, 0.3));
    }

    #[test]
    fn the_table_sits_beside_the_lists_and_a_car_without_it_has_none() {
        let text = format!("[car_sound]\nbackfire = [\"SOUND/A/B.WAV\"]\n{TABLE}");
        let got = effects(&text).unwrap().unwrap();
        assert_eq!(got.backfire.len(), 1);
        assert!(got.overrun.is_some());
        let plain = effects("[car_sound]\nbackfire = [\"SOUND/A/B.WAV\"]\n").unwrap().unwrap();
        assert!(plain.overrun.is_none());
    }

    #[test]
    fn a_missing_key_an_unknown_key_and_a_value_out_of_range_are_refused() {
        let without = TABLE.replace("flame_delay = 0.2\n", "");
        let extra = format!("{TABLE}pop_rate = 1.0\n");
        let mut cases = vec![("a missing key", without), ("an unknown key", extra)];
        for (key, bad) in [
            ("accumulator_rate = 0.2", "accumulator_rate = 5.5"),
            ("throttle_arm = 0.9", "throttle_arm = 1.5"),
            ("throttle_fire = 0.1", "throttle_fire = -0.1"),
            ("window_seconds = 2.0", "window_seconds = 11.0"),
            ("flame_delay = 0.2", "flame_delay = -1.0"),
            ("flame_min_rpm = 0.1", "flame_min_rpm = 2.0"),
            ("bang_audio_threshold = 0.95", "bang_audio_threshold = 1.01"),
            ("bang_effect_threshold = 0.8", "bang_effect_threshold = -0.5"),
            ("flame_effect_threshold = 1.0", "flame_effect_threshold = 3.0"),
            ("probability_closed = 0.8", "probability_closed = 1.1"),
            ("probability_open = 0.3", "probability_open = nan"),
        ] {
            cases.push((bad, TABLE.replace(key, bad)));
        }
        for (what, text) in cases {
            assert!(effects(&text).is_err(), "{what}");
        }
    }

    #[test]
    fn the_range_edges_are_allowed() {
        let edge = TABLE
            .replace("accumulator_rate = 0.2", "accumulator_rate = 5.0")
            .replace("window_seconds = 2.0", "window_seconds = 10.0")
            .replace("flame_delay = 0.2", "flame_delay = 0.0")
            .replace("probability_open = 0.3", "probability_open = 0.0");
        assert!(effects(&edge).unwrap().is_some());
    }
}
