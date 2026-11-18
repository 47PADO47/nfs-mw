//! The stock-rules switch (`--stock-rules`, `NFSMW_STOCK_RULES=1`): everything a run forces so that its mechanics are
//! the original game's, whatever the config file, the mod-style ini files and the saved options say. This file is the
//! whole list; the places that read a setting call into it.
//!
//! | Forced | To | Where it is applied |
//! |---|---|---|
//! | The settings in [`force_settings`] | the values there | `Settings::load`, and again where the world takes its settings (`WorldTuning::from`, `TrafficSettings::from`), so a change made in a menu during the run does not reach the simulation |
//! | The files in [`ABSENT_INI_FILES`] | treated as absent | their loaders, through [`ini_absent`] |
//! | Native mods | [`MODS`]: none load | `ModsSettings::load`, through [`mods_mode`] |
//! | The F5 testing overrides | [`force_testing`]: all off | the world, each frame |
//!
//! Nothing here writes the config file: the forced values exist in memory only.
//!
//! Left as the player set them, because no doc or code states the original's value or because the original has the
//! same player option: `cop_share`, `always_rain`, `transmission`, the Game Moment Camera's main switch, the control
//! deadzones and sensitivities, the wheel options, and the music-only `eatrax_jukebox.toml`.

use crate::extra_options::Overrides;
use crate::settings::game_moments::GameMoments;
use crate::settings::mods::ModsMode;
use crate::settings::{
    DEFAULT_ANIMATION_SPEED, DEFAULT_CAR_SCALE, DEFAULT_GAME_SPEED, DEFAULT_PHYSICS_RATE_HZ, DEFAULT_RAIN_AMOUNT,
    DEFAULT_TRAFFIC, PursuitOptions, Settings, SpeedbreakerTuning, TrafficPolicy, bartender, extra_options, fairbusts,
    helicopter, speedbreaker,
};

/// The mod-style ini files next to the config file that a stock run does not read: the original has none of them.
pub const ABSENT_INI_FILES: [&str; 5] = [
    speedbreaker::FILE_NAME,
    extra_options::FILE_NAME,
    bartender::FILE_NAME,
    fairbusts::FILE_NAME,
    helicopter::FILE_NAME,
];

/// A stock run loads no native mod: mods can change cars, traffic and rules.
pub const MODS: ModsMode = ModsMode::Off;

/// Sets every mechanics-changing setting to the original's value.
pub fn force_settings(settings: &mut Settings) {
    // The original has no traffic lights.
    settings.traffic_lights = false;
    // The installed PC reference's vehicle budget, and its spawn and retire rules (the Reference preset).
    settings.traffic = DEFAULT_TRAFFIC;
    settings.traffic_policy = TrafficPolicy::default();
    // The reference pursuit limits, and the original's bounty rules with their bugs.
    settings.pursuit_options = PursuitOptions { enhanced: false, bounty_fixes: false, ..PursuitOptions::default() };
    // The original's step and clock rates.
    settings.physics_rate_hz = DEFAULT_PHYSICS_RATE_HZ;
    settings.game_speed = DEFAULT_GAME_SPEED;
    settings.animation_speed = DEFAULT_ANIMATION_SPEED;
    // The rev limiter is on.
    settings.no_rev_limiter = false;
    // The original's car size and rain amount.
    settings.car_scale = DEFAULT_CAR_SCALE;
    settings.rain_amount = DEFAULT_RAIN_AMOUNT;
    // The vanilla Speedbreaker (`NFSMWSpeedFixerSettings.ini` and the Extra Options keys absent).
    settings.speedbreaker = SpeedbreakerTuning::default();
    // The original's one switch covers every kind of game moment; the switch itself stays the player's.
    settings.game_moments = GameMoments { camera: settings.game_moments.camera, ..GameMoments::default() };
}

/// Clears the F5 testing overrides (traffic off, police off, locked heat, career bin, unlimited nitrous and
/// Speedbreaker, pause).
pub fn force_testing(testing: &mut Overrides) {
    *testing = Overrides::default();
}

/// Whether this run treats the ini file `name` as absent.
pub fn ini_absent(name: &str) -> bool {
    super::stock_rules() && ABSENT_INI_FILES.contains(&name)
}

/// The mod mode this run uses, given the one the settings ask for.
pub fn mods_mode(asked: ModsMode) -> ModsMode {
    match super::stock_rules() {
        true => MODS,
        false => asked,
    }
}

/// The settings the world runs by: `settings` as they are, or with the stock values forced in a stock run.
pub fn effective(settings: &Settings) -> Settings {
    let mut settings = *settings;
    if super::stock_rules() {
        force_settings(&mut settings);
    }
    settings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{Partial, Percent, Transmission};

    /// Settings in which every forced value is deliberately not the original's.
    fn modded() -> Settings {
        let mut speedbreaker = SpeedbreakerTuning::default();
        speedbreaker.min_speed *= 2.0;
        speedbreaker.max_duration = None;
        speedbreaker.dilation.target = 0.9;
        speedbreaker.physics.car_mass_scale = 9.0;
        Settings {
            traffic_lights: true,
            traffic: 55,
            traffic_policy: TrafficPolicy { custom: true, density_percent: 40, ..TrafficPolicy::extended() },
            pursuit_options: PursuitOptions {
                enhanced: true,
                max_active_cops: 64,
                heat_cap: 7,
                wave_percent: 300,
                bounty_fixes: true,
            },
            physics_rate_hz: 144,
            game_speed: 2.0,
            animation_speed: 3.0,
            no_rev_limiter: true,
            car_scale: 2.0,
            rain_amount: Percent(40),
            speedbreaker,
            game_moments: GameMoments { camera: false, breakers: false, pursuit: false, jumps: false, crashes: false },
            // Not forced: no original value is stated, or the original has the same player option.
            cop_share: Percent(60),
            always_rain: true,
            transmission: Transmission::Manual,
            ..Settings::from(Partial::default())
        }
    }

    #[test]
    fn every_forced_setting_takes_the_originals_value() {
        let mut forced = modded();
        force_settings(&mut forced);
        assert!(!forced.traffic_lights, "the original has no traffic lights");
        assert_eq!(forced.traffic, 20);
        let policy = forced.traffic_policy;
        assert!(!policy.custom && !policy.keep_on_minimap && !policy.common_fill);
        assert_eq!((policy.spawn_min, policy.spawn_max, policy.retire_distance), (300, 400, 650));
        assert_eq!((policy.offscreen_seconds, policy.density_percent, policy.common_rate_percent), (4, 100, 100));
        let pursuit = forced.pursuit_options;
        assert!(!pursuit.enhanced && !pursuit.bounty_fixes);
        assert_eq!((pursuit.max_active_cops, pursuit.heat_cap, pursuit.wave_percent), (16, 0, 100));
        assert_eq!((pursuit.active_limit(8), pursuit.wave_scale(), pursuit.maximum_heat()), (8, 1.0, None));
        assert_eq!(forced.physics_rate_hz, 60);
        assert_eq!((forced.game_speed, forced.animation_speed, forced.car_scale), (1.0, 1.0, 1.0));
        assert!(!forced.no_rev_limiter, "the rev limiter is on");
        assert_eq!(forced.rain_amount, Percent(100));
        assert_eq!(forced.speedbreaker, SpeedbreakerTuning::default());
        let moments = forced.game_moments;
        assert!(moments.breakers && moments.pursuit && moments.jumps && moments.crashes);
    }

    #[test]
    fn the_unresolved_settings_and_everything_else_stay_as_set() {
        let before = modded();
        let mut forced = before;
        force_settings(&mut forced);
        assert_eq!(forced.cop_share, Percent(60));
        assert!(forced.always_rain);
        assert_eq!(forced.transmission, Transmission::Manual);
        assert!(!forced.game_moments.camera, "the main switch is the player's, as in the original");
        let listed = Settings {
            traffic_lights: forced.traffic_lights,
            traffic: forced.traffic,
            traffic_policy: forced.traffic_policy,
            pursuit_options: forced.pursuit_options,
            physics_rate_hz: forced.physics_rate_hz,
            game_speed: forced.game_speed,
            animation_speed: forced.animation_speed,
            no_rev_limiter: forced.no_rev_limiter,
            car_scale: forced.car_scale,
            rain_amount: forced.rain_amount,
            speedbreaker: forced.speedbreaker,
            game_moments: forced.game_moments,
            ..before
        };
        assert_eq!(forced, listed, "only the listed settings change");
    }

    #[test]
    fn the_defaults_differ_from_stock_only_in_the_rewrites_extensions() {
        let defaults = Settings::from(Partial::default());
        let mut forced = defaults;
        force_settings(&mut forced);
        let mut pursuit_options = defaults.pursuit_options;
        pursuit_options.bounty_fixes = false;
        assert_eq!(forced, Settings { traffic_lights: false, pursuit_options, ..defaults });
    }

    #[test]
    fn the_testing_overrides_are_cleared() {
        let mut testing = Overrides {
            traffic_off: true,
            police_off: true,
            unlimited_nos: true,
            unlimited_breaker: true,
            paused: true,
            lock_heat: true,
            heat: 6,
            career_bin: 9,
        };
        force_testing(&mut testing);
        assert!(!testing.active());
        assert!(!testing.traffic_off && !testing.police_off && !testing.unlimited_nos && !testing.unlimited_breaker);
        assert!(!testing.paused && !testing.lock_heat);
        assert_eq!((testing.heat, testing.career_bin), (1, 0));
    }

    #[test]
    fn the_mod_style_files_and_the_mods_are_off() {
        for name in [
            "NFSMWSpeedFixerSettings.ini",
            "NFSMWExtraOptionsSettings.ini",
            "NFSMWBartenderSettings.ini",
            "NFSMWFairBustsSettings.ini",
            "NFSMWHelicopterSettings.ini",
        ] {
            assert!(ABSENT_INI_FILES.contains(&name), "{name}");
        }
        assert_eq!(MODS, ModsMode::Off);
    }

    #[test]
    fn without_the_switch_nothing_is_forced() {
        // Tests never set the run's options.
        assert_eq!(effective(&modded()), modded());
        assert!(!ini_absent("NFSMWBartenderSettings.ini"));
        assert_eq!(mods_mode(ModsMode::All), ModsMode::All);
    }
}
