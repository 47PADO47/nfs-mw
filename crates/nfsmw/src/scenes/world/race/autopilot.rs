//! The autopilot of an unattended race run (`--autopilot`): the opponents' driver takes the player's car in a race.
//! It is the driver a world moment lends the car to (`ai/moment_driver.rs`), told where the race goes as an opponent is
//! (`flow.rs`, `race_goals`): the point past the next gate, the leg and the race's route. Its pedals reach the car as a
//! moment's do, through the car's own input step (`drive/cinematic.rs`), which also gives the car the control set-up
//! the computer's cars drive with. Nothing is placed or moved by hand. Without the option nothing here runs.

use std::sync::Arc;

use blackbox_race::{Phase, Race};
use blackbox_roads::Route;

use super::Stage;
use super::flow::next_aim;
use super::route::leg_of;
use crate::parity::race_run::Autopilot;
use crate::parity::race_trace::Record;
use crate::scenes::world::WorldScene;
use crate::scenes::world::ai::{PilotCar, RaceAim};

/// Where the autopilot drives to now; `None` while it does not have the car: before the green light and its delay, once
/// the player has finished or is out, and outside the running stage.
pub(super) fn pilot_aim(
    stage: &Stage,
    engine: Option<&Race>,
    route: Option<&Arc<Route>>,
    autopilot: Autopilot,
) -> Option<RaceAim> {
    if *stage != Stage::Running {
        return None;
    }
    let engine = engine?;
    if engine.phase() != Phase::Racing || engine.elapsed() < autopilot.after_secs {
        return None;
    }
    let me = engine.racers().first().filter(|racer| racer.racing())?;
    Some(RaceAim {
        goal: next_aim(engine.course(), me.progress.reached),
        leg: leg_of(engine.course(), &me.progress),
        route: route.cloned(),
        skill: autopilot.skill,
    })
}

impl WorldScene {
    /// One physics tick of the autopilot, before the player's car steps: it sets the car's pedals for the tick, or lets
    /// the car go when the race no longer needs it.
    pub(in crate::scenes::world) fn autopilot_tick(&mut self, dt: f32) {
        let Some(autopilot) = self.unattended.plan.autopilot else { return };
        // A world moment has the car for its length (`moment_world.rs`); the road is searched again after it.
        if self.moment.hides_hud() {
            self.unattended.pilot = None;
            return;
        }
        let aim =
            self.race.as_ref().filter(|session| session.challenge.is_none()).and_then(|session| {
                pilot_aim(&session.stage, session.engine.as_ref(), session.route.as_ref(), autopilot)
            });
        let Some(aim) = aim else { return self.release_autopilot() };
        let Some(drive) = self.players.lead() else { return };
        let Some(view) = drive.sim().map(|sim| sim.driver_view()) else { return };
        let car = PilotCar {
            body: drive.ai_body(),
            view,
            performance: drive.ai_performance(),
            ai: nfsmw_data::pursuit::ai_vehicle(self.physics.database(), &drive.car_name),
            mass: drive.physical_mass(),
        };
        let Some(traffic) = self.traffic.as_mut() else { return };
        let input = traffic.race_pilot_input_with_delta(dt, &mut self.unattended.pilot, car, &aim);
        if let Some(drive) = self.players.lead_mut() {
            drive.set_cinematic_input(Some(input));
        }
        if std::mem::replace(&mut self.unattended.engaged, true) {
            return;
        }
        let race_time = self.race.as_ref().and_then(|session| session.engine.as_ref()).map_or(0.0, Race::elapsed);
        log::info!("race run: the autopilot has the player's car at {race_time:.2} s");
        self.unattended.write(|stamp| Record::Autopilot { stamp, race_time });
    }

    /// The autopilot lets the player's car go: the car is the player's again, with its own control set-up.
    pub(super) fn release_autopilot(&mut self) {
        self.unattended.pilot = None;
        if !std::mem::take(&mut self.unattended.engaged) {
            return;
        }
        if let Some(drive) = self.players.lead_mut() {
            drive.set_cinematic_input(None);
        }
    }
}

#[cfg(test)]
mod tests {
    use blackbox_race::{Course, Gate, RaceConfig};
    use glam::Vec3;

    use super::*;

    fn gate(z: f32) -> Gate {
        Gate::new(Vec3::new(0.0, 0.0, z), Vec3::Z, Vec3::Y, [2.0, 40.0, 10.0])
    }

    /// A two-lap race of one checkpoint and a finish line, on the grid.
    fn race() -> Race {
        let course = Course::new(Vec3::ZERO, vec![gate(100.0)], Some(gate(200.0)), 2);
        Race::new(course, RaceConfig::default(), &[(true, Vec3::ZERO), (false, Vec3::ZERO)])
    }

    const NOW: Autopilot = Autopilot { after_secs: 0.0, skill: 0.75 };

    #[test]
    fn the_autopilot_waits_for_the_green_light() {
        let mut engine = race();
        assert!(pilot_aim(&Stage::Running, Some(&engine), None, NOW).is_none(), "on the grid");
        engine.begin_countdown();
        engine.tick(1.0);
        assert!(pilot_aim(&Stage::Running, Some(&engine), None, NOW).is_none(), "counting down");
        let mut engine = race();
        engine.start_now();
        let aim = pilot_aim(&Stage::Running, Some(&engine), None, NOW).expect("the race is on");
        assert_eq!(aim.skill, 0.75);
        assert_eq!(aim.leg, Some(0));
        assert_eq!(aim.goal, next_aim(engine.course(), 0), "the opponents' aim: past the first gate");
    }

    #[test]
    fn a_delay_holds_the_autopilot_back_in_race_time() {
        let late = Autopilot { after_secs: 2.0, ..NOW };
        let mut engine = race();
        engine.start_now();
        engine.tick(1.5);
        assert!(pilot_aim(&Stage::Running, Some(&engine), None, late).is_none());
        engine.tick(0.5);
        assert!(pilot_aim(&Stage::Running, Some(&engine), None, late).is_some());
    }

    #[test]
    fn the_aim_follows_the_players_progress_and_ends_with_the_finish() {
        let mut engine = race();
        engine.start_now();
        engine.move_racer(0, Vec3::new(0.0, 0.0, 99.0));
        engine.move_racer(0, Vec3::new(0.0, 0.0, 101.0));
        let aim = pilot_aim(&Stage::Running, Some(&engine), None, NOW).unwrap();
        assert_eq!((aim.leg, aim.goal), (Some(1), next_aim(engine.course(), 1)), "on to the finish line");
        engine.move_racer(0, Vec3::new(0.0, 0.0, 201.0));
        assert_eq!(pilot_aim(&Stage::Running, Some(&engine), None, NOW).unwrap().leg, Some(2), "the second lap");
        engine.move_racer(0, Vec3::new(0.0, 0.0, 99.0));
        engine.move_racer(0, Vec3::new(0.0, 0.0, 101.0));
        engine.move_racer(0, Vec3::new(0.0, 0.0, 201.0));
        assert!(pilot_aim(&Stage::Running, Some(&engine), None, NOW).is_none(), "the player has finished");
    }

    #[test]
    fn the_autopilot_has_no_car_outside_the_running_stage() {
        let mut engine = race();
        engine.start_now();
        for stage in [Stage::Loading { waited: 0.0 }, Stage::Results { left: 1.0 }, Stage::PostRace { left: 1.0 }] {
            assert!(pilot_aim(&stage, Some(&engine), None, NOW).is_none(), "{}", stage.name());
        }
        assert!(pilot_aim(&Stage::Running, None, None, NOW).is_none(), "no race yet");
    }
}
