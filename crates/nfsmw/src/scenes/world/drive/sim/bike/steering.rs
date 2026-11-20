//! Which steering the bikes loaded now get (`bike_steering`): this game's arcade steering or the reference game's
//! (`docs/specs/ride6-arcade-steering.md`).

use std::sync::atomic::{AtomicBool, Ordering};

/// Whether the next bike loaded keeps the reference game's steering (`bike_steering = reference`) and not this game's arcade steering.
static REFERENCE_STEERING: AtomicBool = AtomicBool::new(false);

/// Tell the bikes which steering the player picked (`bike_steering`); the host pushes it with the other settings. It is part of the bike's
/// tyres and controller, so it reaches the bike that is loaded next, not one on the road.
pub fn set_bike_steering(steering: crate::settings::BikeSteering) {
    REFERENCE_STEERING.store(steering == crate::settings::BikeSteering::Reference, Ordering::Relaxed);
}

/// The arcade steering a bike loaded now gets.
pub(super) fn arcade_steering() -> blackbox_vehicle::bike::ArcadeSteering {
    match REFERENCE_STEERING.load(Ordering::Relaxed) {
        true => crate::settings::BikeSteering::Reference.steering(),
        false => crate::settings::BikeSteering::Arcade.steering(),
    }
}
