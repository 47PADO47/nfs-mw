//! Asking about the race in progress and running it again: the pause menu's Resume Race / Restart items.
//! Spec: `docs/specs/race-restart.md`; the executable's side is in `docs/provenance/race-restart.md`.

use blackbox_race::Course;
use nfsmw_data::gameplay::Race;

use super::{RaceOptions, RaceSession};
use crate::scenes::world::WorldScene;

/// What a restart runs again: the race, the options it was started with and the course a mod gave it.
pub(in crate::scenes::world) type Kit = (Race, RaceOptions, Option<Course>);

/// What running `session` again takes, whatever its stage: the post-race screen's Restart asks for it after the result.
pub(super) fn again_kit(session: &RaceSession) -> Kit {
    (session.origin.race.clone(), session.options, session.origin.course.clone())
}

/// The race and options to start `session` again, or `None` once its result is fixed (the career has been told by then,
/// and the post-race screen's own Restart is the way to go again).
fn restart_kit(session: &RaceSession) -> Option<Kit> {
    session.stage.is_live().then(|| again_kit(session))
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
        // The same two players race again.
        self.queued_split = self.split.as_ref().map(|split| split.setup.clone());
        if let Err(why) = self.begin_again((race, options, course)) {
            log::warn!("race {}: the restart did not start: {why}", session.name);
        }
        true
    }

    /// Starts `kit` as a restart: from the pre-race state, past the intro's tutorial, messages, movie and cutscene
    /// (`docs/specs/race-wrongway.md` section 3). The pause menu's Restart and the post-race screen's both come here.
    pub(in crate::scenes::world) fn begin_again(&mut self, (race, options, course): Kit) -> Result<String, String> {
        let line = self.begin_race(race, options, course)?;
        if let Some(started) = self.race.as_mut() {
            started.restarted = true;
        }
        Ok(line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::world::race::intro::intro_queue;
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
            story: Default::default(),
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
            heat_chase: None,
            previous_traffic: None,
            glue: Vec::new(),
            catch_up: false,
            glue_clock: 0.0,
            demo_clock: 0.0,
            lane: Default::default(),
            emu: Vec::new(),
            log_clock: 0.0,
            banner: Banner::default(),
            lap_mark: 0.0,
            result: None,
            report: None,
            chain: None,
            cash_value: 0.0,
            pool: Vec::new(),
            challenge: None,
            origin: Origin { race: race(), course },
            route: None,
            restarted: false,
            map: crate::hud::MapSource::default(),
            walls: Default::default(),
            collisions: 0,
        }
    }

    #[test]
    fn only_a_pursuit_race_or_a_challenge_has_police() {
        let plain = session(Stage::Running, RaceOptions::default(), None);
        assert!(!plain.has_police(), "an ordinary race, whatever its CopsInRace says");
        let mut pursuit = session(Stage::Running, RaceOptions::default(), None);
        pursuit.origin.race.params.pursuit_race = true;
        assert!(pursuit.has_police());
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
    fn a_restart_goes_back_past_the_intro() {
        // A career race plays its tutorial and messages before the grid; its restart enters the pre-race state of the
        // original, which comes after them (`docs/specs/race-wrongway.md` section 3).
        let options = RaceOptions { career: true, ..Default::default() };
        let mut again = session(Stage::Loading { waited: 0.0 }, options, None);
        assert!(!intro_queue(&again).is_empty(), "the first start has its intro");
        again.restarted = true;
        assert!(intro_queue(&again).is_empty(), "the restart counts down at once");
    }

    #[test]
    fn the_post_race_restart_runs_the_same_race_on_the_same_course() {
        let gate = Gate::new(Vec3::ZERO, Vec3::Z, Vec3::Y, [2.0, 40.0, 10.0]);
        let course = Course::new(Vec3::ZERO, vec![gate], None, 3);
        let options = RaceOptions { career: true, laps: Some(2), ..Default::default() };
        let done = session(Stage::PostRace { left: 1.0 }, options, Some(course.clone()));
        let (again, kept, given) = again_kit(&done);
        assert_eq!(again.name, "race_bin_10/10_2_1_sprint");
        assert_eq!(kept, options);
        assert_eq!(given, Some(course), "a mod's course survives the post-race restart");
    }

    #[test]
    fn a_race_with_a_fixed_result_has_nothing_to_restart() {
        let done = session(Stage::Results { left: 1.0 }, RaceOptions { career: true, ..Default::default() }, None);
        assert!(restart_kit(&done).is_none(), "restarting after the finish would pay the win twice");
        assert!(restart_kit(&session(Stage::PostRace { left: 1.0 }, RaceOptions::default(), None)).is_none());
    }
}
