//! A race in the world: the player and the AI racers on a course of checkpoints, with the countdown, laps, ranking,
//! the time limit and the result. The rules are `libs/blackbox-race`; this module wires them to the player's car, the
//! computer-driven racers, the minimap, the card and the career. Specs: `docs/specs/race-mode.md`,
//! `docs/specs/ai-racers.md`, `docs/specs/ai-racers-route.md`, `docs/specs/ai-racers-catchup.md`.

mod barriers;
mod blips;
mod card;
mod challenge;
mod command;
mod demo;
mod drag;
mod emulate;
mod final_race;
mod final_race_events;
mod finish;
mod flares;
mod flow;
mod intro;
mod load;
pub(super) mod mods;
mod names;
mod outro;
mod report;
mod restart;
pub(in crate::scenes::world) mod scripted_chase;
mod setup;
mod start;
mod startable;

pub(super) use command::race_command;
pub use setup::RaceOptions;

use std::collections::VecDeque;

use blackbox_race::{CatchUp, Course, Race};
use nfsmw_career::story::{IntroStep, OutroStep};
use nfsmw_data::gameplay::RaceType;

use crate::scenes::world::road::Spawn;

/// Where the run is.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Stage {
    /// Waiting for the player's car to stand on the grid and the racers' models to load.
    Loading { waited: f32 },
    /// The race's intro, once the racers stand on the grid and before the countdown: the message, the tutorial, the movie
    /// and the NIS (`intro.rs`). `current` is what plays now.
    Intro { steps: VecDeque<IntroStep>, current: intro::Playing },
    /// The countdown and the race itself (the engine's phase tells which).
    Running,
    /// The race-over message is up for this many more seconds (`RaceOverMessageDuration`).
    Results { left: f32 },
    /// The post-race screen has the race: it waits for the front end to take the report and answer. `left` is how long
    /// the world waits for a front end before it ends the race itself (a run with no front end).
    PostRace { left: f32 },
    /// The race's outro, after the post-race answer: the outro NIS, message and movie (`outro.rs`). `won` is whether the
    /// race's chain runs once the outro is over.
    Outro { steps: VecDeque<OutroStep>, current: intro::Playing, won: bool },
}

impl Stage {
    /// The race is staged or on: loading, the intro, counting down or running. Once the result is fixed (`Results`,
    /// `PostRace`) the career has been told and the post-race screen owns the race (`docs/specs/race-restart.md` section 1).
    pub(super) fn is_live(&self) -> bool {
        matches!(self, Stage::Loading { .. } | Stage::Intro { .. } | Stage::Running)
    }
}

/// What a race was started from, kept so a restart runs exactly the same race again (a race a mod registered has no
/// name in the install's data, and its course is not made from it).
pub(super) struct Origin {
    pub race: nfsmw_data::gameplay::Race,
    /// The course a mod gave, when it gave one.
    pub course: Option<Course>,
}

/// One racer's plan: who it is and where it starts. Index 0 is the player.
#[derive(Debug, Clone)]
pub(super) struct RacerPlan {
    pub name: String,
    /// The `pvehicle` name of the car; `None` for the player.
    pub model: Option<String>,
    pub spawn: Spawn,
    pub skill: f32,
    pub bias: f32,
    /// Models that were tried and failed, so a fallback is not asked for twice.
    pub tried: Vec<String>,
}

/// How the player's race ended.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct RaceResult {
    /// 1 for first.
    pub place: u32,
    pub racers: usize,
    /// The finish time in seconds; `None` when the player did not finish.
    pub time: Option<f32>,
}

/// A line the card flashes for a moment (GO, a checkpoint, a knockout).
#[derive(Debug, Clone, Default)]
pub(super) struct Banner {
    pub text: String,
    pub left: f32,
    /// Counts the lines, so the HUD plays a repeated text again.
    pub serial: u32,
}

pub(super) struct RaceSession {
    /// The race's collection name, for example `race_bin_10/10_2_1_sprint`.
    pub name: String,
    /// The career event id, for example `10.2.1`.
    pub event: String,
    /// The minimap's map file while this race runs: the event's own tiles, or the region's variant for a pursuit race
    /// (`docs/specs/race-minimap.md`).
    pub map: crate::hud::MapSource,
    pub kind: RaceType,
    pub options: RaceOptions,
    pub stage: Stage,
    pub engine: Option<Race>,
    pub plan: Vec<RacerPlan>,
    /// The race's setup, kept until the grid is ready.
    pub setup: Option<start::Prepared>,
    /// The traffic count a console command had set before the race, to give back.
    pub previous_traffic: Option<usize>,
    pub glue: Vec<CatchUp>,
    /// The rubber band is on (`CatchUp`).
    pub catch_up: bool,
    pub glue_clock: f32,
    /// Seconds since the demo run's last jump.
    pub demo_clock: f32,
    /// The lane of the player's car in a drag race (`drag.rs`).
    pub lane: drag::Lane,
    /// Where each emulated racer is on its route (physics space); `None` for a racer that is simulated.
    pub emu: Vec<Option<glam::Vec3>>,
    /// Seconds since the last debug line about the racers.
    pub log_clock: f32,
    pub banner: Banner,
    /// The race clock at the start of the lap the player is on, for the best lap (`custom_race/records.rs`).
    pub lap_mark: f32,
    pub result: Option<RaceResult>,
    /// The post-race report, built when the race-over message ends and taken by the front end.
    pub report: Option<crate::viewer::RaceReport>,
    /// The race the career runs after this one (`PostRaceActivity`), by name.
    pub chain: Option<String>,
    /// The cash a career win pays.
    pub cash_value: f32,
    /// The stock cars for opponents whose model fails to load.
    pub pool: Vec<String>,
    /// The pursuit challenge being run, for a `challenge` race: it has no engine (`challenge.rs`).
    pub challenge: Option<challenge::Challenge>,
    /// The scenery groups the race's `Barriers` list switched on, put back when the race ends (`barriers.rs`).
    pub walls: barriers::RaceWalls,
    /// How many times the player ran into traffic since GO, for the post-race statistics.
    pub collisions: u32,
    /// The race and course this session was started from, for a restart.
    pub origin: Origin,
}

impl RaceSession {
    /// Whether the police are part of this race: the pursuit races and the pursuit challenges. Every other race is clear
    /// of cops, whatever its `CopsInRace` says. **[ours]**
    pub fn has_police(&self) -> bool {
        self.challenge.is_some() || self.origin.race.params.pursuit_race
    }
}

impl crate::scenes::world::WorldScene {
    /// Whether a race that has no police is on, so the cops are kept out of the world.
    pub(in crate::scenes::world) fn race_bars_cops(&self) -> bool {
        self.race.as_ref().is_some_and(|race| !race.has_police())
    }
}
