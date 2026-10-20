//! After the bust's ticket in a career (`docs/specs/busted-aftermath.md`): the impound notice, the career car select and
//! the actions it asks for. The payment itself is settled by the world (`scenes/world/career/bust.rs`); the front end
//! reads what it left on the car's record.

use nfsmw_career::economy::aftermath::{Payment, after_payment_of};

use super::*;
use crate::frontend::career_screens::car_select::{self, CarSelectAction, CareerCarSelectLogic};
use crate::frontend::ids::screen::{CAR_SELECT, DIALOG};
use nfsmw_career::economy::aftermath::CarSelectEntry;

impl Frontend {
    /// The bust's payment was taken in a career: the notice first when the car was impounded (closing it opens the car
    /// select with the impounded animation), else the car select at once. The world stays held; leaving the car select
    /// goes to the safe house menu, whose Resume Free Roam drives again.
    pub(super) fn bust_paid(&mut self, marker: bool, host: &mut Host, env: &mut Env) {
        let payment = if marker { Payment::Marker } else { Payment::Cash };
        let after = self
            .career
            .with(|career| after_payment_of(payment, career.stable.career_for_handle(career.settings.car_handle)))
            .unwrap_or_else(|| after_payment_of(payment, None));
        let Some(notice) = after.notice else {
            return self.open_career_car_select(after.entry, host, env);
        };
        let text = Text::Game(blackbox_text::label_hash(notice.label()));
        let then = Command::CareerCar(CarSelectAction::Open(after.entry));
        let alert = Alert { title: Text::Custom(String::new()), message: text, on_close: Some(Box::new(then)) };
        let commands = self.screens.open_with(DIALOG, Box::new(AlertLogic::new(alert)), false, env);
        self.apply(commands, host, env);
    }

    /// Opens the career car select over the held game, replacing the screen on top (the ticket, or the safe house menu).
    pub(super) fn open_career_car_select(&mut self, entry: CarSelectEntry, host: &mut Host, env: &mut Env) {
        self.latch = true;
        self.paused.set(true);
        self.stage = Stage::Paused;
        // Leaving for the free roam it leads to is a roaming start: the autosave asks for it (`leave_safe_house`).
        self.in_safe_house = true;
        let logic = Box::new(CareerCarSelectLogic::new(entry));
        let commands = self.screens.open_with(CAR_SELECT, logic, true, env);
        self.apply(commands, host, env);
    }

    /// Carries out what the career car select asked for. A refusal is shown in a notice.
    pub(super) fn career_car_action(&mut self, action: CarSelectAction, host: &mut Host, env: &mut Env) {
        let refused = match action {
            CarSelectAction::Open(entry) => return self.open_career_car_select(entry, host, env),
            CarSelectAction::Release { handle, marker } => {
                self.career.with(|career| car_select::release(career, handle, marker))
            }
            CarSelectAction::UseStrikeMarker { handle } => {
                self.career.with(|career| car_select::use_strike_marker(career, handle))
            }
        };
        let Some(Err(text)) = refused else { return };
        let alert = Alert { title: Text::Custom(String::new()), message: text, on_close: None };
        let commands = self.screens.open_with(DIALOG, Box::new(AlertLogic::new(alert)), false, env);
        self.apply(commands, host, env);
    }
}
