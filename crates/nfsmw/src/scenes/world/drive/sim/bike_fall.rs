//! The rider leaving a bike in a fall, as the game's presentation asks for it (`docs/specs/ride6-rider-fall.md`).
//!
//! The dynamics hand the rider over once, at the fall ([`RiderRelease`], in physics space); the rider's body (`drive/rider/fall.rs`)
//! takes him from there, and is told when the bike has been stood up again. A car never has either.

use blackbox_vehicle::bike::RiderRelease;

use super::CarSim;

impl CarSim {
    /// The rider as the bike let him go in the fall that just happened, once; `None` for a car and when nobody fell.
    pub fn take_rider_release(&mut self) -> Option<RiderRelease> {
        self.ride.as_deref_mut()?.bike.take_rider_release()
    }

    /// Whether the rider is off the bike (from the fall until the bike is stood up again); false for a car.
    pub fn rider_off(&self) -> bool {
        self.ride.as_ref().is_some_and(|ride| ride.bike.rider_off())
    }
}

#[cfg(test)]
#[path = "bike_fall_tests.rs"]
mod tests;
