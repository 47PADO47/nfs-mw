//! Deterministic HUD scenarios (`--hud-scenario <name>`): development fixtures that put the HUD in a known state (free
//! roam, a race, a chase, a damaged car, a notification preemption, the map modes) for checking it by eye and for the
//! screenshot matrix (`tools/hud_shots.ps1`, `tools/hud_shots.sh`). Usage: `docs/specs/custom-hud-scenarios.md`.
//!
//! These are dev fixtures, not game data: synthetic numbers chosen to exercise the layout, with no game data copied
//! into them. A frame drawn from one carries a `SCENARIO: <name>` label in the top-left corner, so a screenshot of
//! fixture data cannot be taken for gameplay.
//!
//! **They are off unless the hidden `--hud-scenario` flag is given.** Without it no [`ActiveScenario`] resource exists:
//! the system that writes the fixture over the scene's state does not run, the label is not drawn and the settings are
//! left as they are. The production path only carries the injection points (the run option, the system's registration
//! in the HUD plugin and the label's call in the UI pass), all keyed on that resource or that option.
//!
//! The same scenario gives the same state on the same frame: the values are fixed, the camera the world markers project
//! through is the fixture's own, and the events are stepped by the frame count since the HUD came up.
//!
//! - `common`: the fixture car, its camera and the state every scenario starts from.
//! - `driving`: free roam, the damaged car and the notification preemption.
//! - `race`: the race intro, the race under way and the results.
//! - `pursuit`: the chase, the search, the cool-down and the escape.
//! - `map`: the GPS route under the centred and the expanded map.
//! - `inject`: the resource, the settings a run starts with, the system that writes the state, and the label.

mod common;
mod driving;
mod inject;
mod map;
mod pursuit;
mod race;
#[cfg(test)]
mod tests;

use std::fmt;
use std::str::FromStr;

#[cfg(test)]
pub use common::VIEWPORT;
#[cfg(test)]
pub use driving::PREEMPT_FRAME;
pub use inject::{ActiveScenario, apply, label};

use crate::hud::HudState;
use crate::settings::hud::MapMode;

/// One deterministic HUD state, named on the command line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudScenario {
    /// Cruising in free roam, no race and no chase, with a place to discover on the map.
    FreeRoam,
    /// A circuit's grid: the countdown at three.
    RaceIntro,
    /// A circuit under way: second of six on lap two, the leader a little ahead.
    Racing,
    /// A finished race handing off to the results screens.
    RaceResults,
    /// A chase at heat three: two cops and the helicopter in sight, a roadblock reported.
    PursuitActive,
    /// Contact lost a few seconds ago: the police search from where they last saw the player.
    Search,
    /// The cool-down meter up, the player in a hiding spot.
    Cooldown,
    /// The player got away.
    Escape,
    /// A blown tyre, a punctured one and heavy body damage.
    CriticalDamage,
    /// A Low notice and two Medium ones up, then a Critical event that displaces the Low one.
    NotifyPreempt,
    /// Free roam with a GPS route two turns long, the map centred.
    MapCentered,
    /// The same with the map expanded.
    MapExpanded,
}

impl HudScenario {
    /// Every scenario, in the order the screenshot matrix renders them.
    pub const ALL: [Self; 12] = [
        Self::FreeRoam,
        Self::RaceIntro,
        Self::Racing,
        Self::RaceResults,
        Self::PursuitActive,
        Self::Search,
        Self::Cooldown,
        Self::Escape,
        Self::CriticalDamage,
        Self::NotifyPreempt,
        Self::MapCentered,
        Self::MapExpanded,
    ];

    /// The name the command line takes.
    pub fn name(self) -> &'static str {
        match self {
            Self::FreeRoam => "free-roam",
            Self::RaceIntro => "race-intro",
            Self::Racing => "racing",
            Self::RaceResults => "race-results",
            Self::PursuitActive => "pursuit-active",
            Self::Search => "search",
            Self::Cooldown => "cooldown",
            Self::Escape => "escape",
            Self::CriticalDamage => "critical-damage",
            Self::NotifyPreempt => "notify-preempt",
            Self::MapCentered => "map-centered",
            Self::MapExpanded => "map-expanded",
        }
    }

    /// The map mode the scenario runs with; `None` keeps the default.
    pub fn map_mode(self) -> Option<MapMode> {
        match self {
            Self::MapCentered => Some(MapMode::Centered),
            Self::MapExpanded => Some(MapMode::Expanded),
            _ => None,
        }
    }

    /// The HUD state of `frame` (counted from 0, the first frame the HUD is up), for a window of `viewport` logical
    /// pixels. The events of a frame are new on that frame only: `events_frame` moves on every frame.
    pub fn state(self, frame: u64, viewport: [f32; 2]) -> HudState {
        let mut state = self.fixture(viewport);
        state.events = self.events(frame, std::mem::take(&mut state.events));
        state.events_frame = frame + 1;
        state
    }

    /// The fixture as its first frame shows it, with the events that open it.
    fn fixture(self, viewport: [f32; 2]) -> HudState {
        match self {
            Self::FreeRoam => driving::free_roam(viewport),
            Self::RaceIntro => race::intro(viewport),
            Self::Racing => race::racing(viewport),
            Self::RaceResults => race::results(viewport),
            Self::PursuitActive => pursuit::active(viewport),
            Self::Search => pursuit::search(viewport),
            Self::Cooldown => pursuit::cooldown(viewport),
            Self::Escape => pursuit::escape(viewport),
            Self::CriticalDamage => driving::critical_damage(viewport),
            Self::NotifyPreempt => driving::free_roam(viewport),
            Self::MapCentered | Self::MapExpanded => map::route(viewport),
        }
    }

    /// The events of `frame`: the opening ones on the first frame, none after, except for the scenarios that step.
    fn events(self, frame: u64, opening: Vec<crate::hud::HudEvent>) -> Vec<crate::hud::HudEvent> {
        if self == Self::NotifyPreempt {
            return driving::preempt_events(frame);
        }
        match frame {
            0 => opening,
            _ => Vec::new(),
        }
    }
}

impl fmt::Display for HudScenario {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for HudScenario {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let wanted = s.trim().to_ascii_lowercase();
        if let Some(scenario) = Self::ALL.into_iter().find(|scenario| scenario.name() == wanted) {
            return Ok(scenario);
        }
        let names: Vec<&str> = Self::ALL.iter().map(|scenario| scenario.name()).collect();
        Err(format!("unknown HUD scenario {s:?} (expected one of: {})", names.join(", ")))
    }
}
