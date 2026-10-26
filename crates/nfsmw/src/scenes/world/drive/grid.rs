//! The race grid: before GO the player's body is held while its pedals remain under driver control.

use super::{Drive, DriveInput, SpawnRequest};
use crate::scenes::world::road::Spawn;

/// Below this speed (m/s) the assist leaves the steering to the player.
const ASSIST_MIN_SPEED: f32 = 1.0;

impl Drive {
    /// Holds the car on the grid (`true`) or releases it (`false`). A held car keeps the player's throttle and steering,
    /// so the engine can be revved. Body holding prevents departure independently of pedal input.
    pub fn hold_on_grid(&mut self, held: bool) {
        self.gridded = held;
        if let Some(sim) = self.sim.as_mut() {
            sim.set_race_staging(held);
        }
    }

    /// Selects the race's launch rules before placing the car on its grid.
    pub fn set_launch_mode(&mut self, drag: bool) {
        self.launch_cancel_on_shift = drag;
        if let Some(sim) = self.sim.as_mut() {
            sim.configure_player_launch(drag);
        }
    }

    /// Only GO awards a launch. Other grid releases (abort, results) do not.
    pub fn launch_from_grid(&mut self) -> bool {
        if !self.gridded {
            return false;
        }
        self.gridded = false;
        self.sim.as_mut().is_some_and(|sim| sim.launch_from_grid())
    }

    /// Sets the point the car steers at by itself (physics space), or clears it. While it is set, the car's steering
    /// follows the point above 1 m/s and the player's own steering is ignored (`docs/specs/race-mode.md` section 8).
    pub fn set_steer_assist(&mut self, aim: Option<glam::Vec3>) {
        self.assist_aim = aim;
    }

    /// The steering the player asks for now, -1 to 1 (positive to the right), whatever the assist does with it.
    pub fn steer_request(&self) -> f32 {
        self.frame_input.steer
    }

    /// The input with the assist's steering, when it is on and the car is moving.
    pub(super) fn assisted(aim: Option<glam::Vec3>, sim: &super::sim::CarSim, input: DriveInput) -> DriveInput {
        let Some(aim) = aim else { return input };
        let view = sim.driver_view();
        if view.forward_speed.abs() < ASSIST_MIN_SPEED {
            return input;
        }
        let (steer, _) = blackbox_driver::simple_steering(view.position, view.forward, aim, view.max_steer, false);
        DriveInput { steer, ..input }
    }

    /// Asks for the car to be put exactly at `spawn` (the race's start), rolling at `speed` m/s (0 for a standing
    /// start). The placement happens when the world around it has loaded, as for any respawn.
    pub fn place_exactly(&mut self, spawn: Spawn, speed: f32) {
        self.pending_speed = speed;
        self.tail_trails.clear();
        self.effects.disconnect();
        self.vehicle_effects.clear();
        self.request = Some(SpawnRequest {
            near: [spawn.position.x, spawn.position.y],
            heading: Some(spawn.heading),
            exact: Some(spawn),
            strict: true,
        });
    }
}

impl DriveInput {
    /// The input of a grid car: driver controls stay; nitrous and Speedbreaker are unavailable.
    pub(super) fn on_grid(self, held: bool) -> Self {
        if !held {
            return self;
        }
        Self { nos: false, speedbreaker: false, ..self }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_held_car_keeps_the_driver_pedals_and_steering() {
        let input = DriveInput { throttle: 1.0, brake: 0.3, steer: 0.5, nos: true, ..Default::default() };
        let held = input.on_grid(true);
        assert_eq!((held.throttle, held.steer, held.brake, held.handbrake, held.nos), (1.0, 0.5, 0.3, false, false));
        assert_eq!(input.on_grid(false), input);
    }

    #[test]
    #[ignore = "requires installed player geometry/physics; CPU-only race mode lifecycle"]
    fn installed_leaving_drag_restores_normal_gauge_and_launch_context() {
        use crate::app::test_backend::NullBackend;
        use crate::scenes::world::drive::{rig::CarRig, sim::CarSim};
        use crate::settings::CarShading;
        use nfsmw_data::car::{LoadOptions, physics::PhysicsData};

        let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
        let data = PhysicsData::load(&dir).unwrap();
        let physics = data.car("elise").unwrap();
        let model =
            nfsmw_data::car::load_driving(&dir, "ELISE", &LoadOptions { lod: 'A', all_parts: false, preset: None })
                .unwrap();
        let mut renderer = NullBackend::new(crate::settings::test_caps::native());
        let rig = CarRig::upload(&mut renderer, 123, model, CarShading::Original, glam::Vec3::Z);
        let sim = CarSim::new(physics.clone(), [0.2; 4]);
        let mut drive = Drive::new(
            "ELISE".into(),
            rig,
            physics,
            SpawnRequest { near: [0.0; 2], heading: None, exact: None, strict: false },
            None,
            Default::default(),
        );
        drive.sim = Some(sim);
        drive.telemetry = drive.sim.as_ref().unwrap().telemetry();
        drive.gauges.snap(super::super::gauges::GaugeSample { rpm: 1200.0, speed: 0.0 });
        drive.note_visual_rpm(Some(0.75));
        drive.set_launch_mode(true);
        assert_eq!(drive.hud_state().unwrap().rpm, 1200.0);
        drive.hold_on_grid(true);
        assert!(drive.sim.as_ref().unwrap().staged_launch_band().is_some());
        drive.hold_on_grid(false);
        drive.set_launch_mode(false);
        assert!(!drive.launch_cancel_on_shift);
        assert!(drive.sim.as_ref().unwrap().staged_launch_band().is_none());
        assert!(!drive.launch_from_grid(), "teardown cannot award a launch");
        let expected = drive.telemetry.idle + 0.75 * (drive.telemetry.red_line - drive.telemetry.idle);
        assert_eq!(drive.hud_state().unwrap().rpm, expected, "free roam returns to audio visual RPM");
        drive.rig.release(&mut renderer);
    }
}
