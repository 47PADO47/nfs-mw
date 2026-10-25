//! Periodic trace of a scripted run, timestamped in actual simulated seconds.

use super::CarPose;
use super::input::DriveInput;
use super::sim::Telemetry;
use crate::scenes::world::effects::TireEffects;
use crate::scenes::world::exhaust::ExhaustFlames;
use crate::scenes::world::vehicle_effects::VehicleEffects;

impl super::Drive {
    /// Opt-in frame telemetry for movement and streaming regressions.
    pub fn trace_frame(&self, dt: f32, ticks: u32, tick_sim_dt: f32, loaded: bool, complete: bool) {
        if !log::log_enabled!(target: "nfsmw::movement", log::Level::Trace) || self.sim.is_none() {
            return;
        }
        log::trace!(target: "nfsmw::movement",
            "frame {{\"dt\":{dt:?},\"ticks\":{ticks},\"sim_dt\":{tick_sim_dt:?},\"speedbreaker\":{},\"step\":{},\"alpha\":{:?},\"loaded\":{loaded},\"complete\":{complete},\"waiting\":{},\"speed\":{:?},\"rpm\":{:?},\"wheels\":{},\"finite\":{},\"car\":{:?},\"draw\":{:?},\"camera\":{:?},\"camera_previous\":{:?},\"camera_current\":{:?}}}",
            self.breaker.active(),
            self.steps, self.alpha, self.request.is_some(), self.telemetry.speed_mps,
            self.telemetry.rpm, self.telemetry.wheels_on_ground, self.sim.as_ref().is_some_and(|s| s.is_finite()),
            self.current.position.to_array(), self.pose().position.to_array(),
            self.camera().position.to_array(), self.chase.sample(0.0).position.to_array(),
            self.chase.view().position.to_array(),
        );
    }
}

/// Log the effects and car at the script's actual simulation time.
pub(super) fn log(
    simulation_seconds: f32,
    effects: (&TireEffects, &VehicleEffects, &ExhaustFlames),
    car: (&CarPose, &Telemetry),
    input: &DriveInput,
) {
    log::info!("tire effects: {}", effects.0.status());
    log::info!("vehicle effects: {}", effects.1.status());
    log::info!("{}", effects.2.status());
    let (p, t) = (car.0.position, car.1);
    log::info!(
        "t={:>5.1}s  {:>6.1} km/h  {:>5.0} rpm  gear {}  at ({:.1}, {:.1}, {:.2})  {} on ground  throttle {:.1} brake {:.1} steer {:+.2}",
        simulation_seconds,
        t.speed_mps * 3.6,
        t.rpm,
        t.gear,
        p.x,
        p.y,
        p.z,
        t.wheels_on_ground,
        input.throttle,
        input.brake,
        input.steer
    );
}
