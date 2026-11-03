//! Artistic celestial cycle for the optional full day/night feature. Angles and transition times are remake
//! choices; the original game has no night scene.
use glam::{Quat, Vec3};
use nfsmw_data::time_of_day::{sun_direction, sun_phase_limit};

/// One evaluation of the cycle. It keeps three things apart:
/// - `sun` and `moon`: where the bodies are drawn (sky disk, lens glare, sky scattering);
/// - `light`: the direction of direct light and shadows (the moon's at night), never lower than the original sun;
/// - `preset_height`: the sun height the time-of-day presets are read at. It follows the sun, never the moon, so
///   night is never brighter than dusk. Moonlight strength comes from the night blend, not from the presets.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Celestial {
    pub hour: f32,
    pub sun: Vec3,
    pub moon: Vec3,
    pub light: Vec3,
    pub preset_height: f32,
    pub night: f32,
}

fn smooth(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The lowest the original sun gets: its height at the free-roam phase limit. The height does not depend on the
/// azimuth. Direct light is not taken lower, so dusk shadows are no longer than the original's longest.
pub(super) fn light_floor() -> f32 {
    let lo = sun_phase_limit(0);
    sun_direction(lo, lo, 0.0, super::sun::ANGLE_B)[2]
}

/// `v` raised to at least `floor` above the horizon, keeping its azimuth.
fn lift(v: Vec3, floor: f32) -> Vec3 {
    if v.z >= floor {
        return v;
    }
    let flat = Vec3::new(v.x, v.y, 0.0).normalize_or(Vec3::X);
    flat * (1.0 - floor * floor).sqrt() + Vec3::Z * floor
}

pub(super) fn at(hour: f32) -> Celestial {
    let hour = nfsmw_data::day_night::wrap_hour(hour);
    let night = (1.0 - smooth(5.0, 6.5, hour)).max(smooth(17.5, 19.0, hour));
    let angle = (hour - 6.0) * std::f32::consts::PI / 12.0;
    let sun = Vec3::new(angle.cos() * 0.94, 0.34, angle.sin() * 0.94).normalize();
    // The moon sits a quarter turn ahead of the sun in azimuth with its height mirrored, so it rises as the sun sets.
    let moon = (Quat::from_rotation_z(std::f32::consts::FRAC_PI_2) * Vec3::new(sun.x, sun.y, -sun.z)).normalize();
    let floor = light_floor();
    let daytime = lift(sun, floor);
    let nighttime = lift(moon, floor);
    let light = daytime.lerp(nighttime, night).normalize_or(daytime);
    Celestial { hour, sun, moon, light, preset_height: daytime.z, night }
}

/// Automatic player lights: the headlight state to switch to this frame, or `None` to leave the player's choice.
/// `night` is the cycle's night amount while the feature is on (`None` otherwise); `applied` is the side of the
/// dusk/dawn threshold last applied, `None` after a reset (feature off or a new car). Manual toggles between
/// crossings are kept because the same side is never applied twice.
pub(super) fn auto_lights(night: Option<f32>, applied: Option<bool>) -> Option<bool> {
    let night = night? >= 0.5;
    (applied != Some(night)).then_some(night)
}

const NIGHT_STEPS: f32 = 16.0;

fn night_step(night: f32) -> i32 {
    if night <= 0.0 {
        return 0;
    }
    if night >= 1.0 {
        return NIGHT_STEPS as i32 + 1;
    }
    1 + (night * NIGHT_STEPS).floor() as i32
}

/// The cached-reflection key for the cycle: the night amount and the light's angle, in finer steps while day turns to
/// night, and whether local lamps are lit. Constant without the cycle, so it then never invalidates a capture.
pub(super) fn reflection_key(cycle: Option<Celestial>, lamps_on: bool) -> [i32; 4] {
    let Some(c) = cycle else { return [-1, 0, 0, 0] };
    let step = match c.night > 0.0 && c.night < 1.0 {
        true => 1.0_f32.to_radians(),
        false => 4.0_f32.to_radians(),
    };
    let azimuth = c.light.y.atan2(c.light.x);
    let elevation = c.light.z.clamp(-1.0, 1.0).asin();
    [night_step(c.night), (azimuth / step).floor() as i32, (elevation / step).floor() as i32, i32::from(lamps_on)]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cycle_is_continuous_and_wraps_midnight() {
        assert_eq!(at(12.0).night, 0.0);
        assert_eq!(at(0.0).night, 1.0);
        assert!(at(12.0).sun.z > 0.0 && at(0.0).sun.z < 0.0);
        assert!(at(0.0).moon.z > 0.0 && at(12.0).moon.z < 0.0);
        for i in 0..2400 {
            let a = at(i as f32 / 100.0);
            let b = at((i + 1) as f32 / 100.0);
            assert!((a.night - b.night).abs() < 0.02);
            assert!(a.light.distance(b.light) < 0.04);
            assert!((a.preset_height - b.preset_height).abs() < 0.04);
        }
        assert!(at(0.0).light.distance(at(24.0).light) < 1e-5);
    }

    #[test]
    fn the_moon_is_the_sun_turned_a_quarter_in_azimuth_with_its_height_mirrored() {
        for hour in [0.0, 3.0, 7.5, 12.0, 18.0, 21.0] {
            let c = at(hour);
            let expected = Vec3::new(-c.sun.y, c.sun.x, -c.sun.z);
            assert!(c.moon.distance(expected) < 1e-6, "{hour}");
        }
    }

    #[test]
    fn light_never_drops_below_the_original_lowest_sun() {
        let floor = light_floor();
        assert!((0.12..0.16).contains(&floor), "{floor}");
        for i in 0..2400 {
            let c = at(i as f32 / 100.0);
            assert!(c.light.z >= floor - 1e-5, "{}: {}", c.hour, c.light.z);
            assert!((c.light.length() - 1.0).abs() < 1e-5);
        }
        assert!(at(12.0).light.distance(at(12.0).sun) < 1e-6, "a high sun is used unchanged");
    }

    #[test]
    fn presets_never_read_brighter_than_dusk_at_night() {
        let dusk = nfsmw_data::time_of_day::dusk_factor;
        for hour in [19.0, 21.0, 0.0, 2.0, 4.5] {
            let c = at(hour);
            assert!(c.moon.z > c.preset_height, "the moon must not drive the presets at {hour}");
            assert_eq!(dusk(c.preset_height), 1.0, "{hour}");
            assert_eq!(nfsmw_data::time_of_day::shadows::strength(0.0, c.preset_height), 0.1, "{hour}");
        }
        assert!(dusk(at(12.0).preset_height) < 1.0);
    }

    #[test]
    fn automatic_lights_switch_once_per_crossing_and_keep_manual_choices() {
        assert_eq!(auto_lights(None, None), None, "feature off");
        assert_eq!(auto_lights(None, Some(true)), None);
        assert_eq!(auto_lights(Some(0.0), None), Some(false), "first frame applies the current side");
        assert_eq!(auto_lights(Some(1.0), None), Some(true));
        assert_eq!(auto_lights(Some(0.2), Some(false)), None, "same side: a manual toggle survives");
        assert_eq!(auto_lights(Some(0.5), Some(false)), Some(true), "dusk crossing");
        assert_eq!(auto_lights(Some(0.9), Some(true)), None);
        assert_eq!(auto_lights(Some(0.49), Some(true)), Some(false), "dawn crossing");
        // A new car resets the applied side: at noon its default-on headlights are switched off.
        assert_eq!(auto_lights(Some(0.0), None), Some(false));
    }

    #[test]
    fn reflection_key_settles_by_day_and_steps_finely_through_dusk() {
        assert_eq!(reflection_key(None, true), reflection_key(None, false), "no cycle: never invalidates");
        let mut day: Vec<_> = (0..600).map(|i| reflection_key(Some(at(9.0 + i as f32 / 100.0)), false)).collect();
        day.dedup();
        assert!(day.len() < 100, "six daylight hours must reuse captures: {} keys", day.len());
        let noon = reflection_key(Some(at(12.0)), false);
        assert_ne!(noon, reflection_key(Some(at(12.0)), true), "lamps switching on refresh the capture");
        let mut keys = Vec::new();
        for i in 0..=150 {
            keys.push(reflection_key(Some(at(17.5 + i as f32 / 100.0)), false));
        }
        keys.dedup();
        assert!(keys.len() >= 16, "dusk must refresh in fine steps: {}", keys.len());
        assert_ne!(reflection_key(Some(at(17.0)), false)[0], reflection_key(Some(at(17.6)), false)[0]);
    }
}
