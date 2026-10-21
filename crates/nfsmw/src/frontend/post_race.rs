//! The post-race results screen (`PostRace_Results.fng`): the lap table of one racer, with the racer's name above it
//! and the shoulder arrows to look at the next racer. Exit goes on in free roam, Restart runs the race again. The
//! flow opens it with the race's [`RaceReport`] (`flow.rs`, `Screens::open_with`). What each object is and where the
//! flow comes from: `docs/specs/race-mode.md` section 11 and `docs/provenance/race-mode.md` section 10.

use blackbox_feng::ids::{PAD_ACCEPT, PAD_BUTTON0, PAD_LTRIGGER, PAD_RTRIGGER};

use super::flow::{Frontend, Stage};
use super::ids::screen::{MILESTONE_REWARDS, PHOTO, POST_RACE_PURSUIT, POST_RACE_RESULTS};
use super::logic::{Command, Cx, Env, ScreenLogic};
use crate::app::Host;
use crate::viewer::{RaceAnswer, RaceReport, ReportRacer};

/// The racer's name over the table: the group whose two strings (shadow and face) hold it.
const NAME_GROUP: u32 = 0xEB43_CCB0;
/// The table's rows, by the name hashes of their strings: the lap number, the place and the lap's time. The strings
/// of row `n` (1 to 9) are consecutive hashes from the first row's; the tenth and the eleventh are named on their own.
const FIRST_LAP: u32 = 0x38B1_36A8;
const FIRST_PLACE: u32 = 0x85AB_7369;
const FIRST_TIME: u32 = 0xD2A5_B02A;
/// The groups of the rows 1 to 10, top to bottom (the package's draw order is not the row order).
const ROW_GROUPS: [u32; 10] = [
    0x57F1_7624,
    0xA4EB_B2E5,
    0xF1E5_EFA6,
    0x3EE0_2C67,
    0x8BDA_6928,
    0xD8D4_A5E9,
    0x25CE_E2AA,
    0x72C9_1F6B,
    0xBFC3_5C2C,
    0x6B65_6154,
];
/// The strings of the tenth row (lap, place, time), and the group of the eleventh row, which stays hidden.
const TENTH: [u32; 3] = [0x4ED8_0BD8, 0x3B19_E0B9, 0x275B_B59A];
const ELEVENTH_GROUP: u32 = 0xB85F_9E15;
/// The buttons this build has no screen for (the stats page, the online gamer card, mute and feedback), hidden, by GUID.
const UNUSED: [u32; 6] = [0xE690F, 0xFA5B3, 0xFD000, 0x10688A, 0x122661, 0x1062F9];

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
        self.latch = true;
        self.paused.set(true);
        self.stage = Stage::Paused;
        let logic = PostRaceLogic::new(report);
        let commands = self.screens.open_with(POST_RACE_RESULTS, Box::new(logic), true, env);
        self.apply(commands, host, env);
    }

    /// Freezes the game and puts a screen over it.
    fn open_over_game(&mut self, name: &str, logic: Box<dyn ScreenLogic>, host: &mut Host, env: &mut Env) {
        self.latch = true;
        self.paused.set(true);
        self.stage = Stage::Paused;
        let commands = self.screens.open_with(name, logic, true, env);
        self.apply(commands, host, env);
    }

    /// The lap table, from the photo's Race Stats button.
    pub(super) fn show_results(&mut self, report: RaceReport, host: &mut Host, env: &mut Env) {
        let commands = self.screens.open_with(POST_RACE_RESULTS, Box::new(PostRaceLogic::new(report)), true, env);
        self.apply(commands, host, env);
    }

    /// The answer on the results screen: the race runs again or ends, and the game goes on driving.
    pub(super) fn race_answer(&mut self, answer: crate::viewer::RaceAnswer, host: &mut Host, env: &mut Env) {
        if !host.scene.answer_race(answer) {
            return;
        }
        log::info!("race: the results screen answered {answer:?}");
        // An event of the challenge series ends on the series screen.
        if self.series.active && matches!(answer, crate::viewer::RaceAnswer::Exit) {
            return self.show_series(host, env);
        }
        self.resume();
    }
}

/// Shows a [`RaceReport`].
pub struct PostRaceLogic {
    report: RaceReport,
    shown: usize,
}

impl PostRaceLogic {
    pub fn new(report: RaceReport) -> Self {
        Self { report, shown: 0 }
    }

    fn racer(&self) -> Option<&ReportRacer> {
        self.report.racers.get(self.shown)
    }

    /// Looks at the racer `step` places on, wrapping.
    fn step(&mut self, cx: &mut Cx, step: isize) {
        let n = self.report.racers.len() as isize;
        if n <= 1 {
            return;
        }
        self.shown = (self.shown as isize + step).rem_euclid(n) as usize;
        self.fill(cx);
    }

    fn fill(&self, cx: &mut Cx) {
        let Some(racer) = self.racer() else { return };
        cx.text_group(NAME_GROUP, &racer.name.to_uppercase());
        for (row, group) in ROW_GROUPS.iter().enumerate() {
            let lap = racer.laps.get(row);
            cx.hide(*group, lap.is_none());
            let Some(lap) = lap else { continue };
            let (lap_name, place_name, time_name) = names(row);
            cx.text_object(lap_name, &(row + 1).to_string());
            cx.text_object(place_name, &lap.place.to_string());
            cx.text_object(time_name, &lap_time(lap.time));
        }
    }
}

/// The name hashes of the strings of row `row` (from 0): lap number, place, time.
fn names(row: usize) -> (u32, u32, u32) {
    if row >= 9 {
        return (TENTH[0], TENTH[1], TENTH[2]);
    }
    let n = row as u32;
    (FIRST_LAP + n, FIRST_PLACE + n, FIRST_TIME + n)
}

/// A lap's time as the package's text shows it: `mm:ss.cc`.
pub fn lap_time(secs: f32) -> String {
    let hundredths = (secs.max(0.0) * 100.0).round() as u64;
    format!("{:02}:{:02}.{:02}", hundredths / 6000, (hundredths / 100) % 60, hundredths % 100)
}

/// Hides the objects this build has no use for.
fn hide_unused(cx: &mut Cx) {
    cx.hide(ELEVENTH_GROUP, true);
    for guid in UNUSED {
        let Some(o) = cx.guid(guid) else { continue };
        cx.rt.set_hidden(o, true);
    }
}

impl ScreenLogic for PostRaceLogic {
    fn start(&mut self, cx: &mut Cx) {
        self.fill(cx);
        hide_unused(cx);
    }

    /// The package's scripts show these again as the screen opens, so they are held hidden every frame.
    fn tick(&mut self, cx: &mut Cx, _dt: f32) {
        hide_unused(cx);
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        match message {
            PAD_ACCEPT | PAD_BUTTON0 => cx.send(Command::RaceAnswer(RaceAnswer::Exit)),
            m if m == PAD_BUTTON0 + 1 => cx.send(Command::RaceAnswer(RaceAnswer::Restart)),
            PAD_LTRIGGER => self.step(cx, -1),
            PAD_RTRIGGER => self.step(cx, 1),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_lap_reads_minutes_seconds_and_hundredths() {
        assert_eq!(lap_time(61.2), "01:01.20");
        assert_eq!(lap_time(0.0), "00:00.00");
        assert_eq!(lap_time(-4.0), "00:00.00");
    }

    #[test]
    fn the_rows_name_consecutive_strings_up_to_the_ninth() {
        assert_eq!(names(0), (0x38B1_36A8, 0x85AB_7369, 0xD2A5_B02A));
        assert_eq!(names(8), (0x38B1_36B0, 0x85AB_7371, 0xD2A5_B032));
        assert_eq!(names(9), (0x4ED8_0BD8, 0x3B19_E0B9, 0x275B_B59A));
    }
}
