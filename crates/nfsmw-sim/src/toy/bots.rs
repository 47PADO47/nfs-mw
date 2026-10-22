//! Bot drivers: traffic wanders between random points, police chase the nearest player car.
//!
//! A bot builds the same [`Controls`] a player does and drives the same [`Car`], so there is one physics. The only state a bot
//! owns is its car, its current target and a timer that picks a new target when it is stuck.

use super::car::{Car, Controls};
use super::rng::ToyRng;
use crate::hash::StateHasher;

/// A bot picks a new wander target at least this often (ticks), so one that is boxed in does not sit forever.
const RETARGET_TICKS: u16 = 600;
const ARRIVE_RADIUS: f32 = 10.0;

/// A police car brakes this close to its quarry instead of driving through it.
const RAM_RANGE: f32 = 5.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bot {
    pub car: Car,
    pub target: [f32; 2],
    /// Ticks until the target is replaced whether or not it was reached.
    pub retarget: u16,
}

impl Bot {
    pub fn new(car: Car) -> Self {
        Self { car, target: car.pos, retarget: 0 }
    }

    /// Controls for a car that wanders: cruise at 70 % of top speed towards a random point inside `bounds`.
    pub fn wander(&mut self, rng: &mut ToyRng, bounds: f32) -> Controls {
        self.retarget = self.retarget.saturating_sub(1);
        let reached = distance_sq(self.car.pos, self.target) < ARRIVE_RADIUS * ARRIVE_RADIUS;
        if reached || self.retarget == 0 {
            let reach = bounds * 0.8;
            self.target = [rng.range(-reach, reach), rng.range(-reach, reach)];
            self.retarget = RETARGET_TICKS;
        }
        cruise(&self.car, self.target, self.car.top_speed * 0.7)
    }

    /// Controls for a police car: flat out towards `quarry`, braking when close; without one, patrol like traffic.
    pub fn chase(&mut self, quarry: Option<[f32; 2]>, rng: &mut ToyRng, bounds: f32) -> Controls {
        let Some(quarry) = quarry else {
            return self.wander(rng, bounds);
        };
        self.target = quarry;
        self.retarget = RETARGET_TICKS;
        if distance_sq(self.car.pos, quarry) < RAM_RANGE * RAM_RANGE {
            return Controls { brake: 0.5, steer: steer_toward(&self.car, quarry), ..Controls::default() };
        }
        cruise(&self.car, quarry, self.car.top_speed)
    }

    pub fn hash_into(&self, h: &mut StateHasher) {
        self.car.hash_into(h);
        h.write_f32_slice(&self.target);
        h.write_u16(self.retarget);
    }
}

/// Squared distance on the ground plane.
pub fn distance_sq(a: [f32; 2], b: [f32; 2]) -> f32 {
    let dx = a[0] - b[0];
    let dz = a[1] - b[1];
    dx * dx + dz * dz
}

/// The steering (-1..1, + = right) that turns `car` towards `target`. Zero when the target is on top of the car.
pub fn steer_toward(car: &Car, target: [f32; 2]) -> f32 {
    let dx = target[0] - car.pos[0];
    let dz = target[1] - car.pos[1];
    let len = (dx * dx + dz * dz).sqrt();
    if len < 1.0e-3 {
        return 0.0;
    }
    let ahead = (car.dir[0] * dx + car.dir[1] * dz) / len;
    // Positive when the target lies to the left (the toy world turns left towards +x from +z).
    let left = (car.dir[1] * dx - car.dir[0] * dz) / len;
    if ahead < 0.0 {
        // Behind: turn the long way, towards whichever side it is on, hard.
        return match left > 0.0 {
            true => -1.0,
            false => 1.0,
        };
    }
    (-left * 3.0).clamp(-1.0, 1.0)
}

/// Controls that drive `car` towards `target` at about `speed`.
pub fn cruise(car: &Car, target: [f32; 2], speed: f32) -> Controls {
    let steer = steer_toward(car, target);
    let mut throttle = 0.0;
    let mut brake = 0.0;
    if car.speed < speed {
        throttle = 1.0;
    }
    if car.speed > speed + 3.0 {
        brake = 0.3;
    }
    Controls { throttle, brake, steer, ..Controls::default() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn car_at(pos: [f32; 2], dir: [f32; 2]) -> Car {
        Car::new(pos, dir, 1.0, 20.0)
    }

    #[test]
    fn steer_points_at_the_target() {
        let car = car_at([0.0, 0.0], [0.0, 1.0]);
        assert!(steer_toward(&car, [-10.0, 10.0]) > 0.0, "right of +z is -x");
        assert!(steer_toward(&car, [10.0, 10.0]) < 0.0);
        assert_eq!(steer_toward(&car, [0.0, 10.0]), 0.0);
        assert_eq!(steer_toward(&car, [0.0, 0.0]), 0.0);
        assert_eq!(steer_toward(&car, [0.0, -10.0]).abs(), 1.0);
    }

    #[test]
    fn wander_picks_a_target_inside_the_bounds() {
        let mut rng = ToyRng::new(3);
        let mut bot = Bot::new(car_at([0.0, 0.0], [0.0, 1.0]));
        for _ in 0..50 {
            bot.retarget = 0;
            bot.wander(&mut rng, 100.0);
            assert!(bot.target[0].abs() <= 80.0 && bot.target[1].abs() <= 80.0);
        }
    }

    #[test]
    fn police_brake_next_to_the_quarry_and_patrol_without_one() {
        let mut rng = ToyRng::new(3);
        let mut cop = Bot::new(car_at([0.0, 0.0], [0.0, 1.0]));
        let near = cop.chase(Some([0.0, 3.0]), &mut rng, 100.0);
        assert!(near.brake > 0.0 && near.throttle == 0.0);
        let far = cop.chase(Some([0.0, 80.0]), &mut rng, 100.0);
        assert_eq!(far.throttle, 1.0);
        let before = rng.state();
        cop.retarget = 0;
        cop.chase(None, &mut rng, 100.0);
        assert_ne!(rng.state(), before, "patrol draws a new target");
    }
}
