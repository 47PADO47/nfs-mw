//! A hinged part that moves with the car's speed: an active spoiler.
//!
//! The part slides by an offset when the car is at its deploy speed or faster and slides back below its retract speed
//! (between the two it keeps what it was doing), taking a set time for the whole way. The deployment is a number from 0
//! to 1 that the car keeps ([`Deployment`]) and that reaches the car draw as a mark in the set of revealed slots, as the
//! zone levels do ([`deploy_mark`]).

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
    pub above_mph: f32,
    pub below_mph: f32,
    /// Seconds the whole movement takes.
    pub seconds: f32,
}

/// How far a part is deployed, and which way it is going.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Deployment {
    /// 0 (at rest) to 1 (fully deployed).
    pub amount: f32,
    up: bool,
}

impl Deployment {
    /// Advance by `dt` seconds with the car at `mph`: it starts going up at the deploy speed, and down below the retract
    /// speed; at a speed between the two it goes on the way it was going.
    pub fn step(&mut self, active: &Active, mph: f32, dt: f32) {
        if mph >= active.above_mph {
            self.up = true;
        } else if mph < active.below_mph {
            self.up = false;
        }
        let rate = dt.max(0.0) / active.seconds.max(0.01);
        self.amount = match self.up {
            true => (self.amount + rate).min(1.0),
            false => (self.amount - rate).max(0.0),
        };
    }
}

/// The deployment eased: slow at both ends of the movement.
pub fn eased(amount: f32) -> f32 {
    let t = amount.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl Active {
    /// The movement of the part at `amount` of its deployment, in the car's solid space.
    pub fn translation(&self, amount: f32) -> Mat4 {
        match amount > 0.0 {
            true => Mat4::from_translation(Vec3::from_array(self.offset) * eased(amount)),
            false => Mat4::IDENTITY,
        }
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

    const SPOILER: Active = Active { offset: [-0.07, 0.0, 0.02], above_mph: 70.0, below_mph: 40.0, seconds: 0.6 };

    fn run(deployment: &mut Deployment, mph: f32, seconds: f32) {
        for _ in 0..(seconds * 60.0).round() as usize {
            deployment.step(&SPOILER, mph, 1.0 / 60.0);
        }
    }

    #[test]
    fn it_goes_up_at_the_deploy_speed_in_the_time_it_takes_and_stays() {
        let mut d = Deployment::default();
        run(&mut d, 60.0, 2.0);
        assert_eq!(d.amount, 0.0, "below the deploy speed it stays down");
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
    fn the_movement_is_eased_and_exact_at_the_ends() {
        assert_eq!(SPOILER.translation(0.0), Mat4::IDENTITY);
        let full = SPOILER.translation(1.0).transform_point3(Vec3::ZERO);
        assert!((full - Vec3::new(-0.07, 0.0, 0.02)).length() < 1e-6);
        assert!(eased(0.25) < 0.25 && eased(0.75) > 0.75);
        assert_eq!((eased(0.0), eased(1.0), eased(2.0)), (0.0, 1.0, 1.0));
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
