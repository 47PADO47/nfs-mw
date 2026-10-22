//! The game modes as server rules (features.md section 3.1, ADR 0011): one state machine per mode, fed by the world's
//! [`nfsmw_sim::SimEvent`]s, one [`CarSample`] per player car per tick and the session's own facts ([`ModeInput`]),
//! returning the [`ModeEvent`]s clients hear and the actions the server takes. Pure: no I/O, no clock; time is the
//! 60 Hz tick.
//!
//! **Per tick** the server calls [`ModeRules::feed`] for every input of the tick, then [`ModeRules::end_tick`] once.
//! [`ModeRules::begin`] runs once at the start tick. When [`ModeRules::is_over`] turns true (a [`ModeEvent::Over`]
//! said so), [`ModeRules::placings`] is the result for [`crate::results::build_record`]. Inputs before the setup's
//! start tick are ignored.
//!
//! | Mode | Rules | Over when |
//! |---|---|---|
//! | MP-01 free roam | [`FreeRoam`]: pursuits on or off; one shared pursuit | never (the lobby ends it) |
//! | MP-02 circuit, MP-03 sprint | [`Race`]: best lap, finish time plus reset penalties | nobody is racing |
//! | MP-05 speedtrap | [`Speedtrap`]: trap points rank first | nobody is racing |
//! | MP-06 knockout | [`Knockout`]: the last racer of each lap becomes a spectator | one racer left, or nobody racing |
//! | MP-07 pursuit co-op | [`PursuitCoop`] over [`PursuitTracker`] | the pursuit ends, or everyone left |
//! | MP-09 tag | [`Tag`]: distance and relative-speed contact, time as "it" | time up, or one player left |
//! | MP-11 custom rules | [`CustomRules`]: laps, traffic, cop cap, time of day, nitrous, pursuits | |
//!
//! MP-04 drag and MP-08 cops vs racers are refused ([`Unsupported`], ADR 0011 section 5), as are mod modes.
//! [`messages`] turns a mode event into the gameplay messages of protocol-messages.md section 7.6, where one exists.
//!
//! **Common rules [ours]:** in a race a busted racer is out (busted and DNF in the result) and spectates; a player
//! who leaves is out with the left flag but keeps a finish already made; of two equal finish times the lower player id
//! is ahead.

mod coop;
mod core;
mod event;
mod free_roam;
mod input;
mod knockout;
mod pursuit;
mod race;
mod setup;
mod speedtrap;
mod support;
mod tag;
mod wire;

#[cfg(test)]
mod tests;

pub use coop::PursuitCoop;
pub use event::{ModeEvent, OverReason};
pub use free_roam::FreeRoam;
pub use input::{CarSample, ModeInput};
pub use knockout::Knockout;
pub use pursuit::PursuitTracker;
pub use race::{Race, RaceKind};
pub use setup::{CustomRules, DEFAULT_TAG_ROUND_TICKS, MINUTES_PER_DAY, ModeSetup, RuleError};
pub use speedtrap::Speedtrap;
pub use support::{Unsupported, support};
pub use tag::{TAG_BACK_TICKS, TAG_DISTANCE, TAG_MIN_RELATIVE_SPEED, Tag, is_tag};
pub use wire::messages;

use nfsmw_sim::Tick;

use crate::message::Mode;
use crate::results::Placing;

/// The rules of one game mode.
pub trait ModeRules {
    /// Runs once at the start tick (tag announces its first holder).
    fn begin(&mut self, _out: &mut Vec<ModeEvent>) {}

    /// Reads one input of `tick`.
    fn feed(&mut self, tick: Tick, input: &ModeInput, out: &mut Vec<ModeEvent>);

    /// Ends `tick`, after every input of it (timers).
    fn end_tick(&mut self, _tick: Tick, _out: &mut Vec<ModeEvent>) {}

    /// Whether the round is over.
    fn is_over(&self) -> bool;

    /// Every player's line, in place order (unranked modes: player-id order).
    fn placings(&self) -> Vec<Placing>;

    /// Whether the world runs police and pursuits in this session.
    fn police(&self) -> bool;
}

/// Any runnable mode.
#[derive(Debug, Clone, PartialEq)]
pub enum AnyMode {
    FreeRoam(FreeRoam),
    Race(Race),
    Speedtrap(Speedtrap),
    Knockout(Knockout),
    PursuitCoop(PursuitCoop),
    Tag(Tag),
}

impl AnyMode {
    /// The rules of `setup`, after [`ModeSetup::validate`].
    pub fn build(setup: &ModeSetup) -> Result<Self, RuleError> {
        let (players, rules) = setup.validate()?;
        let start = setup.start_tick;
        let police = rules.pursuits;
        let tracker = || PursuitTracker::new(rules.cop_level_cap, &setup.carried_heat);
        let mode = match setup.mode {
            Mode::FreeRoam => Self::FreeRoam(FreeRoam::new(&players, start, police.then(tracker))),
            Mode::Circuit => Self::Race(Race::new(RaceKind::Circuit { laps: rules.laps }, &players, start, police)),
            Mode::Sprint => Self::Race(Race::new(RaceKind::Sprint, &players, start, police)),
            Mode::Speedtrap => Self::Speedtrap(Speedtrap::new(&setup.trap_thresholds_kmh, &players, start, police)),
            Mode::Knockout => Self::Knockout(Knockout::new(rules.laps, &players, start, police)),
            Mode::PursuitCoop => Self::PursuitCoop(PursuitCoop::new(&players, start, tracker())),
            Mode::Tag => Self::Tag(Tag::new(&players, start, setup.tag_round_ticks, setup.tag_first, police)),
            Mode::Drag => return Err(Unsupported::Drag.into()),
            Mode::CopsVsRacers => return Err(Unsupported::CopsVsRacers.into()),
            Mode::Custom(id) => return Err(Unsupported::ModMode(id).into()),
        };
        Ok(mode)
    }

    fn rules(&mut self) -> &mut dyn ModeRules {
        match self {
            Self::FreeRoam(mode) => mode,
            Self::Race(mode) => mode,
            Self::Speedtrap(mode) => mode,
            Self::Knockout(mode) => mode,
            Self::PursuitCoop(mode) => mode,
            Self::Tag(mode) => mode,
        }
    }

    fn rules_ref(&self) -> &dyn ModeRules {
        match self {
            Self::FreeRoam(mode) => mode,
            Self::Race(mode) => mode,
            Self::Speedtrap(mode) => mode,
            Self::Knockout(mode) => mode,
            Self::PursuitCoop(mode) => mode,
            Self::Tag(mode) => mode,
        }
    }
}

impl ModeRules for AnyMode {
    fn begin(&mut self, out: &mut Vec<ModeEvent>) {
        self.rules().begin(out);
    }

    fn feed(&mut self, tick: Tick, input: &ModeInput, out: &mut Vec<ModeEvent>) {
        self.rules().feed(tick, input, out);
    }

    fn end_tick(&mut self, tick: Tick, out: &mut Vec<ModeEvent>) {
        self.rules().end_tick(tick, out);
    }

    fn is_over(&self) -> bool {
        self.rules_ref().is_over()
    }

    fn placings(&self) -> Vec<Placing> {
        self.rules_ref().placings()
    }

    fn police(&self) -> bool {
        self.rules_ref().police()
    }
}
