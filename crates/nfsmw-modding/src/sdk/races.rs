//! `Races`: the race list, starts, custom races and results (catalogue: docs/modding/api-nfsmw.md, "Races").
//!
//! The race runtime is not built yet, so every call returns `Error::Unsupported` for now. The signatures are the ones the
//! race module will keep.

use core::mem::size_of;

use blackbox_modding::sdk::ModResult;

use super::career_types::{name_abi, name_text};
use super::ffi::races::{self as host, RacesFns};
use crate::abi::{BbRaceDef, BbRaceInfo, BbRaceResult, BbRaceState, BbVec3};

/// One race of the list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Race {
    /// Race id.
    pub id: u32,
    /// Its name.
    pub name: String,
    /// The race kind number. [unconfirmed] list.
    pub kind: u32,
    /// Laps.
    pub laps: u32,
    /// Racers.
    pub racers: u32,
    /// Whether it is done.
    pub done: bool,
}

impl Race {
    fn from_abi(raw: &BbRaceInfo) -> ModResult<Self> {
        Ok(Race {
            id: raw.id,
            name: name_text(&raw.name)?,
            kind: raw.kind,
            laps: raw.laps,
            racers: raw.racers,
            done: raw.done != 0,
        })
    }
}

/// The race being driven.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RaceState {
    /// Race id.
    pub id: u32,
    /// Lap the player is on.
    pub lap: u32,
    /// Checkpoint the player is heading for.
    pub checkpoint: u32,
    /// Position in the field, 1 is first.
    pub position: u32,
    /// Seconds since the start.
    pub elapsed: f32,
}

/// The result of a finished race.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RaceResult {
    /// Final position, 1 is first.
    pub position: u32,
    /// Finish time, seconds.
    pub time: f32,
    /// Whether the player won.
    pub won: bool,
    /// Cash won.
    pub winnings: u32,
}

impl RaceResult {
    fn from_abi(raw: &BbRaceResult) -> Self {
        RaceResult { position: raw.position, time: raw.time, won: raw.won != 0, winnings: raw.winnings }
    }
}

/// The definition of a custom race. Its checkpoints go to [`Races::register`] beside it.
#[derive(Debug, Clone, PartialEq)]
pub struct RaceDef {
    /// The registry key, `modid:name`.
    pub key: String,
    /// The name shown to the player.
    pub name: String,
    /// The race kind number. [unconfirmed] list.
    pub kind: u32,
    /// Laps.
    pub laps: u32,
    /// Cash paid for a win.
    pub reward_cash: u32,
    /// Start position, render space, metres.
    pub start: [f32; 3],
    /// Start heading, radians.
    pub start_heading: f32,
    /// Opponents on the grid besides the player, 0 to 7.
    pub opponents: u32,
}

impl RaceDef {
    fn to_abi(&self) -> ModResult<BbRaceDef> {
        Ok(BbRaceDef {
            size: size_of::<BbRaceDef>() as u32,
            kind: self.kind,
            laps: self.laps,
            reward_cash: self.reward_cash,
            start: BbVec3::from_array(self.start),
            start_heading: self.start_heading,
            key: name_abi(&self.key)?,
            name: name_abi(&self.name)?,
            opponents: self.opponents,
        })
    }
}

/// The `nfsmw.races` interface of the host, as a mod uses it.
#[derive(Clone, Copy)]
pub struct Races {
    fns: RacesFns,
}

impl Races {
    pub(crate) fn new(fns: RacesFns) -> Self {
        Races { fns }
    }

    /// The race list.
    pub fn list(&self) -> ModResult<Vec<Race>> {
        let count = host::race_count(&self.fns)?;
        (0..count).map(|index| Race::from_abi(&host::race_at(&self.fns, index)?)).collect()
    }

    /// The checkpoints of race `id`, in order.
    pub fn checkpoints(&self, id: u32) -> ModResult<Vec<[f32; 3]>> {
        let count = host::race_checkpoint_count(&self.fns, id)?;
        (0..count).map(|index| host::race_checkpoint_at(&self.fns, id, index).map(BbVec3::to_array)).collect()
    }

    /// Start race `id`, as driving into its start would. (write)
    pub fn start(&self, id: u32) -> ModResult<()> {
        host::race_start(&self.fns, id)
    }

    /// Abort the running race. (write)
    pub fn abort(&self) -> ModResult<()> {
        host::race_abort(&self.fns)
    }

    /// The race being driven, or `None` when none runs.
    pub fn current(&self) -> ModResult<Option<RaceState>> {
        let raw: BbRaceState = host::race_current(&self.fns)?;
        if raw.active == 0 {
            return Ok(None);
        }
        Ok(Some(RaceState {
            id: raw.id,
            lap: raw.lap,
            checkpoint: raw.checkpoint,
            position: raw.position,
            elapsed: raw.elapsed,
        }))
    }

    /// Register a custom race with its checkpoints; returns its id. (write)
    pub fn register(&self, def: &RaceDef, checkpoints: &[[f32; 3]]) -> ModResult<u32> {
        let points = checkpoints.iter().copied().map(BbVec3::from_array).collect::<Vec<_>>();
        host::race_register(&self.fns, &def.to_abi()?, &points)
    }

    /// Set the opponents' skill of race `id`, 0 to 1. (write)
    pub fn set_opponent_skill(&self, id: u32, skill: f32) -> ModResult<()> {
        host::race_set_opponent_skill(&self.fns, id, skill)
    }

    /// Set the cash for a win of race `id`. (write)
    pub fn set_reward(&self, id: u32, cash: u32) -> ModResult<()> {
        host::race_set_reward(&self.fns, id, cash)
    }

    /// The result of race `id`; `Error::NotFound` when it has none.
    pub fn results(&self, id: u32) -> ModResult<RaceResult> {
        host::race_results(&self.fns, id).map(|raw| RaceResult::from_abi(&raw))
    }
}
