//! The player's car as a multiplayer client predicts it (`scenes/world/net_client/car.rs`,
//! `docs/specs/multiplayer/sync-model.md` section 2): one physics tick from a network input against the static world,
//! and the saved state of the whole car for rewinding. Nothing here runs in single player.

use blackbox_collision::CollisionWorld;
use nfsmw_data::car::physics::SurfaceTable;

use super::sim::Telemetry;
use super::sim::predict::{SimMotion, SimSave};
use super::{CarPose, Drive, DriveInput, STEP};
use crate::input::ActionState;
use crate::scenes::world::props::PropWorld;

/// Everything of the car a later predicted tick depends on: the physics and the poses drawn from it.
#[derive(Debug, Clone)]
pub struct DriveSave {
    sim: SimSave,
    previous: CarPose,
    current: CarPose,
    telemetry: Telemetry,
    gauges: super::gauges::GaugeHistory,
}

impl Drive {
    /// One physics tick with `input` (a network seat's input, as the server's car takes it), against the static world
    /// and the props: other cars are not in a prediction (sync-model.md section 4.2). The tick's effects, sound and road
    /// checks run as in single player. Returns whether the car stepped: not while it waits for a road.
    pub(in crate::scenes::world) fn predict_step(
        &mut self,
        input: DriveInput,
        collision: &CollisionWorld,
        props: &mut PropWorld,
        surfaces: &SurfaceTable,
    ) -> bool {
        self.set_remote_input(Some(input));
        self.prepare_frame(&ActionState::default(), STEP);
        let Some(step) = self.step(collision, props, surfaces) else { return false };
        self.finish_step(collision, surfaces, step);
        true
    }

    /// The car's state for [`Drive::predict_restore`]; `None` before the car stands on a road.
    pub(in crate::scenes::world) fn predict_save(&self) -> Option<DriveSave> {
        let sim = self.sim.as_ref()?;
        Some(DriveSave {
            sim: sim.save(),
            previous: self.previous,
            current: self.current,
            telemetry: self.telemetry,
            gauges: self.gauges,
        })
    }

    pub(in crate::scenes::world) fn predict_restore(&mut self, saved: &DriveSave) {
        let Some(sim) = self.sim.as_mut() else { return };
        sim.restore(&saved.sim);
        self.previous = saved.previous;
        self.current = saved.current;
        self.telemetry = saved.telemetry;
        self.gauges = saved.gauges;
        self.clear_visual_rpm();
    }

    /// The reduced motion of the car (render space), `None` before it stands on a road.
    pub(in crate::scenes::world) fn motion(&self) -> Option<SimMotion> {
        self.sim.as_ref().map(|sim| sim.motion())
    }

    /// Overwrite the car's pose and velocities (a server's correction or its first placement). The drawn pose jumps
    /// with it: the visual smoothing is the session's (`ClientSession::own_pose`).
    pub(in crate::scenes::world) fn set_motion(&mut self, motion: &SimMotion) {
        let Some(sim) = self.sim.as_mut() else { return };
        sim.set_motion(motion);
        self.current = sim.pose();
        self.previous = self.current;
        self.telemetry = sim.telemetry();
        self.gauges.snap(sim.gauge_sample());
        self.clear_visual_rpm();
    }

    /// What the player's devices ask of the car this frame, with the wheel's options: a client sends it as its input.
    pub(in crate::scenes::world) fn device_input(&self, actions: &ActionState) -> DriveInput {
        DriveInput::from_actions(actions, self.wheel)
    }

    /// Draw the car at `pose` instead of its physics pose (the session's smoothed pose), or at the physics pose again
    /// with `None`. A NIS scene's pose still wins.
    pub(in crate::scenes::world) fn set_net_pose(&mut self, pose: Option<CarPose>) {
        self.net_pose = pose;
    }
}
