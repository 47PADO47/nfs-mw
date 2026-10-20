//! Quick Play, the brief (`Quick_Race_Brief.fng`, `UIQRBrief`): a random car and a random race from the pool the career's stage
//! offers, with Randomize to pick again and Accept to drive them. Spec: `docs/specs/quick-play.md`.
//!
//! The screen shows the pick in the package's objects (`fill.rs`) and names the car as the menu backdrop's look, so the 3D
//! backdrop shows it ([`ScreenLogic::look`]). Accept plays the package's leave script and then starts the race with the car
//! (`Command::StartQuickPlay`); Back returns to the Quick Race menu.

pub(crate) mod fill;
pub mod pool;
pub(crate) mod text;

use blackbox_feng::ids::{PAD_ACCEPT, PAD_BACK};

use self::pool::{CarPool, PROLOGUE_STAGE, Pick, Rng};
use super::ids::{self, screen};
use super::logic::{Args, Command, Cx, ScreenLogic, Sub};
use super::look::{CarLook, Look};
use super::quick_race::{self, Board};
use crate::career::CareerHandle;

pub use self::pool::load_in_background;

/// The game message the Randomize button sends (key `1`); the same message as the Game Stats button and the rap sheet's
/// Switch View.
pub const RANDOMIZE: u32 = 0xC519_BFC4;

/// The Quick Play brief.
pub struct QuickPlay {
    rng: Rng,
    /// The pick being shown, once the car pool and the race board were read.
    pick: Option<Pick>,
    /// Accept was pressed: the leave script runs and `EXIT_COMPLETE` starts the race.
    leaving: bool,
}

impl QuickPlay {
    pub fn new() -> Self {
        Self::with_rng(Rng::from_clock())
    }

    /// A screen whose picks follow `rng`, for tests.
    pub fn with_rng(rng: Rng) -> Self {
        Self { rng, pick: None, leaving: false }
    }

    /// The pool the pick is taken from, `None` until both the cars and the races were read.
    fn sources() -> Option<(std::sync::Arc<CarPool>, std::sync::Arc<Board>)> {
        Some((pool::cars()?, quick_race::board()?))
    }

    /// The Blacklist stage: the loaded profile's, the prologue's when none is loaded.
    fn stage() -> u8 {
        CareerHandle::shared().with_loaded(|career| career.stage()).unwrap_or(PROLOGUE_STAGE)
    }

    /// Picks a car and a race (another pair than the one shown) and shows them.
    fn randomize(&mut self, cx: &mut Cx) {
        let Some((cars, board)) = Self::sources() else { return };
        let stage = Self::stage();
        self.pick = pool::pick(&cars.cars, &board.tracks, stage, &mut self.rng, self.pick);
        self.show(cx);
    }

    /// Writes the pick into the package; nothing while there is none.
    fn show(&self, cx: &mut Cx) {
        let Some(pick) = self.pick else { return };
        let Some((cars, board)) = Self::sources() else { return };
        let (Some(car), Some(track)) = (cars.cars.get(pick.car), board.tracks.get(pick.race)) else { return };
        fill::hide_unused(cx);
        fill::show(cx, car, track, &cars, Self::stage());
    }

    /// The car and race names Accept starts the race with.
    fn start_command(&self) -> Option<Command> {
        let pick = self.pick?;
        let (cars, board) = Self::sources()?;
        let car = cars.cars.get(pick.car)?;
        let track = board.tracks.get(pick.race)?;
        Some(Command::StartQuickPlay { race: track.name.clone(), car: car.folder.clone() })
    }

    /// Plays the event handler's forward script, which sends `EXIT_COMPLETE` when the leave animation is done.
    fn leave(&mut self, cx: &mut Cx) {
        if self.pick.is_none() {
            return;
        }
        self.leaving = true;
        cx.script(ids::EVENT_HANDLER, ids::SCRIPT_FORWARD);
    }

    fn finish(&self, cx: &mut Cx) {
        if !self.leaving {
            return;
        }
        let Some(command) = self.start_command() else { return };
        cx.send(command);
    }
}

impl Default for QuickPlay {
    fn default() -> Self {
        Self::new()
    }
}

impl ScreenLogic for QuickPlay {
    fn start(&mut self, cx: &mut Cx) {
        self.randomize(cx);
    }

    fn resume(&mut self, cx: &mut Cx) {
        self.show(cx);
    }

    /// The cars and races are read in the background: until they are there, every frame tries again.
    fn tick(&mut self, cx: &mut Cx, _dt: f32) {
        if self.pick.is_some() {
            return;
        }
        self.randomize(cx);
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        match message {
            RANDOMIZE if !self.leaving => self.randomize(cx),
            PAD_ACCEPT if !self.leaving => self.leave(cx),
            PAD_BACK if !self.leaving => {
                let args = Args { sub: Sub::QuickRace, ..Args::default() };
                cx.send(Command::Switch(screen::MAIN_MENU_SUB.into(), args));
            }
            ids::EXIT_COMPLETE => self.finish(cx),
            _ => {}
        }
    }

    fn look(&self) -> Option<Look> {
        let pick = self.pick?;
        let (cars, _) = Self::sources()?;
        let car = cars.cars.get(pick.car)?;
        Some(Look::Car(CarLook { car: car.folder.clone(), preset: None }))
    }
}

#[cfg(test)]
#[path = "quick_play_tests.rs"]
mod tests;
