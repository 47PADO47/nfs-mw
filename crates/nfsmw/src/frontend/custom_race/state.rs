//! What the custom race screens remember between them: the mode, the region filter, the picked track and the options of each
//! mode, and the picked car. The original keeps the same in `cFrontendDatabase` (`RaceMode`, `TheQuickRaceSettings`); the
//! screens read and write it through [`with`] because they are made by name and get no constructor data.

use std::sync::Mutex;

use super::mode::Mode;
use super::region::Filter;
use super::settings::Settings;
use super::split::Split;

/// The picked car of the car select: the list it is in and its name in the catalogue.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CarPick {
    /// Which list the car select showed (`CarList::index`).
    pub list: usize,
    /// The catalogue name of the car (`cobaltss`).
    pub name: Option<String>,
}

#[derive(Debug)]
pub struct State {
    pub mode: Mode,
    pub filter: Filter,
    /// The picked track of each mode, by the race's collection name.
    events: [Option<String>; 6],
    settings: [Settings; 6],
    pub car: CarPick,
    /// The two-player flow's part (`split.rs`): the flag, the devices, the gearboxes and the second car.
    pub split: Split,
}

impl State {
    const fn new() -> Self {
        Self {
            mode: Mode::Circuit,
            filter: Filter::ALL,
            events: [None, None, None, None, None, None],
            settings: [Settings { laps: 1, opponents: 3, difficulty: 1, traffic: 1, catch_up: true, heat: 1 }; 6],
            car: CarPick { list: 0, name: None },
            split: Split::new(),
        }
    }

    /// The options of the current mode.
    pub fn settings(&self) -> &Settings {
        &self.settings[self.mode.index()]
    }

    pub fn settings_mut(&mut self) -> &mut Settings {
        &mut self.settings[self.mode.index()]
    }

    /// Puts the current mode's options back to the defaults (the options page's Defaults button).
    pub fn reset_settings(&mut self) {
        let mode = self.mode;
        self.settings[mode.index()] = Settings::defaults_for(mode);
    }

    /// The car pick of a player: the custom race's one, or in split screen the one of player `player` (0 or 1).
    pub fn pick(&self, player: usize) -> &CarPick {
        match self.split.on {
            true => &self.split.cars[player.min(1)],
            false => &self.car,
        }
    }

    pub fn pick_mut(&mut self, player: usize) -> &mut CarPick {
        match self.split.on {
            true => &mut self.split.cars[player.min(1)],
            false => &mut self.car,
        }
    }

    /// The picked track of the current mode.
    pub fn event(&self) -> Option<&str> {
        self.events[self.mode.index()].as_deref()
    }

    pub fn set_event(&mut self, name: Option<String>) {
        self.events[self.mode.index()] = name;
    }

    /// Opens a mode: its options start from the defaults the first time (the knockout's laps follow its opponents).
    pub fn choose_mode(&mut self, mode: Mode) {
        self.mode = mode;
        if mode == Mode::Knockout && self.settings[mode.index()].laps != self.settings[mode.index()].opponents {
            self.settings[mode.index()] = Settings::defaults_for(mode);
        }
    }
}

static STATE: Mutex<State> = Mutex::new(State::new());

/// Reads or changes the state.
pub fn with<R>(act: impl FnOnce(&mut State) -> R) -> R {
    let mut state = STATE.lock().unwrap_or_else(|e| e.into_inner());
    act(&mut state)
}

/// The current mode.
pub fn mode() -> Mode {
    with(|state| state.mode)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::custom_race::settings::Row;

    #[test]
    fn each_mode_keeps_its_own_options_and_pick() {
        let mut state = State::new();
        state.choose_mode(Mode::Circuit);
        state.settings_mut().step(Mode::Circuit, Row::Laps, true);
        state.set_event(Some("a".into()));
        state.choose_mode(Mode::Drag);
        assert_eq!(state.settings().laps, 1);
        assert_eq!(state.event(), None);
        state.choose_mode(Mode::Circuit);
        assert_eq!(state.settings().laps, 2);
        assert_eq!(state.event(), Some("a"));
    }

    #[test]
    fn defaults_restore_the_current_modes_options() {
        let mut state = State::new();
        state.choose_mode(Mode::Sprint);
        state.settings_mut().step(Mode::Sprint, Row::Opponents, true);
        state.reset_settings();
        assert_eq!(*state.settings(), Settings::default());
    }
}
