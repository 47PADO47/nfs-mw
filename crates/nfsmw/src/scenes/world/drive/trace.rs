//! The once-a-second trace of a scripted drive.

use super::{CarPose, DriveInput, sim::Telemetry};
use crate::scenes::world::effects::TireEffects;

impl super::Drive {
    /// Opt-in frame telemetry for movement and streaming regressions.
    pub fn trace_frame(&self, dt: f32, ticks: u32, loaded: bool, complete: bool) {
        if !log::log_enabled!(target: "nfsmw::movement", log::Level::Trace) || self.sim.is_none() {
            return;
        }
        log::trace!(target: "nfsmw::movement",
            "frame {{\"dt\":{dt:?},\"ticks\":{ticks},\"step\":{},\"alpha\":{:?},\"loaded\":{loaded},\"complete\":{complete},\"waiting\":{},\"speed\":{:?},\"car\":{:?},\"draw\":{:?},\"camera\":{:?},\"camera_previous\":{:?},\"camera_current\":{:?}}}",
            self.steps, self.alpha, self.request.is_some(), self.telemetry.speed_mps,
            self.current.position.to_array(), self.pose().position.to_array(),
            self.camera().position.to_array(), self.chase.sample(0.0).position.to_array(),
            self.chase.view().position.to_array(),
        );
    }
}

/// Log the effects and the car after `steps` physics steps of a scripted run.
pub(super) fn log(steps: u32, effects: &TireEffects, car: (&CarPose, &Telemetry), input: &DriveInput) {
    log::info!("tire effects: {}", effects.status());
    let (p, t) = car;
    log::info!(
        "t={:>5.1}s  {:>6.1} km/h  {:>5.0} rpm  gear {}  at ({:.1}, {:.1}, {:.2})  {} on ground  throttle {:.1} brake {:.1} steer {:+.2}",
        steps as f32 / 60.0,
        t.speed_mps * 3.6,
        t.rpm,
        t.gear,
        p.position.x,
        p.position.y,
        p.position.z,
        t.wheels_on_ground,
        input.throttle,
        input.brake,
        input.steer
    );
}
