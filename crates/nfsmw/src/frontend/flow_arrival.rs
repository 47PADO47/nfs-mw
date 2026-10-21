//! The arrival sequence after a rival is beaten (`docs/specs/story-flow-2.md` section 4.6, F-18 to F-20). The steps are the
//! ones of the Blacklist sequence: [`Frontend::blacklist_step`] (`flow_first_visit.rs`) runs one call of the stepper and
//! opens what the step names. This file starts the sequence at the safe house entry, carries the screens' way out on to
//! the next step, and saves the profile for the autosave step.
//!
//! The sequence runs over the menu backdrop: its first screen releases the world, as the tutorial movies do, and leaving
//! it starts free roam again. A step screen is wrapped ([`StepScreen`]) so that its exit (the switch to the safe house
//! menu, or the region screen's Resume) hands the sequence back with the stepper state of that step.
//!
//! The rules are the stepper's; the tests here walk it through the steps of each stage.

use glam::Vec2;
use nfsmw_career::safehouse::{BlacklistStepper, FrontEndContext, StepAction};
use nfsmw_career::savefile::GateInputs;

use crate::app::Host;
use crate::career::CareerHandle;
use crate::frontend::factory;
use crate::frontend::ids::screen;
use crate::frontend::logic::{Args, Command, Cx, Env, Footer, Pick, ScreenLogic};
use crate::frontend::look::Look;
use crate::frontend::name_entry::TextKey;
use crate::ui::UiAssets;
use crate::ui::theme::MenuList;

use super::{Frontend, Stage};

/// True when a rival was beaten and its arrival sequence is due at the next safe house entry.
pub(super) fn is_due(career: &CareerHandle) -> bool {
    career.with_loaded(|career| career.rival_beaten.is_some()).unwrap_or(false)
}

/// Takes the pending arrival: true when one was due, which the safe house entry then runs instead of the menu.
pub(super) fn take_due(career: &CareerHandle) -> bool {
    career.with_loaded(|career| career.rival_beaten.take().is_some()).unwrap_or(false)
}

impl Frontend {
    /// The safe house is entered with an arrival due: the autosave gate is read once, the safe house menu stands before the
    /// world, and the sequence starts from the stepper's start value.
    pub(super) fn start_arrival(&mut self, host: &mut Host, env: &mut Env) {
        let autosave = self
            .career
            .with_loaded(|career| {
                let possible = career.autosave.can_autosave(&GateInputs::unforced(None));
                career.arrival_autosave = possible;
                possible
            })
            .unwrap_or(false);
        log::info!("arrival: a rival is beaten; autosave possible: {autosave}");
        self.career.set_prelude(true);
        self.in_safe_house = true;
        self.latch = true;
        self.screens.clear();
        self.blacklist_step(BlacklistStepper::new(), host, env);
    }

    /// Opens a screen of the Blacklist sequence over the menu backdrop. Its exit goes on with the sequence from `stepper`,
    /// the state after this step.
    pub(super) fn sequence_screen(
        &mut self,
        name: &'static str,
        args: Args,
        stepper: BlacklistStepper,
        host: &mut Host,
        env: &mut Env,
    ) {
        log::info!("career: Blacklist sequence screen {name}");
        self.latch = true;
        self.screens.clear();
        self.show_backdrop(host);
        let mod_rows = env.mods.is_some_and(crate::modding::Mods::has_mods_page);
        let look = factory::look_for(name, &args);
        let inner = factory::make(name, args, mod_rows);
        let commands = self.screens.open_with(name, Box::new(StepScreen { inner, look, next: stepper }), true, env);
        self.stage = Stage::Menus;
        self.sync_backdrop(host);
        self.apply(commands, host, env);
    }

    /// The autosave step: the profile is written with no screen (the autosave screen is not built).
    pub(super) fn sequence_save(&self) {
        let Some(profiles) = self.profiles.as_ref() else {
            return log::warn!("arrival: no profile folder; the autosave is not written");
        };
        let Some((alias, bytes)) = self.career.with_loaded(|career| (career.alias.clone(), career.profile_bytes()))
        else {
            return;
        };
        match profiles.write(&alias, &bytes) {
            Ok(()) => log::info!("arrival: autosaved {alias}"),
            Err(e) => log::warn!("arrival: the autosave of {alias} failed: {e:#}"),
        }
    }
}

/// Whether a command is the way out of a step screen: the switch to the safe house menu, or the Resume the region screen
/// sends when it is done.
fn is_step_exit(command: &Command) -> bool {
    match command {
        Command::Switch(name, _) => name.eq_ignore_ascii_case(screen::MAIN_MENU_SUB),
        Command::Resume => true,
        _ => false,
    }
}

/// A step screen of the Blacklist sequence: its logic, with the look the screen asks for. The logic is unchanged, except
/// that its exits (see [`is_step_exit`]) become `Command::BlacklistStep(next)`.
struct StepScreen {
    inner: Box<dyn ScreenLogic>,
    look: Option<Look>,
    /// The stepper state after this step: where the sequence goes on when the screen's exit is taken.
    next: BlacklistStepper,
}

impl StepScreen {
    /// Turns the exits the logic queued since `from` into the sequence's next step.
    fn carry_exits(&self, cx: &mut Cx, from: usize) {
        for command in cx.commands[from..].iter_mut() {
            if is_step_exit(command) {
                *command = Command::BlacklistStep(self.next);
            }
        }
    }
}

impl ScreenLogic for StepScreen {
    fn start(&mut self, cx: &mut Cx) {
        let from = cx.commands.len();
        self.inner.start(cx);
        self.carry_exits(cx, from);
    }

    fn resume(&mut self, cx: &mut Cx) {
        let from = cx.commands.len();
        self.inner.resume(cx);
        self.carry_exits(cx, from);
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        let from = cx.commands.len();
        self.inner.message(cx, message);
        self.carry_exits(cx, from);
    }

    fn pressed(&mut self, cx: &mut Cx, button: u32) {
        let from = cx.commands.len();
        self.inner.pressed(cx, button);
        self.carry_exits(cx, from);
    }

    fn pad_pressed(&mut self, cx: &mut Cx, bits: u32) {
        let from = cx.commands.len();
        self.inner.pad_pressed(cx, bits);
        self.carry_exits(cx, from);
    }

    fn tick(&mut self, cx: &mut Cx, dt: f32) {
        let from = cx.commands.len();
        self.inner.tick(cx, dt);
        self.carry_exits(cx, from);
    }

    fn click(&mut self, cx: &mut Cx, point: Vec2) -> bool {
        let from = cx.commands.len();
        let handled = self.inner.click(cx, point);
        self.carry_exits(cx, from);
        handled
    }

    fn hover(&mut self, cx: &mut Cx, point: Vec2) {
        let from = cx.commands.len();
        self.inner.hover(cx, point);
        self.carry_exits(cx, from);
    }

    fn scroll(&mut self, cx: &mut Cx, steps: i32) {
        let from = cx.commands.len();
        self.inner.scroll(cx, steps);
        self.carry_exits(cx, from);
    }

    fn typed(&mut self, cx: &mut Cx, key: TextKey) {
        let from = cx.commands.len();
        self.inner.typed(cx, key);
        self.carry_exits(cx, from);
    }

    fn pick_row(&mut self, cx: &mut Cx, row: usize, pick: Pick) {
        let from = cx.commands.len();
        self.inner.pick_row(cx, row, pick);
        self.carry_exits(cx, from);
    }

    fn list(&self, assets: &UiAssets) -> Option<MenuList> {
        self.inner.list(assets)
    }

    fn vertical(&self) -> bool {
        self.inner.vertical()
    }

    fn rotate_pad(&self) -> bool {
        self.inner.rotate_pad()
    }

    fn wants_text(&self) -> bool {
        self.inner.wants_text()
    }

    fn footer(&self) -> Footer {
        self.inner.footer()
    }

    fn look(&self) -> Option<Look> {
        self.look.clone().or_else(|| self.inner.look())
    }
}

#[cfg(test)]
mod tests {
    use nfsmw_career::settings::{CareerSettings, Flags};

    use super::*;
    use crate::frontend::flow::flow_first_visit::{RAP_SHEET_MOVIE, rival_movie};

    /// The steps the stepper takes at `stage`, up to its leave or its end, with autosave possible or not and the rap sheet
    /// flag set or not. The flow carries out each one (`Frontend::blacklist_step`).
    fn steps(stage: u8, autosave: bool, rap_sheet: bool) -> Vec<StepAction> {
        let mut settings = CareerSettings::new_career();
        settings.stage = stage;
        if rap_sheet {
            settings.flags.insert(Flags::HAS_RAP_SHEET);
        }
        let mut context = FrontEndContext::CAREER;
        context.insert(FrontEndContext::BLACKLIST_SEQUENCE);
        let mut stepper = BlacklistStepper::new();
        let mut taken = Vec::new();
        loop {
            let action = stepper.advance(&mut settings, &mut context, autosave);
            taken.push(action);
            if matches!(action, StepAction::LeaveFrontEnd | StepAction::End) {
                return taken;
            }
        }
    }

    use StepAction::*;

    #[test]
    fn stage_14_has_no_region_screen_and_plays_blacklist_14_then_leaves() {
        let taken = steps(14, false, false);
        assert_eq!(taken, [RivalChallenge, Markers, RivalBio, RivalMovie, LeaveFrontEnd]);
        assert_eq!(rival_movie(14), "blacklist_14");
    }

    #[test]
    fn stage_12_opens_the_region_screen_after_the_marker_choice() {
        assert_eq!(
            steps(12, false, false),
            [RivalChallenge, Markers, RegionUnlock, RivalBio, RivalMovie, LeaveFrontEnd]
        );
    }

    #[test]
    fn stage_13_without_the_rap_sheet_plays_rap30_and_then_leaves() {
        assert_eq!(
            steps(13, false, false),
            [RivalChallenge, Markers, RivalBio, RivalMovie, RapSheetMovie, LeaveFrontEnd]
        );
        assert_eq!(RAP_SHEET_MOVIE, "storyfmv_rap30");
    }

    #[test]
    fn stage_13_with_the_rap_sheet_leaves_after_its_rival_movie() {
        assert_eq!(steps(13, false, true), [RivalChallenge, Markers, RivalBio, RivalMovie, LeaveFrontEnd]);
    }

    #[test]
    fn autosave_comes_before_the_bio_when_it_is_possible() {
        assert_eq!(
            steps(14, true, false),
            [RivalChallenge, Markers, AutosaveFlow, RivalBio, RivalMovie, LeaveFrontEnd]
        );
    }

    #[test]
    fn stage_15_ends_at_the_reputation_overview_stand_in_and_the_end() {
        assert_eq!(steps(15, false, false), [RivalChallenge, Markers, RivalBio, RivalMovie, ReputationOverview, End]);
    }

    #[test]
    fn only_the_switch_to_the_safe_house_menu_and_resume_are_step_exits() {
        assert!(is_step_exit(&Command::Resume));
        assert!(is_step_exit(&Command::Switch(screen::MAIN_MENU_SUB.to_owned(), Args::default())));
        assert!(!is_step_exit(&Command::Switch(screen::SAFE_HOUSE_RIVAL_BIO.to_owned(), Args::default())));
        assert!(!is_step_exit(&Command::ChallengeRival { bin: 14 }));
    }
}
