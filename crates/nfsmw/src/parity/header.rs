//! The run header: what a trace records about the run it was taken in, so two recordings can be compared knowing the
//! seed, whether the stock rules were on, every mechanics-changing setting as the world ran it, the mod-style ini files
//! and the mods. One struct for every trace: the pursuit trace's `session.json` records it with the world's part, the
//! UI trace's header without (the front end has no world).

use std::path::Path;
use std::sync::OnceLock;

use serde::Serialize;

use super::{ParityOptions, stock};
use crate::extra_options::Overrides;
use crate::scenes::world::{SessionRules, ViewSource};
use crate::settings::extra_options::ExtraOptions;
use crate::settings::game_moments::GameMoments;
use crate::settings::mods::ModsMode;
use crate::settings::{PursuitOptions, SpeedbreakerTuning, TrafficPolicy, TrafficSettings, Transmission, WorldTuning};

/// What a trace records about its run.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RunHeader {
    /// The run seed (`--seed`); `null` when the clock seeds the run.
    pub seed: Option<u64>,
    /// Whether the stock rules were forced (`--stock-rules`).
    pub stock_rules: bool,
    /// The mod-style ini files next to the config file.
    pub ini_files: Vec<IniFile>,
    /// The Extra Options keys the pursuit reads, as the run uses them.
    pub extra_options: ExtraOptionsRecord,
    /// The mods of the run; `null` when the run started none.
    pub mods: Option<ModsRecord>,
    /// The world's settings as it runs them; absent in a trace taken outside a world.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub world: Option<WorldRecord>,
}

/// One mod-style ini file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IniFile {
    pub name: &'static str,
    /// The file exists next to the config file.
    pub present: bool,
    /// The run read it: present, and not a stock run.
    pub applied: bool,
}

/// The mods of the run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ModsRecord {
    /// `off`, `allowlist` or `all`, after the stock rules.
    pub mode: &'static str,
    /// The ids of the mods that loaded, in load order.
    pub loaded: Vec<String>,
}

/// The Extra Options pursuit keys; `null` is unset.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize)]
pub struct ExtraOptionsRecord {
    /// The forced heat range, minimum and maximum.
    pub heat_bounds: Option<[f32; 2]>,
    pub speeding_limit_mps: Option<f32>,
    pub reckless_limit_mps: Option<f32>,
}

impl From<ExtraOptions> for ExtraOptionsRecord {
    fn from(options: ExtraOptions) -> Self {
        Self {
            heat_bounds: options.heat_bounds().map(|bounds| [bounds.min, bounds.max]),
            speeding_limit_mps: options.speeding_limit(),
            reckless_limit_mps: options.reckless_limit(),
        }
    }
}

/// The session rules of the world (`scenes/world/session_rules.rs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct SessionRulesRecord {
    pub time_scale_allowed: bool,
    pub seed: Option<u64>,
    /// `local_camera` (single player) or `player_cars` (a session).
    pub views: &'static str,
    pub blocking_models: bool,
    pub lockstep_streaming: bool,
    pub race_screens: bool,
}

impl From<SessionRules> for SessionRulesRecord {
    fn from(rules: SessionRules) -> Self {
        Self {
            time_scale_allowed: rules.time_scale_allowed,
            seed: rules.seed,
            views: match rules.views {
                ViewSource::LocalCamera => "local_camera",
                ViewSource::PlayerCars => "player_cars",
            },
            blocking_models: rules.blocking_models,
            lockstep_streaming: rules.lockstep_streaming,
            race_screens: rules.race_screens,
        }
    }
}

/// The traffic settings of the world.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct TrafficRecord {
    /// The active vehicle budget.
    pub cars: u32,
    /// The patrol share, percent.
    pub cop_share: u8,
    /// A budget set by hand in the console, in place of `cars`.
    pub manual: Option<usize>,
    pub lights: bool,
    pub policy: TrafficPolicy,
}

/// What the world gives the header: its state, as it runs.
pub struct WorldState<'a> {
    pub track: &'a str,
    /// The lead player's car folder and gearbox, when a car is driven.
    pub player_car: Option<&'a str>,
    pub transmission: Option<Transmission>,
    pub rules: SessionRules,
    pub tuning: &'a WorldTuning,
    pub traffic: TrafficSettings,
    pub traffic_manual: Option<usize>,
    /// Rain on every track, and the rain amount from 0 to 1.
    pub rain: (bool, f32),
    pub testing: &'a Overrides,
}

/// The world's part of the header: every setting of the stock list (`stock.rs`) with its effective value, and the
/// settings that list leaves to the player.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WorldRecord {
    pub track: String,
    pub player_car: Option<String>,
    /// `automatic` or `manual`.
    pub transmission: Option<String>,
    pub physics_rate_hz: u32,
    pub game_speed: f32,
    pub animation_speed: f32,
    pub no_rev_limiter: bool,
    pub car_scale: f32,
    pub always_rain: bool,
    /// 0 to 1; 1 is the original amount.
    pub rain_amount: f32,
    pub traffic: TrafficRecord,
    pub pursuit_options: PursuitOptions,
    pub game_moments: GameMoments,
    pub speedbreaker: SpeedbreakerTuning,
    pub session_rules: SessionRulesRecord,
    /// The F5 testing overrides.
    pub testing: Overrides,
}

impl From<WorldState<'_>> for WorldRecord {
    fn from(world: WorldState<'_>) -> Self {
        Self {
            track: world.track.to_owned(),
            player_car: world.player_car.map(str::to_owned),
            transmission: world.transmission.map(|transmission| transmission.to_string()),
            physics_rate_hz: world.tuning.physics_rate_hz,
            game_speed: world.tuning.game_speed,
            animation_speed: world.tuning.animation_speed,
            no_rev_limiter: world.tuning.no_rev_limiter,
            car_scale: world.tuning.car_scale,
            always_rain: world.rain.0,
            rain_amount: world.rain.1,
            traffic: TrafficRecord {
                cars: world.traffic.cars,
                cop_share: world.traffic.cop_share.0,
                manual: world.traffic_manual,
                lights: world.traffic.lights,
                policy: world.traffic.policy,
            },
            pursuit_options: world.traffic.pursuit,
            game_moments: world.tuning.game_moments,
            speedbreaker: world.tuning.speedbreaker,
            session_rules: world.rules.into(),
            testing: world.testing.clone(),
        }
    }
}

/// The mods that loaded, told once by the code that starts them (`modding/open.rs`).
static MODS: OnceLock<ModsRecord> = OnceLock::new();

/// Records the mod mode the run used and the mods that loaded. A later call changes nothing.
pub fn record_mods(mode: ModsMode, loaded: &[String]) {
    let mode = match mode {
        ModsMode::Off => "off",
        ModsMode::Allowlist => "allowlist",
        ModsMode::All => "all",
    };
    let _ = MODS.set(ModsRecord { mode, loaded: loaded.to_vec() });
}

/// The state of each mod-style ini file in `dir`, the config file's folder. With `stock` on none is applied.
pub fn ini_files(dir: Option<&Path>, stock: bool) -> Vec<IniFile> {
    stock::ABSENT_INI_FILES
        .iter()
        .map(|&name| {
            let present = dir.is_some_and(|dir| dir.join(name).is_file());
            IniFile { name, present, applied: present && !stock }
        })
        .collect()
}

impl RunHeader {
    /// The header from its parts.
    pub fn new(
        options: ParityOptions,
        ini_files: Vec<IniFile>,
        extra_options: ExtraOptions,
        mods: Option<ModsRecord>,
        world: Option<WorldState<'_>>,
    ) -> Self {
        Self {
            seed: options.seed,
            stock_rules: options.stock_rules,
            ini_files,
            extra_options: extra_options.into(),
            mods,
            world: world.map(WorldRecord::from),
        }
    }

    /// The header of this run: the run's options, the ini files next to the config file, the Extra Options keys in
    /// use and the mods that loaded, with the world's part when there is one.
    pub fn of_run(world: Option<WorldState<'_>>) -> Self {
        let options = super::options();
        let config = crate::settings::Settings::config_path();
        Self::new(
            options,
            ini_files(config.as_deref().and_then(Path::parent), options.stock_rules),
            crate::settings::extra_options::active(),
            MODS.get().cloned(),
            world,
        )
    }
}

#[cfg(test)]
mod tests;
