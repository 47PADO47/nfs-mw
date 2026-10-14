//! The police helicopter's tunables, read from `NFSMWHelicopterSettings.ini` next to the config file. The keys are the
//! section names of the NFSMWHelicopterOptions mod (`[Helicopter:<Name>]`), so a file written for that mod reads the
//! same numbers; the vanilla values are the install-verified ones of `docs/specs/ai-helicopter.md` (section 2.3 and the
//! constants of section 8), not the mod's defaults (see `docs/provenance/ai-helicopter-tuning.md`).
//!
//! Each numeric key has a value for each heat level 1 to 10 (`heatNN`) and each race level 1 to 10 (`raceNN`). A
//! `default` line sets every level that has no line of its own; with neither, the vanilla value is used. A value outside
//! its range is clamped to the nearest bound; a malformed value is the vanilla value. A missing file is all vanilla.

use std::path::Path;

use blackbox_pursuit::helicopter::Tuning;
use ini_tuning::{Entry, Range, Read, entries, last, read_number, switch};

/// The file's name, next to the config file.
pub const FILE_NAME: &str = "NFSMWHelicopterSettings.ini";

/// The levels of each table: heat 1 to 10, or race 1 to 10.
pub const LEVELS: usize = 10;

/// The install-verified values (`docs/specs/ai-helicopter.md`): `pursuitlevels` per heat level and race level, and the
/// constants of the helicopter's code. Each is the game's own; none is a tuning choice of ours.
pub mod vanilla {
    /// `HeliFuelTime` by heat level 1 to 10, seconds (section 2.3, [verified]).
    pub const FUEL_HEAT: [f32; 10] = [10.0, 10.0, 10.0, 60.0, 90.0, 180.0, 10.0, 75.0, 120.0, 400.0];
    /// `HeliFuelTime` by race level 1 to 10, seconds.
    pub const FUEL_RACE: [f32; 10] = [10.0, 20.0, 60.0, 80.0, 120.0, 120.0, 220.0, 75.0, 120.0, 120.0];
    /// `TimeBetweenHeliActive` by heat level 1 to 10, seconds: the wait before the next helicopter may join.
    pub const RESPAWN_HEAT: [f32; 10] = [0.0, 0.0, 120.0, 180.0, 180.0, 180.0, 180.0, 120.0, 60.0, 5.0];
    /// `TimeBetweenHeliActive` by race level 1 to 10, seconds.
    pub const RESPAWN_RACE: [f32; 10] = [120.0; 10];
    /// `heliLOSdistance` by heat level 1 to 10, metres: 251 for heat 1 to 7, 301 for heat 8 to 10.
    pub const LINE_OF_SIGHT_HEAT: [f32; 10] = [251.0, 251.0, 251.0, 251.0, 251.0, 251.0, 251.0, 301.0, 301.0, 301.0];
    /// `heliLOSdistance` by race level 1 to 10, metres.
    pub const LINE_OF_SIGHT_RACE: [f32; 10] = [251.0, 251.0, 251.0, 251.0, 251.0, 251.0, 251.0, 301.0, 301.0, 301.0];
    /// `SpawnPursuitHelicopter`: metres behind the target on the road.
    pub const SPAWN_DISTANCE: f32 = 250.0;
    /// Metres from the player at which it leaves. The spec does not give this value; 600 is the one the police code has
    /// always used, not yet checked against the install.
    pub const DESPAWN_DISTANCE: f32 = 600.0;
    /// `MAX_SPEED_MPS` of the chopper specs.
    pub const SPEED_CAP: f32 = 100.0;
    /// The search flight, m/s (spec section 6.1).
    pub const SEARCH_SPEED: f32 = 70.0;
    /// The skid hit and the exit, m/s (spec sections 6.1 and 6.2).
    pub const FLY_SPEED: f32 = 100.0;
    /// The skid timer, seconds (spec section 6.1).
    pub const ATTACK_DURATION: f32 = 8.0;
    /// The visual sphere, metres (spec section 3).
    pub const ALL_AROUND_SIGHT: f32 = 75.0;
    /// The skid-hit approach needs no distance gate in the spec's rules; the value is read and held, not applied.
    pub const IGNORE_HELI_SHEET_DISTANCE: f32 = 0.0;
}

/// Metres, seconds and speeds: the ranges a value may take before it is clamped.
const METRES: Range = Range { min: 0.0, max: 5000.0 };
const SECONDS: Range = Range { min: 0.0, max: 3600.0 };
const SPEED: Range = Range { min: 1.0, max: 1000.0 };
const SPEED_CAP: Range = Range { min: 1.0, max: 10000.0 };
const SECONDS_MIN: Range = Range { min: 0.1, max: 60.0 };

/// One tunable at each heat level and each race level, 1 to 10. Index 0 is level 1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Levels {
    pub heat: [f32; LEVELS],
    pub race: [f32; LEVELS],
}

impl Levels {
    /// The same value at every level.
    pub const fn flat(value: f32) -> Self {
        Self { heat: [value; LEVELS], race: [value; LEVELS] }
    }

    /// The value at `level` (clamped to 1 to 10), of the race table when `racing`, else the heat table.
    pub fn at(&self, level: u32, racing: bool) -> f32 {
        let index = (level as usize).clamp(1, LEVELS) - 1;
        match racing {
            true => self.race[index],
            false => self.heat[index],
        }
    }
}

/// Every helicopter tunable the game reads from the file. [`Default`] is the vanilla tuning.
#[derive(Debug, Clone, PartialEq)]
pub struct HeliSettings {
    pub spawn_distance: Levels,
    pub despawn_distance: Levels,
    pub respawning: Levels,
    pub fuel_time: Levels,
    pub line_of_sight: Levels,
    pub speed_cap: Levels,
    pub fly_speed: Levels,
    pub search_speed: Levels,
    pub attack_duration: Levels,
    pub all_around_sight: Levels,
    /// Read and held. Not applied yet: the spec's `CheckHeliSheet` has no distance gate to put it in.
    pub ignore_heli_sheet_distance: Levels,
    /// Beyond `all_around_sight`, the helicopter sees only ahead (`FrontSightOnly`).
    pub front_sight_only: bool,
    /// Whether a hiding spot hides the player from the helicopter (`HidingSpots`).
    pub hiding_spots: bool,
    /// Whether the track's height sheet floors the helicopter (`HeliSheet`); off, it keeps a flat floor.
    pub heli_sheet: bool,
}

impl Default for HeliSettings {
    fn default() -> Self {
        Self {
            spawn_distance: Levels::flat(vanilla::SPAWN_DISTANCE),
            despawn_distance: Levels::flat(vanilla::DESPAWN_DISTANCE),
            respawning: Levels { heat: vanilla::RESPAWN_HEAT, race: vanilla::RESPAWN_RACE },
            fuel_time: Levels { heat: vanilla::FUEL_HEAT, race: vanilla::FUEL_RACE },
            line_of_sight: Levels { heat: vanilla::LINE_OF_SIGHT_HEAT, race: vanilla::LINE_OF_SIGHT_RACE },
            speed_cap: Levels::flat(vanilla::SPEED_CAP),
            fly_speed: Levels::flat(vanilla::FLY_SPEED),
            search_speed: Levels::flat(vanilla::SEARCH_SPEED),
            attack_duration: Levels::flat(vanilla::ATTACK_DURATION),
            all_around_sight: Levels::flat(vanilla::ALL_AROUND_SIGHT),
            ignore_heli_sheet_distance: Levels::flat(vanilla::IGNORE_HELI_SHEET_DISTANCE),
            front_sight_only: true,
            hiding_spots: true,
            heli_sheet: true,
        }
    }
}

impl HeliSettings {
    /// The helicopter's model tuning for `level` (heat or race). `skid_hit` is not in the file: the level's
    /// `pursuitsupport` decides it, so the caller sets it.
    pub fn tuning(&self, level: u32, racing: bool) -> Tuning {
        Tuning {
            spawn_distance: self.spawn_distance.at(level, racing),
            fuel_time: self.fuel_time.at(level, racing),
            line_of_sight: self.line_of_sight.at(level, racing),
            despawn_distance: self.despawn_distance.at(level, racing),
            speed_cap: self.speed_cap.at(level, racing),
            skid_hit: false,
            attack_duration: self.attack_duration.at(level, racing),
            search_speed: self.search_speed.at(level, racing),
            fly_speed: self.fly_speed.at(level, racing),
            all_around_sight: self.all_around_sight.at(level, racing),
            front_sight_only: self.front_sight_only,
            hiding_spots: self.hiding_spots,
        }
    }

    /// The seconds before the next helicopter may join at `level`.
    pub fn respawn_secs(&self, level: u32, racing: bool) -> f32 {
        self.respawning.at(level, racing)
    }
}

/// The settings in the file next to the config file. A missing file is the vanilla tuning.
pub fn load() -> HeliSettings {
    match crate::settings::Settings::config_path().as_deref().and_then(Path::parent) {
        Some(dir) => read(&dir.join(FILE_NAME)),
        None => HeliSettings::default(),
    }
}

/// The settings in the file at `path`. A missing file is the vanilla tuning.
pub fn read(path: &Path) -> HeliSettings {
    match std::fs::read_to_string(path) {
        Ok(text) => parse(&text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => HeliSettings::default(),
        Err(e) => {
            log::warn!("cannot read {}: {e}", path.display());
            HeliSettings::default()
        }
    }
}

/// The tunables from the text of the file.
pub fn parse(text: &str) -> HeliSettings {
    let e = entries(text);
    let d = HeliSettings::default();
    HeliSettings {
        spawn_distance: levels(&e, "Helicopter:SpawnDistance", METRES, d.spawn_distance),
        despawn_distance: levels(&e, "Helicopter:DespawnDistance", METRES, d.despawn_distance),
        respawning: levels(&e, "Helicopter:Respawning", SECONDS, d.respawning),
        fuel_time: levels(&e, "Helicopter:FuelTime", SECONDS, d.fuel_time),
        line_of_sight: levels(&e, "Helicopter:LineOfSight", METRES, d.line_of_sight),
        speed_cap: levels(&e, "Helicopter:SpeedCap", SPEED_CAP, d.speed_cap),
        fly_speed: levels(&e, "Helicopter:FlySpeed", SPEED, d.fly_speed),
        search_speed: levels(&e, "Helicopter:SearchSpeed", SPEED, d.search_speed),
        attack_duration: levels(&e, "Helicopter:AttackDuration", SECONDS_MIN, d.attack_duration),
        all_around_sight: levels(&e, "Helicopter:AllAroundSight", METRES, d.all_around_sight),
        ignore_heli_sheet_distance: levels(
            &e,
            "Helicopter:IgnoreHeliSheetDistance",
            METRES,
            d.ignore_heli_sheet_distance,
        ),
        front_sight_only: flag(&e, "Helicopter:FrontSightOnly", d.front_sight_only),
        hiding_spots: flag(&e, "Helicopter:HidingSpots", d.hiding_spots),
        heli_sheet: flag(&e, "Helicopter:HeliSheet", d.heli_sheet),
    }
}

/// One tunable's table from `section`: the `heatNN` and `raceNN` lines, else the `default` line, else `vanilla`.
fn levels(e: &[Entry], section: &str, range: Range, vanilla: Levels) -> Levels {
    let fallback = number(e, section, "default", range);
    Levels {
        heat: row(e, section, "heat", range, fallback, &vanilla.heat),
        race: row(e, section, "race", range, fallback, &vanilla.race),
    }
}

/// The 10 values of one prefix (`heat` or `race`).
fn row(
    e: &[Entry],
    section: &str,
    prefix: &str,
    range: Range,
    fallback: Option<f32>,
    vanilla: &[f32; LEVELS],
) -> [f32; LEVELS] {
    std::array::from_fn(|i| {
        let key = format!("{prefix}{:02}", i + 1);
        number(e, section, &key, range).or(fallback).unwrap_or(vanilla[i])
    })
}

/// A number from `section.key`, through `range`. Absent gives `None`; clamped and malformed lines are logged.
fn number(e: &[Entry], section: &str, key: &str, range: Range) -> Option<f32> {
    let read = read_number(e, section, key, range);
    note(section, key, read);
    read.value()
}

/// A switch from `section.default`, or `vanilla` when absent or not a switch.
fn flag(e: &[Entry], section: &str, vanilla: bool) -> bool {
    let Some(entry) = last(e, section, "default") else { return vanilla };
    match switch(&entry.value) {
        Some(on) => on,
        None => {
            log::warn!("helicopter: {section} default = {:?} is not a switch, using the vanilla value", entry.value);
            vanilla
        }
    }
}

/// Logs the lines the file got wrong. Both cases fall back or clamp, so the game still starts.
fn note(section: &str, key: &str, read: Read) {
    match read {
        Read::Clamped(v) => log::warn!("helicopter: [{section}] {key} is out of range, using {v}"),
        Read::Malformed => log::warn!("helicopter: [{section}] {key} is not a number, using the vanilla value"),
        Read::Absent | Read::Value(_) => {}
    }
}

#[cfg(test)]
#[path = "helicopter_tests.rs"]
mod tests;
