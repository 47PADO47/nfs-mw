//! [`PredictedCar`] for the toy world's car, so prediction and reconciliation run in tests and bots without the game.

use blackbox_vehicle::InputState;
use nfsmw_sim::toy::{Car, Controls};

use super::{CarState, PredictedCar};

/// A toy car predicted on its own, on a plane of the same bounds as the server's [`nfsmw_sim::toy::ToyWorld`].
///
/// Stepped with the same input it follows the world's player car bit for bit, because the world steps its players
/// with the same [`Car::step`] before any contact.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToyCar {
    pub car: Car,
    /// Half the side of the world plane, metres (`ToyConfig::bounds`).
    pub bounds: f32,
}

impl ToyCar {
    pub fn new(car: Car, bounds: f32) -> Self {
        Self { car, bounds }
    }
}

impl PredictedCar for ToyCar {
    type Saved = Car;

    fn step(&mut self, input: &InputState) {
        self.car.step(Controls::from_input(input), self.bounds);
    }

    fn state(&self) -> CarState {
        let v = self.car.velocity();
        CarState {
            position: [self.car.pos[0], 0.0, self.car.pos[1]],
            orientation: self.car.orientation(),
            linear_velocity: [v[0], 0.0, v[1]],
            angular_velocity: [0.0, self.car.yaw_rate, 0.0],
            rpm: self.car.rpm(),
            gear: self.car.gear(),
            nitrous: self.car.nitrous,
            boost: self.car.boost,
        }
    }

    /// The toy car's heading is the forward vector of the orientation (+z turned about the up axis) and its speed the
    /// velocity along it. Rpm and gear follow from the speed and are not stored. A heading or velocity equal to the
    /// car's own is left untouched, so `set_state(&state())` changes no bit.
    fn set_state(&mut self, state: &CarState) {
        if state.orientation != self.car.orientation() {
            self.set_heading(state.orientation);
        }
        if state.position.iter().all(|c| c.is_finite()) {
            self.car.pos = [state.position[0], state.position[2]];
        }
        let v = self.car.velocity();
        let speed = state.linear_velocity[0] * self.car.dir[0] + state.linear_velocity[2] * self.car.dir[1];
        if state.linear_velocity != [v[0], 0.0, v[1]] && speed.is_finite() {
            self.car.speed = speed;
        }
        if state.angular_velocity[1].is_finite() {
            self.car.yaw_rate = state.angular_velocity[1];
        }
        self.car.nitrous = finite_unit(state.nitrous, self.car.nitrous);
        self.car.boost = finite_unit(state.boost, self.car.boost);
    }

    fn save(&self) -> Car {
        self.car
    }

    fn restore(&mut self, saved: &Car) {
        self.car = *saved;
    }
}

impl ToyCar {
    /// Point the car along `orientation`: +z turned by the quaternion, projected on the ground plane.
    fn set_heading(&mut self, orientation: [f32; 4]) {
        let [x, y, z, w] = crate::geom::normalize(orientation);
        let forward = [2.0 * (x * z + w * y), 1.0 - 2.0 * (x * x + y * y)];
        let len = (forward[0] * forward[0] + forward[1] * forward[1]).sqrt();
        if !len.is_finite() || len < 1.0e-6 {
            return;
        }
        self.car.dir = [forward[0] / len, forward[1] / len];
    }
}

fn finite_unit(x: f32, fallback: f32) -> f32 {
    match x.is_finite() {
        true => x.clamp(0.0, 1.0),
        false => fallback,
    }
}
