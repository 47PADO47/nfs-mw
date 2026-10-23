//! The custom rules of a session (MP-11) as validated parameters, and everything a mode needs to start.

use nfsmw_sim::{PlayerId, Tick};

use crate::message::{MAX_COP_LEVEL, MAX_LAPS, MAX_TRAFFIC_DENSITY, Mode, Settings};

use super::support::{Unsupported, support};

/// Minutes in a day: a time of day is below this.
pub const MINUTES_PER_DAY: u16 = 24 * 60;
/// The default length of a tag round: five minutes of ticks. **[ours]**
pub const DEFAULT_TAG_ROUND_TICKS: u32 = 5 * 60 * nfsmw_sim::TICK_HZ;

/// The rules a host sets for a session (MP-11). Build with [`CustomRules::from_settings`], check with
/// [`CustomRules::validate`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CustomRules {
    /// Laps of a circuit or knockout; 0 in a mode without laps.
    pub laps: u8,
    /// Percent, at most [`MAX_TRAFFIC_DENSITY`].
    pub traffic_density: u8,
    /// The highest heat the police may reach, at most [`MAX_COP_LEVEL`].
    pub cop_level_cap: u8,
    /// Minutes after midnight, under [`MINUTES_PER_DAY`].
    pub time_of_day: u16,
    pub nitrous: bool,
    /// Whether the police chase players at all.
    pub pursuits: bool,
}

/// Why a set of rules or a setup was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
pub enum RuleError {
    #[error(transparent)]
    Unsupported(#[from] Unsupported),
    #[error("a circuit or knockout needs at least one lap")]
    NoLaps,
    #[error("too many laps")]
    TooManyLaps,
    #[error("traffic density above 100 percent")]
    TrafficDensity,
    #[error("cop level cap above the highest heat")]
    CopLevel,
    #[error("time of day past midnight")]
    TimeOfDay,
    #[error("pursuits need a cop level cap of at least 1")]
    PursuitsWithoutPolice,
    #[error("pursuit co-op needs pursuits on")]
    CoopNeedsPursuits,
    #[error("the mode needs at least {0} players")]
    TooFewPlayers(u8),
    #[error("a player appears twice")]
    DuplicatePlayer,
}

impl CustomRules {
    /// The rules of a session's settings at `time_of_day` (the settings struct has no clock; `LoadWorld` carries it).
    pub fn from_settings(settings: &Settings, time_of_day: u16) -> Self {
        Self {
            laps: settings.laps,
            traffic_density: settings.traffic_density,
            cop_level_cap: settings.cop_level_cap,
            time_of_day,
            nitrous: settings.nitrous,
            pursuits: settings.pursuits,
        }
    }

    /// The rules checked for `mode`: every range, a lap count where the mode has laps, police where pursuits are on.
    /// The laps of a mode without laps are set to 0 rather than refused, so a host's earlier circuit setting does not
    /// block a change of mode. **[ours]**
    pub fn validate(self, mode: Mode) -> Result<Self, RuleError> {
        support(mode)?;
        if self.laps > MAX_LAPS {
            return Err(RuleError::TooManyLaps);
        }
        if self.traffic_density > MAX_TRAFFIC_DENSITY {
            return Err(RuleError::TrafficDensity);
        }
        if self.cop_level_cap > MAX_COP_LEVEL {
            return Err(RuleError::CopLevel);
        }
        if self.time_of_day >= MINUTES_PER_DAY {
            return Err(RuleError::TimeOfDay);
        }
        if self.pursuits && self.cop_level_cap == 0 {
            return Err(RuleError::PursuitsWithoutPolice);
        }
        if mode == Mode::PursuitCoop && !self.pursuits {
            return Err(RuleError::CoopNeedsPursuits);
        }
        let has_laps = matches!(mode, Mode::Circuit | Mode::Knockout);
        if has_laps && self.laps == 0 {
            return Err(RuleError::NoLaps);
        }
        let laps = match has_laps {
            true => self.laps,
            false => 0,
        };
        Ok(Self { laps, ..self })
    }
}

/// Everything a mode needs to start.
#[derive(Debug, Clone, PartialEq)]
pub struct ModeSetup {
    pub mode: Mode,
    pub rules: CustomRules,
    /// The players at the start; order does not matter.
    pub players: Vec<PlayerId>,
    /// The tick the race or round starts (the `StartAt` tick). Inputs before it are ignored.
    pub start_tick: Tick,
    /// Each player's heat carried in from a career record; a missing player carries none. **[ours]**: LAN sessions
    /// without a career pass none.
    pub carried_heat: Vec<(PlayerId, u8)>,
    /// The threshold speed of each speed trap, km/h, by trap index (speedtrap only).
    pub trap_thresholds_kmh: Vec<f32>,
    /// The length of a tag round in ticks.
    pub tag_round_ticks: u32,
    /// The first "it" in tag; `None` picks the lowest player id. **[ours]**
    pub tag_first: Option<PlayerId>,
}

impl ModeSetup {
    /// A setup for `mode` with `rules`, `players` and `start_tick`, no carried heat, no speed traps, the default tag
    /// round.
    pub fn new(mode: Mode, rules: CustomRules, players: Vec<PlayerId>, start_tick: Tick) -> Self {
        Self {
            mode,
            rules,
            players,
            start_tick,
            carried_heat: Vec::new(),
            trap_thresholds_kmh: Vec::new(),
            tag_round_ticks: DEFAULT_TAG_ROUND_TICKS,
            tag_first: None,
        }
    }

    /// The setup checked: the rules for the mode, unique players, and two players for knockout and tag. Returns the
    /// players sorted by id and the validated rules.
    pub fn validate(&self) -> Result<(Vec<PlayerId>, CustomRules), RuleError> {
        let rules = self.rules.validate(self.mode)?;
        let mut players = self.players.clone();
        players.sort();
        let count = players.len();
        players.dedup();
        if players.len() != count {
            return Err(RuleError::DuplicatePlayer);
        }
        let need = match self.mode {
            Mode::Knockout | Mode::Tag => 2,
            _ => 1,
        };
        if players.len() < usize::from(need) {
            return Err(RuleError::TooFewPlayers(need));
        }
        Ok((players, rules))
    }

    /// The heat `player` carries, 0 when none.
    pub fn carried(&self, player: PlayerId) -> u8 {
        self.carried_heat.iter().filter(|(id, _)| *id == player).map(|(_, level)| *level).max().unwrap_or(0)
    }
}
