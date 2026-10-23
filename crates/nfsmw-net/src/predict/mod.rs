//! The owner's prediction of its own car (sync-model.md section 2.1 and 2.2, MP-80).
//!
//! - [`CarState`]: the reduced vehicle state the server sends back in the own-car block (position, orientation,
//!   linear and angular velocity, rpm, gear, nitrous, boost), and [`Pose`], the part that is drawn.
//! - [`PredictedCar`]: what the game implements for its car: step one tick with an [`InputState`], read and write the
//!   reduced state, save and restore everything else. The same prediction then drives the toy car of the tests
//!   ([`ToyCar`]) and the real vehicle.
//! - [`Predictor`]: the car plus a ring of `(tick, input, predicted state)` for the unacknowledged window. Each tick it
//!   quantises the input to wire precision before stepping, so the client predicts with exactly what the server will
//!   receive. [`crate::reconcile`] rewinds and replays it.
//!
//! No clock and no I/O: the caller says which tick it is.

mod predictor;
mod toy;

#[cfg(test)]
mod tests;

pub use predictor::{Entry, MAX_TICKS_PER_FRAME, Predictor, RING_TICKS};
pub use toy::ToyCar;

pub use blackbox_vehicle::InputState;

use crate::geom::{self, IDENTITY, Quat, Vec3};
use crate::schema::OwnBlock;

/// Where a car is and which way it faces.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pose {
    pub position: Vec3,
    /// Unit quaternion `[x, y, z, w]`.
    pub orientation: Quat,
}

impl Default for Pose {
    fn default() -> Self {
        Self { position: [0.0; 3], orientation: IDENTITY }
    }
}

/// The reduced state of a car: what the own-car block carries and what reconciliation compares.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CarState {
    /// World metres.
    pub position: Vec3,
    /// Unit quaternion `[x, y, z, w]`.
    pub orientation: Quat,
    /// Metres per second.
    pub linear_velocity: Vec3,
    /// Radians per second.
    pub angular_velocity: Vec3,
    pub rpm: f32,
    pub gear: u8,
    /// Nitrous left, 0..1.
    pub nitrous: f32,
    /// Speedbreaker meter, 0..1.
    pub boost: f32,
}

impl Default for CarState {
    fn default() -> Self {
        Self {
            position: [0.0; 3],
            orientation: IDENTITY,
            linear_velocity: [0.0; 3],
            angular_velocity: [0.0; 3],
            rpm: 0.0,
            gear: 0,
            nitrous: 0.0,
            boost: 0.0,
        }
    }
}

impl CarState {
    /// The state an own-car block carries.
    pub fn from_own_block(block: &OwnBlock) -> Self {
        Self {
            position: block.position,
            orientation: geom::normalize(block.orientation),
            linear_velocity: block.linear_velocity,
            angular_velocity: block.angular_velocity,
            rpm: block.own.rpm,
            gear: block.own.gear,
            nitrous: block.own.nitrous,
            boost: block.own.boost,
        }
    }

    pub fn pose(&self) -> Pose {
        Pose { position: self.position, orientation: self.orientation }
    }
}

/// A car the client can predict. The game implements it for its vehicle; [`ToyCar`] implements it for the toy world.
///
/// The contract: two cars that were restored from the same [`PredictedCar::Saved`], given the same
/// [`PredictedCar::set_state`] and stepped with the same inputs, end in the same state.
pub trait PredictedCar {
    /// Everything a later step depends on beyond [`CarState`] (tyres, suspension, engine internals), for rewinding.
    type Saved: Clone;

    /// Advance one fixed physics step under `input`. Contacts with the static world are included; contacts with
    /// remote cars, traffic and police are not (sync-model.md section 4.2).
    fn step(&mut self, input: &InputState);

    /// The reduced state now.
    fn state(&self) -> CarState;

    /// Overwrite the reduced fields with `state`, keeping the rest.
    fn set_state(&mut self, state: &CarState);

    /// Everything, for [`PredictedCar::restore`].
    fn save(&self) -> Self::Saved;

    fn restore(&mut self, saved: &Self::Saved);
}
