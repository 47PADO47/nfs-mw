//! The first safe house visit (`docs/specs/story-flow.md` section 2.6, `docs/specs/story-flow-2.md` sections 4.6 and F-18).
//!
//! The safe house request at stage 16 (the prologue's end) plays `storyfmv_saf25`, then opens the rival bio. The bio, opened
//! in the Blacklist sequence, enters bin 15 (stage 16 to 15). Its Accept sends the stepper on from counter 4: the rival
//! movie `blacklist_15` plays, and when it is over step 6 sets the "opening seen" flag and opens the reputation overview.
//!
//! The rules are the `nfsmw_career` ones: [`FIRST_VISIT_MOVIE`], [`enter_first_bin`] and [`BlacklistStepper`]. The stepper
//! state travels in [`Command::BlacklistStep`], so the front end keeps no counter of its own.

use nfsmw_career::blacklist::enter_first_bin;
use nfsmw_career::safehouse::{BlacklistStepper, FIRST_VISIT_MOVIE, FrontEndContext, StepAction};
use nfsmw_career::settings::PROLOGUE_STAGE;

use crate::app::Host;
use crate::career::CareerHandle;
use crate::frontend::ids::screen::{
    MAIN_MENU_SUB, SAFE_HOUSE_RIVAL_BIO, SAFE_HOUSE_RIVAL_CHALLENGE, SAFEHOUSE_MARKERS, SAFEHOUSE_REGION_UNLOCK,
};
use crate::frontend::logic::{Args, Command, Env};

use super::Frontend;
use super::Stage;
use super::flow_movie::AfterMovie;

/// The rap sheet movie of step 6 at stage 13 (`storyfmv_rap30`, story-flow-2 F-20).
pub(in crate::frontend) const RAP_SHEET_MOVIE: &str = "storyfmv_rap30";

/// The counter the first visit's bio leaves the stepper at. The stepper starts at -1 and the bio sets 4, so its next call
/// is step 5, the rival movie (story-flow-2 F-18).
pub(in crate::frontend) const BIO_COUNTER: i32 = 4;

/// The rival movie of a stage: `blacklist_NN`, the stage byte in two digits (story-flow-2 F-19).
pub(in crate::frontend) fn rival_movie(stage: u8) -> String {
    format!("blacklist_{stage:02}")
}

/// The command the rival bio sends when the first visit leaves it (Accept). Back does the same: the sequence has no other
/// way on, and a bio left without it would never open the rival movie or set the opening seen flag.
pub(in crate::frontend) fn bio_continue() -> Command {
    Command::BlacklistStep(BlacklistStepper { counter: BIO_COUNTER })
}

/// The rival bio opens in the Blacklist sequence: at stage 16 the first visit enters bin 15 with the full bin entry
/// (`enter_first_bin`: the stage byte, the bin's milestones and speed traps). True when bin 15 was entered, which tells the
/// bio that its Accept is the first visit's.
pub(in crate::frontend) fn enter_first_bin_on_open() -> bool {
    let entered = CareerHandle::shared().with(|career| {
        enter_first_bin(&mut career.settings, &mut career.blacklist, &mut career.milestones, &mut career.traps)
    });
    let Some(outcome) = entered else { return false };
    if outcome.bins_entered.is_empty() {
        return false;
    }
    log::info!("career: the first safe house visit entered bins {:?}", outcome.bins_entered);
    true
}

impl Frontend {
    /// The safe house request of the world at stage 16: the first visit plays `storyfmv_saf25`, and the rival bio follows it
    /// (`Args::sequence`). True when the intro played. False leaves the request to the safe house menu: any other stage, or a
    /// movie that does not open.
    pub(super) fn first_visit_intro(&mut self, host: &mut Host) -> bool {
        let prologue = self.career.with(|career| career.stage() == PROLOGUE_STAGE).unwrap_or(false);
        if !prologue {
            return false;
        }
        let args = Args { safehouse: true, sequence: true, ..Args::default() };
        self.play_movie(FIRST_VISIT_MOVIE, AfterMovie::Open(SAFE_HOUSE_RIVAL_BIO.to_owned(), args), host);
        if !matches!(self.stage, Stage::Movie { .. }) {
            return false;
        }
        // The movie replaces the world, as the original unloads it before the safe house exists. The safe house menu
        // stands before the world now, and Resume Free Roam starts it (`start_from_safe_house`).
        self.career.set_prelude(true);
        self.in_safe_house = true;
        true
    }

    /// The Blacklist sequence goes on from `stepper`: one call of the stepper, then the screen, movie or save the step names.
    /// Screens hand the sequence back when they close (`flow_arrival.rs`), and movies when they are over.
    pub(super) fn blacklist_step(&mut self, mut stepper: BlacklistStepper, host: &mut Host, env: &mut Env) {
        let stepped = self.career.with(|career| {
            let mut context = FrontEndContext::CAREER;
            context.insert(FrontEndContext::BLACKLIST_SEQUENCE);
            // The autosave gate was evaluated once when the arrival started (`flow_arrival.rs`); the first visit never
            // reaches step 3, so its flag stays false.
            let action = stepper.advance(&mut career.settings, &mut context, career.arrival_autosave);
            (action, career.settings.stage)
        });
        let Some((action, stage)) = stepped else {
            return log::warn!("career: no career is played, so the Blacklist sequence stops");
        };
        log::info!("career: Blacklist sequence {action:?} at stage {stage}");
        match action {
            StepAction::RivalMovie => self.sequence_movie(&rival_movie(stage), stepper, host, env),
            StepAction::RapSheetMovie => self.sequence_movie(RAP_SHEET_MOVIE, stepper, host, env),
            StepAction::LeaveFrontEnd => self.start_from_safe_house(host, env),
            StepAction::RivalChallenge => {
                self.sequence_screen(SAFE_HOUSE_RIVAL_CHALLENGE, safe_house_args(), stepper, host, env)
            }
            StepAction::RegionUnlock => {
                self.sequence_screen(SAFEHOUSE_REGION_UNLOCK, safe_house_args(), stepper, host, env)
            }
            StepAction::RivalBio => self.sequence_screen(SAFE_HOUSE_RIVAL_BIO, safe_house_args(), stepper, host, env),
            StepAction::Markers => self.sequence_screen(SAFEHOUSE_MARKERS, safe_house_args(), stepper, host, env),
            // The autosave (step 3) writes the profile with no screen.
            StepAction::AutosaveFlow => {
                self.sequence_save();
                self.blacklist_step(stepper, host, env);
            }
            // Step 6 at stage 15 opens the reputation overview. No screen of that name exists yet, so the safe house menu
            // stands in for it. Step 7 ends the sequence in the safe house menu too.
            StepAction::ReputationOverview | StepAction::End => self.open_safe_house(host, env),
        }
    }

    /// Plays a movie of the sequence; when it is over the sequence goes on from `stepper`. A movie that does not open is
    /// skipped and the sequence goes on at once.
    fn sequence_movie(&mut self, movie: &str, stepper: BlacklistStepper, host: &mut Host, env: &mut Env) {
        let then = AfterMovie::Command(Box::new(Command::BlacklistStep(stepper)));
        self.play_movie(movie, then, host);
        if matches!(self.stage, Stage::Movie { .. }) {
            return;
        }
        self.blacklist_step(stepper, host, env);
    }

    /// The safe house menu, over the menu backdrop: where the sequence ends. The menu stands before the world (the world was
    /// replaced by the intro movie), so its Resume Free Roam starts the world.
    pub(super) fn open_safe_house(&mut self, host: &mut Host, env: &mut Env) {
        self.in_safe_house = true;
        self.latch = true;
        self.screens.clear();
        self.open_menu(MAIN_MENU_SUB, Args { safehouse: true, ..Args::default() }, host, env);
    }
}

/// The arguments of a safe house screen of the sequence.
fn safe_house_args() -> Args {
    Args { safehouse: true, ..Args::default() }
}

#[cfg(test)]
#[path = "flow_first_visit_tests.rs"]
mod tests;
