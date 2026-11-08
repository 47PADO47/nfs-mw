//! What the rider hands the bike besides the levers: his weight (back to forward and side to side), the tuck behind the screen and
//! the wake of a vehicle ahead, and where his body sits because of them.

use glam::Vec3;

use super::{Bike, RiderRotation, ThrottleWheelie};

impl Bike {
    /// The rider's weight on the bike, -1 (fully back) to 1 (fully forward). It moves the combined centre of gravity and
    /// asks the wheelie and stoppie control for a pitch.
    pub fn set_weight(&mut self, weight: f32) {
        self.weight_input = match weight.is_finite() {
            true => weight.clamp(-1.0, 1.0),
            false => 0.0,
        };
    }

    /// Switch the game's wheelie from the throttle alone: [`ThrottleWheelie::GAME`] or [`ThrottleWheelie::REFERENCE`] (the default), which
    /// is the reference's behaviour (the weight lever alone lifts a wheel).
    pub fn set_throttle_wheelie(&mut self, config: ThrottleWheelie) {
        self.throttle_wheelie = config;
    }

    /// The throttle-only wheelie's settings now.
    pub fn throttle_wheelie(&self) -> &ThrottleWheelie {
        &self.throttle_wheelie
    }

    /// Whether the throttle is holding the weight lever back this step (a wheelie it asked for), as opposed to the rider's own lever.
    pub fn throttle_lever(&self) -> bool {
        self.trick.throttle_lever
    }

    /// The rider's weight now (-1 back to 1 forward): the weight the rider help aims at after the rate limit of the input
    /// channel. It differs from the one set with [`Bike::set_weight`] while the auto-rider has the weight.
    pub fn rider_weight(&self) -> f32 {
        self.weight.value()
    }

    /// The rider's lateral weight, -1 (to the left) to 1 (to the right): with the lean it asks for the hang-off (the lateral
    /// shift of his place, `RiderSpec::weight_influence`).
    pub fn set_weight_lateral(&mut self, weight: f32) {
        self.weight_lateral_input = match weight.is_finite() {
            true => weight.clamp(-1.0, 1.0),
            false => 0.0,
        };
    }

    /// How far the rider tucks in behind the screen, 0 (upright) to 1: it scales the air drag by the rider's drag curve.
    pub fn set_tuck(&mut self, tuck: f32) {
        self.tuck = match tuck.is_finite() {
            true => tuck.clamp(0.0, 1.0),
            false => 0.0,
        };
    }

    /// Where the rider's centre of mass is from his rest place on the bike, in the body frame (m; x right, y up, z forward): the
    /// place the weight, the lean and the speed ask for plus how far his spring has let him move.
    pub fn rider_offset(&self) -> Vec3 {
        self.placement + self.rider_body.deflection
    }

    /// The rider's body against the bike's attitude (see [`RiderRotation`]).
    pub fn rider_rotation(&self) -> &RiderRotation {
        &self.rider_rotation
    }

    /// The wake of a vehicle ahead, 0 to 1: the share of the air drag it takes away (see [`super::Slipstream`]). 0 on an empty road.
    pub fn set_slipstream(&mut self, share: f32) {
        self.slipstream = match share.is_finite() {
            true => share.clamp(0.0, 1.0),
            false => 0.0,
        };
    }
}
