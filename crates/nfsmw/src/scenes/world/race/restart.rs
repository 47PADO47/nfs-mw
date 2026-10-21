//! Asking about the race in progress and running it again: the pause menu's Resume Race / Restart items.
//! Spec: `docs/specs/race-restart.md`; the executable's side is in `docs/provenance/race-restart.md`.

use blackbox_race::Course;
use nfsmw_data::gameplay::Race;

use super::{RaceOptions, RaceSession};
use crate::scenes::world::WorldScene;

/// What a restart runs again: the race, the options it was started with and the course a mod gave it.
type Kit = (Race, RaceOptions, Option<Course>);

/// The race and options to start `session` again, or `None` once its result is fixed (the career has been told by then,
/// and the post-race screen's own Restart is the way to go again).
fn restart_kit(session: &RaceSession) -> Option<Kit> {
    session.stage.is_live().then(|| (session.origin.race.clone(), session.options, session.origin.course.clone()))
}

impl WorldScene {
    /// Whether the pause menu offers Resume Race and Restart: a race is staged (asked for and waiting for the grid, on the
    /// grid, counting down) or running, and its result is not fixed yet. Pursuit challenges are races; the free-roam
    /// speed traps and the career's events that are not races have no session, so they are not.
    pub(in crate::scenes::world) fn race_in_progress(&self) -> bool {
        match self.race.as_ref() {
            Some(session) => session.stage.is_live(),
            None => self.queued_race.is_some(),
        }
    }

    /// Runs the race in progress again from its grid with the same parameters. The racers, the clock, the checkpoints, the
    /// laps and the place are made again; the player's car goes back to the start; the cops are reset. The career is not
    /// entered again (no autosave at the event start, no roaming autosave) and nothing is paid: the result is applied
    /// only when a race finishes, and this one did not. `false` when there is no race to restart.
    pub(in crate::scenes::world) fn restart_race_now(&mut self) -> bool {
        // A race that is only asked for starts from its grid anyway.
        if self.race.is_none() {
            return self.queued_race.is_some();
        }
        let Some(mut session) = self.race.take_if(|s| s.stage.is_live()) else { return false };
        let Some((race, options, course)) = restart_kit(&session) else { return false };
        log::info!("race: {} restarts", session.name);
        self.release_race(&mut session);
        // The original resets the cops as the restart begins (`ResetCopsForRestart`): a chase, its roadblocks and spike
        // strips end, so the new start is clear of them.
        if let Some(traffic) = self.traffic.as_mut() {
            traffic.stop_pursuit();
        }
        // The damage of the abandoned run does not carry over (the car's state at the first start is not kept, and a repair
        // costs nothing in the rewrite) **[ours]**.
        self.repair_player_car();
        if let Err(why) = self.begin_race(race, options, course) {
            log::warn!("race {}: the restart did not start: {why}", session.name);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::world::race::{Banner, Origin, Stage};
    use blackbox_race::Gate;
    use glam::Vec3;
    use nfsmw_data::gameplay::{RaceParams, RaceType};

    fn race() -> Race {
        Race {
            key: 7,
            name: "race_bin_10/10_2_1_sprint".into(),
            event: None,
            event_icon: "p2p".into(),
            race_type: Some(RaceType::Sprint),
            region: None,
            laps: 1,
            start: None,
            finish: None,
            checkpoints: Vec::new(),
            barriers: Vec::new(),
            params: RaceParams::default(),
        }
    }

    fn session(stage: Stage, options: RaceOptions, course: Option<Course>) -> RaceSession {
        RaceSession {
            name: "race_bin_10/10_2_1_sprint".into(),
            event: "10.2.1".into(),
            kind: RaceType::Sprint,
            options,
            stage,
            engine: None,
            plan: Vec::new(),
            setup: None,
            previous_traffic: None,
            glue: Vec::new(),
            catch_up: false,
            glue_clock: 0.0,
            demo_clock: 0.0,
            lane: Default::default(),
            emu: Vec::new(),
            log_clock: 0.0,
            banner: Banner::default(),
            result: None,
            report: None,
            chain: None,
            cash_value: 0.0,
            pool: Vec::new(),
            challenge: None,
            origin: Origin { race: race(), course },
            map: crate::hud::MapSource::default(),
            walls: Default::default(),
        }
    }

    #[test]
    fn a_race_is_live_until_its_result_is_fixed() {
        assert!(Stage::Loading { waited: 3.0 }.is_live(), "staged: waiting for the grid");
        assert!(Stage::Running.is_live(), "the countdown and the race itself");
        assert!(!Stage::Results { left: 2.0 }.is_live(), "the race-over message: the result is fixed");
        assert!(!Stage::PostRace { left: 5.0 }.is_live(), "the post-race screen has the race");
    }

    #[test]
    fn a_restart_runs_the_same_race_with_the_same_options_and_course() {
        let options =
            RaceOptions { laps: Some(3), opponents: Some(2), career: true, cash: Some(900.0), ..Default::default() };
        let gate = Gate::new(Vec3::ZERO, Vec3::Z, Vec3::Y, [2.0, 40.0, 10.0]);
        let course = Course::new(Vec3::ZERO, vec![gate], None, 3);
        let live = session(Stage::Running, options, Some(course.clone()));
        let (again, kept, given) = restart_kit(&live).expect("a running race restarts");
        assert_eq!(again.name, "race_bin_10/10_2_1_sprint");
        assert_eq!(kept, options, "a career race stays a career race, with the cash and laps it had");
        assert_eq!(given, Some(course), "a mod's course is not made again from the install's data");
        let plain = session(Stage::Loading { waited: 0.0 }, RaceOptions::default(), None);
        assert_eq!(restart_kit(&plain).map(|kit| kit.2), Some(None), "a stock race builds its course from the data");
    }

    #[test]
    fn a_race_with_a_fixed_result_has_nothing_to_restart() {
        let done = session(Stage::Results { left: 1.0 }, RaceOptions { career: true, ..Default::default() }, None);
        assert!(restart_kit(&done).is_none(), "restarting after the finish would pay the win twice");
        assert!(restart_kit(&session(Stage::PostRace { left: 1.0 }, RaceOptions::default(), None)).is_none());
    }
}
