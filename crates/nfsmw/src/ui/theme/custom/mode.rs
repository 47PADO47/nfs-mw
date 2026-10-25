//! The Custom HUD's contextual mode (`docs/specs/custom-hud/modes.md` section 1): a pure resolver from the frame's
//! [`HudState`] to a [`HudMode`], the [`ModeClock`] that remembers when the mode began so the layouts crossfade on
//! elapsed time, and the [`Detail`] level the preset asks for.
//!
//! Resolution order, the first rule that holds wins:
//!
//! 1. the HUD is not visible (a cutscene, the garage, the fly view, a split-screen race): `Hidden`;
//! 2. a race is set and its results flow has it: `RaceResults`;
//! 3. a race is set and its intro or countdown is up: `RaceIntro`;
//! 4. a race is set and running: `Racing` (a chase in a race keeps the race layout, which adds a compact pursuit strip);
//! 5. the pursuit phase is not `None`: `Pursuit(phase)`;
//! 6. otherwise `FreeRoam`.
//!
//! The menus and the world map do not show the HUD at all (the plugin draws nothing then), so they need no rule here.

use crate::hud::{HudState, PursuitPhase, RaceHud, RaceStage};
use crate::settings::hud::{HudPreset, HudSettings};

use super::tokens::ramp;

/// What the player is doing, as the HUD lays itself out for it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HudMode {
    /// Nothing of the contextual layouts shows.
    #[default]
    Hidden,
    FreeRoam,
    /// The race's intro and its countdown.
    RaceIntro,
    Racing,
    /// The results flow has the race: the strip and the leaderboard leave.
    RaceResults,
    /// A chase outside a race, in its phase (never `PursuitPhase::None`).
    Pursuit(PursuitPhase),
}

impl HudMode {
    /// The pursuit phase of a pursuit mode.
    pub fn pursuit_phase(self) -> Option<PursuitPhase> {
        match self {
            HudMode::Pursuit(phase) => Some(phase),
            _ => None,
        }
    }
}

/// The mode of the frame `hud`.
pub fn resolve(hud: &HudState) -> HudMode {
    if !hud.visible {
        return HudMode::Hidden;
    }
    // The world says the race's stage; a race readout without one (a scene that fills only the package's fields) runs.
    let stage = hud.context.race_stage.or(hud.race.map(|_| RaceStage::Running));
    if let Some(stage) = stage {
        return race_mode(stage, hud.race.as_ref());
    }
    match hud.pursuit_phase {
        PursuitPhase::None => HudMode::FreeRoam,
        phase => HudMode::Pursuit(phase),
    }
}

/// The race mode of a race at `stage`: its countdown still counts as the intro.
fn race_mode(stage: RaceStage, race: Option<&RaceHud>) -> HudMode {
    match stage {
        RaceStage::Intro => HudMode::RaceIntro,
        RaceStage::Results => HudMode::RaceResults,
        RaceStage::Running if race.is_some_and(|race| race.count.is_some()) => HudMode::RaceIntro,
        RaceStage::Running => HudMode::Racing,
    }
}

/// When the mode began, and the one before it, so an element can fade in with its mode and out with the last one.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ModeClock {
    mode: HudMode,
    previous: HudMode,
    /// Elapsed seconds (`CustomFeed::elapsed_s`) when `mode` began.
    since: f32,
}

impl ModeClock {
    /// Takes the frame's mode at `now`. Returns whether it changed.
    pub fn update(&mut self, mode: HudMode, now: f32) -> bool {
        if mode == self.mode {
            return false;
        }
        self.previous = self.mode;
        self.mode = mode;
        self.since = now;
        true
    }

    pub fn mode(&self) -> HudMode {
        self.mode
    }

    /// Seconds since the mode began.
    pub fn age(&self, now: f32) -> f32 {
        (now - self.since).max(0.0)
    }

    /// The opacity at `now` of an element that shows in the modes `shows` accepts, crossfading over `duration`
    /// (`Motion::mode_change`): it fades in with a mode it shows in, out with one it showed in, and does not fade
    /// between two modes it shows in both.
    pub fn opacity(&self, shows: impl Fn(HudMode) -> bool, now: f32, duration: f32) -> f32 {
        let t = ramp(self.age(now), duration);
        match (shows(self.mode), shows(self.previous)) {
            (true, true) => 1.0,
            (true, false) => t,
            (false, true) => 1.0 - t,
            (false, false) => 0.0,
        }
    }
}

/// How much each layout shows (`docs/specs/custom-hud/modes.md` section 3), from the preset.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Detail {
    Minimal,
    Balanced,
    Advanced,
}

impl Detail {
    /// The level the `[custom_hud]` preset chose.
    pub fn of(settings: &HudSettings) -> Self {
        match settings.preset {
            HudPreset::Minimal => Detail::Minimal,
            HudPreset::Balanced => Detail::Balanced,
            HudPreset::Advanced => Detail::Advanced,
        }
    }
}

#[cfg(test)]
#[path = "mode_tests.rs"]
mod tests;
