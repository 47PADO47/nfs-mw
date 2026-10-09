//! The once-a-second trace of a scripted run.

use super::CarPose;
use super::input::DriveInput;
use super::sim::Telemetry;
use crate::scenes::world::effects::TireEffects;
use crate::scenes::world::exhaust::ExhaustFlames;
use crate::scenes::world::vehicle_effects::VehicleEffects;

/// Log the effects and the car after `steps` physics steps of a scripted run.
pub(super) fn log(
    steps: u32,
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
        steps as f32 / 60.0,
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
