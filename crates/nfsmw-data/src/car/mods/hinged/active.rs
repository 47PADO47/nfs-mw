//! A hinged part that moves with the car's speed: an active spoiler.
//!
//! The part turns about its pivot and slides by an offset when the car is above its deploy speed and goes back when the
//! car is below its retract speed (between the two it keeps what it was doing), taking a set time for the whole way, at a
//! constant rate as the game's animation state plays (`docs/formats/chameleon-active-aero.md`). The deployment is a
//! number from 0 to 1 that the car keeps ([`Deployment`]) and that reaches the car draw as a mark in the set of revealed
//! slots, as the zone levels do ([`deploy_mark`]).

use std::collections::BTreeSet;

use glam::{Mat4, Vec3};

/// The first number of the deployment marks, above the zone-level marks (`level_mark`).
const DEPLOY_BASE: usize = 2048;
/// Marks per part, and the steps a deployment is told in (1 to this).
const MARKS_PER_PART: usize = 32;
const STEPS: usize = 16;
/// Metres per second in a mile per hour.
pub const MPS_TO_MPH: f32 = 2.236_936;

/// How a part moves with the car's speed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Active {
    /// The movement at full deployment, in the car's solid space, metres.
    pub offset: [f32; 3],
    /// The turn at full deployment about the part's pivot: unit axis and degrees. `None`: no turn.
    pub rotation: Option<([f32; 3], f32)>,
    pub above_mph: f32,
    pub below_mph: f32,
    /// Seconds the whole movement takes.
    pub seconds: f32,
    /// The gas pedal matters: deploys only with the gas pressed, retracts when it is released.
    pub gas: bool,
}

/// How far a part is deployed, and which way it is going.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Deployment {
    /// 0 (at rest) to 1 (fully deployed).
    pub amount: f32,
    up: bool,
}

impl Deployment {
    /// Advance by `dt` seconds with the car at `mph`: it starts going up above the deploy speed, and down below the
    /// retract speed; at a speed between the two (or exactly at one) it goes on the way it was going.
    pub fn step(&mut self, active: &Active, mph: f32, gas: bool, dt: f32) {
        if mph > active.above_mph && (gas || !active.gas) {
            self.up = true;
        } else if mph < active.below_mph || (active.gas && !gas) {
            self.up = false;
        }
        let rate = dt.max(0.0) / active.seconds.max(0.01);
        self.amount = match self.up {
            true => (self.amount + rate).min(1.0),
            false => (self.amount - rate).max(0.0),
        };
    }
}

impl Active {
    /// The movement of the part at `amount` of its deployment, in the car's solid space: turned about `pivot` by the
    /// share of the rotation, then moved by the share of the offset. Both grow in proportion to `amount`.
    pub fn motion(&self, pivot: Vec3, amount: f32) -> Mat4 {
        let amount = amount.clamp(0.0, 1.0);
        if amount <= 0.0 {
            return Mat4::IDENTITY;
        }
        let slide = Mat4::from_translation(Vec3::from_array(self.offset) * amount);
        let Some((axis, degrees)) = self.rotation else { return slide };
        let turn = Mat4::from_axis_angle(Vec3::from_array(axis), (degrees * amount).to_radians());
        slide * Mat4::from_translation(pivot) * turn * Mat4::from_translation(-pivot)
    }
}

/// The mark that tells the car draw how far part `part` is deployed, `None` when it is at rest.
pub fn deploy_mark(part: usize, amount: f32) -> Option<usize> {
    let step = (amount.clamp(0.0, 1.0) * STEPS as f32).round() as usize;
    (step > 0 && part < 16).then_some(DEPLOY_BASE + part * MARKS_PER_PART + step)
}

/// How far part `part` is deployed, as the marks say it (to a sixteenth).
pub fn deploy_in(marks: &BTreeSet<usize>, part: usize) -> f32 {
    let base = DEPLOY_BASE + part * MARKS_PER_PART;
    marks.range(base + 1..=base + STEPS).next_back().map_or(0.0, |mark| (mark - base) as f32 / STEPS as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPOILER: Active = Active {
        offset: [-0.07, 0.0, 0.02],
        rotation: None,
        above_mph: 70.0,
        below_mph: 40.0,
        seconds: 0.6,
        gas: false,
    };

    fn run(deployment: &mut Deployment, mph: f32, seconds: f32) {
        for _ in 0..(seconds * 60.0).round() as usize {
            deployment.step(&SPOILER, mph, false, 1.0 / 60.0);
        }
    }

    #[test]
    fn it_goes_up_above_the_deploy_speed_in_the_time_it_takes_and_stays() {
        let mut d = Deployment::default();
        run(&mut d, 60.0, 2.0);
        assert_eq!(d.amount, 0.0, "below the deploy speed it stays down");
        run(&mut d, 70.0, 1.0);
        assert_eq!(d.amount, 0.0, "the condition is greater than, not at");
        run(&mut d, 71.0, 0.3);
        assert!((d.amount - 0.5).abs() < 0.03, "half way after half the time: {}", d.amount);
        run(&mut d, 71.0, 1.0);
        assert_eq!(d.amount, 1.0);
    }

    #[test]
    fn between_the_two_speeds_it_goes_on_the_way_it_was_going() {
        let mut d = Deployment::default();
        run(&mut d, 80.0, 1.0);
        run(&mut d, 55.0, 2.0);
        assert_eq!(d.amount, 1.0, "slowing down to 55 mph keeps it up");
        run(&mut d, 39.0, 1.0);
        assert_eq!(d.amount, 0.0, "below 40 mph it comes down");
        run(&mut d, 55.0, 2.0);
        assert_eq!(d.amount, 0.0, "speeding up to 55 mph does not raise it");
    }

    #[test]
    fn a_car_with_one_speed_goes_up_above_it_and_down_below_it() {
        let single = Active { above_mph: 75.0, below_mph: 75.0, ..SPOILER };
        let mut d = Deployment::default();
        d.step(&single, 76.0, false, 0.3);
        assert!(d.amount > 0.4);
        d.step(&single, 74.0, false, 0.3);
        assert_eq!(d.amount, 0.0);
    }

    #[test]
    fn a_gas_part_deploys_above_the_speed_with_the_gas_and_retracts_when_it_is_released() {
        let wing = Active { gas: true, below_mph: 0.0, ..SPOILER };
        let mut d = Deployment::default();
        d.step(&wing, 100.0, false, 0.3);
        assert_eq!(d.amount, 0.0, "fast but off the gas: it stays down");
        d.step(&wing, 100.0, true, 0.3);
        assert!(d.amount > 0.4);
        d.step(&wing, 100.0, false, 0.3);
        assert_eq!(d.amount, 0.0, "off the gas it comes down at any speed");
        d.step(&wing, 60.0, true, 0.3);
        assert_eq!(d.amount, 0.0, "slow: it does not deploy with the gas");
    }

    #[test]
    fn the_movement_is_linear_and_exact_at_the_ends() {
        assert_eq!(SPOILER.motion(Vec3::ZERO, 0.0), Mat4::IDENTITY);
        let full = SPOILER.motion(Vec3::ZERO, 1.0).transform_point3(Vec3::ZERO);
        assert!((full - Vec3::new(-0.07, 0.0, 0.02)).length() < 1e-6);
        let half = SPOILER.motion(Vec3::ZERO, 0.5).transform_point3(Vec3::ZERO);
        assert!((half - Vec3::new(-0.035, 0.0, 0.01)).length() < 1e-6, "no easing: {half}");
    }

    #[test]
    fn a_turn_is_about_the_pivot_in_proportion_to_the_deployment() {
        let flap = Active { offset: [0.0; 3], rotation: Some(([0.0, 1.0, 0.0], 30.0)), ..SPOILER };
        let pivot = Vec3::new(1.0, 0.0, 0.5);
        assert!((flap.motion(pivot, 1.0).transform_point3(pivot) - pivot).length() < 1e-6, "the pivot stays");
        let tip = pivot + Vec3::X;
        let at = |amount: f32, degrees: f32| {
            let moved = flap.motion(pivot, amount).transform_point3(tip);
            // A right-handed turn about +y takes +x down, towards -z.
            let wanted = pivot + Vec3::new(degrees.to_radians().cos(), 0.0, -degrees.to_radians().sin());
            (moved - wanted).length()
        };
        assert!(at(1.0, 30.0) < 1e-5);
        assert!(at(0.5, 15.0) < 1e-5);
        let both = Active { offset: [0.0, 0.0, 0.1], ..flap };
        assert!((both.motion(pivot, 1.0).transform_point3(pivot) - (pivot + Vec3::Z * 0.1)).length() < 1e-6);
    }

    #[test]
    fn the_marks_carry_the_deployment_to_a_sixteenth() {
        assert_eq!(deploy_mark(0, 0.0), None);
        assert_eq!(deploy_mark(16, 1.0), None, "only the first sixteen parts");
        let marks: BTreeSet<usize> =
            [deploy_mark(0, 0.5), deploy_mark(1, 1.0), deploy_mark(2, 0.01)].into_iter().flatten().collect();
        assert_eq!(deploy_in(&marks, 0), 0.5);
        assert_eq!(deploy_in(&marks, 1), 1.0);
        assert_eq!(deploy_in(&marks, 2), 0.0, "less than a sixteenth is at rest");
        assert_eq!(deploy_in(&marks, 3), 0.0);
        assert!(marks.iter().all(|&mark| mark > 1024 + 8 * 8 + 3), "clear of the zone-level marks");
    }
}
