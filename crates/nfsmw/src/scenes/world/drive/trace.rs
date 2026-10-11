//! The once-a-second trace of a scripted drive.

use super::{CarPose, DriveInput, sim::Telemetry};
use crate::scenes::world::effects::TireEffects;

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
