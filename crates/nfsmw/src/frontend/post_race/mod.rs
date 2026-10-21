//! The post-race results screen (`PostRace_Results.fng`): the standings, the statistics and the stages of the race, one
//! page at a time. The flow opens it with the race's [`RaceReport`] (`Frontend::poll_race`, `Screens::open_with`); the
//! pages' rows are `pages.rs` and the screen's logic is `screen.rs`. What each object is and where the flow comes from:
//! `docs/specs/race-results.md`, `docs/specs/race-mode.md` section 11 and `docs/provenance/race-results-screens.md`.

#[cfg(test)]
mod install_tests;
mod pages;
#[cfg(test)]
mod pages_tests;
mod screen;

pub use screen::PostRaceLogic;

use super::flow::{Frontend, Stage};
use super::ids::screen::{MILESTONE_REWARDS, PHOTO, POST_RACE_PURSUIT, POST_RACE_RESULTS};
use super::logic::{Env, ScreenLogic};
use crate::app::Host;
use crate::viewer::{RaceAnswer, RaceReport};

/// A time as the pursuit result's text shows it: `mm:ss.cc`.
pub fn lap_time(secs: f32) -> String {
    let hundredths = (secs.max(0.0) * 100.0).round() as u64;
    format!("{:02}:{:02}.{:02}", hundredths / 6000, (hundredths / 100) % 60, hundredths % 100)
}

impl Frontend {
    /// Opens the post-race results when the world's race reaches them: over the frozen game, as the ticket is.
    pub(super) fn poll_race(&mut self, host: &mut Host, env: &mut Env) {
        if self.stage != Stage::Playing {
            return;
        }
        if let Some(rewards) = host.scene.take_rewards_report() {
            log::info!("chase: the rewards screen is up ({} milestones)", rewards.lines.len());
            self.latch = true;
            self.paused.set(true);
            self.stage = Stage::Paused;
            let logic = super::rewards::RewardsLogic::new(rewards);
            let commands = self.screens.open_with(MILESTONE_REWARDS, Box::new(logic), true, env);
            self.apply(commands, host, env);
            return;
        }
        if let Some(photo) = host.scene.take_trap_photo() {
            log::info!("career: the speed trap photo is up");
            self.open_over_game(PHOTO, Box::new(super::photo::PhotoLogic::for_trap(photo)), host, env);
            return;
        }
        let Some(report) = host.scene.take_race_report() else { return };
        if let Some(pursuit) = report.pursuit {
            log::info!("race: the pursuit result is up");
            let logic = super::pursuit_results::PursuitResultsLogic::new(pursuit);
            self.open_over_game(POST_RACE_PURSUIT, Box::new(logic), host, env);
            return;
        }
        if let Some(photo) = report.photo {
            log::info!("race: the win photo is up");
            self.open_over_game(PHOTO, Box::new(super::photo::PhotoLogic::for_race(photo, report)), host, env);
            return;
        }
        log::info!("race: the results screen is up ({} racers)", report.racers.len());
        self.open_over_game(POST_RACE_RESULTS, Box::new(PostRaceLogic::new(report)), host, env);
    }

    /// Freezes the game and puts a screen over it.
    fn open_over_game(&mut self, name: &str, logic: Box<dyn ScreenLogic>, host: &mut Host, env: &mut Env) {
        self.latch = true;
        self.paused.set(true);
        self.stage = Stage::Paused;
        let commands = self.screens.open_with(name, logic, true, env);
        self.apply(commands, host, env);
    }

    /// The standings of a race, from the photo's Race Stats button.
    pub(super) fn show_results(&mut self, report: RaceReport, host: &mut Host, env: &mut Env) {
        let commands = self.screens.open_with(POST_RACE_RESULTS, Box::new(PostRaceLogic::new(report)), true, env);
        self.apply(commands, host, env);
    }

    /// The answer on the results screen: the race runs again or ends, and the game goes on driving.
    pub(super) fn race_answer(&mut self, answer: RaceAnswer, host: &mut Host, env: &mut Env) {
        if !host.scene.answer_race(answer) {
            return;
        }
        log::info!("race: the results screen answered {answer:?}");
        // An event of the challenge series ends on the series screen.
        if self.series.active && matches!(answer, RaceAnswer::Exit) {
            return self.show_series(host, env);
        }
        self.resume();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pursuit_time_reads_minutes_seconds_and_hundredths() {
        assert_eq!(lap_time(61.2), "01:01.20");
        assert_eq!(lap_time(0.0), "00:00.00");
        assert_eq!(lap_time(-4.0), "00:00.00");
    }
}
