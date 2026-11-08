//! A point that follows a moving target as a critically damped spring does (`docs/specs/ride6-feel-round14.md`, section 1).
//!
//! The thrown rider's pelvis is the camera's target while he is down (`rider_follow.rs`). A ragdoll's pelvis is not a calm point: its
//! capsule turns and bounces on the road, and the camera that took it as it came shook with it (a measured 49 m/s^2 of mean noise in the
//! followed point, 360 at worst, where a body falling freely accelerates at 9.8). The spring here removes what is too quick for the eye
//! to follow, loses almost nothing of the path (the lag behind a steady speed is `2 v / omega`) and has no overshoot, and its
//! acceleration is capped.

use glam::Vec3;

/// A point and its velocity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Damped {
    position: Vec3,
    velocity: Vec3,
}

impl Damped {
    /// A point at rest at `position`.
    pub fn at(position: Vec3) -> Self {
        Self { position, velocity: Vec3::ZERO }
    }

    pub fn position(&self) -> Vec3 {
        self.position
    }

    /// The velocity of the point (m/s): the smoothed velocity of what it follows.
    pub fn velocity(&self) -> Vec3 {
        self.velocity
    }

    /// Follow `target` for `dt` seconds: the exact solution of `x'' = omega^2 (target - x) - 2 omega x'` for a target that holds still
    /// during the step, so any step is stable. The acceleration is held to `max_accel` (m/s^2). A step of no time or a target that is
    /// not a number changes nothing.
    pub fn step(&mut self, target: Vec3, omega: f32, max_accel: f32, dt: f32) {
        if !(dt > 0.0 && dt.is_finite() && omega > 0.0 && target.is_finite()) {
            return;
        }
        let gap = self.position - target;
        let decay = (-omega * dt).exp();
        let carried = self.velocity + gap * omega;
        let mut position = target + (gap + carried * dt) * decay;
        let mut velocity = (self.velocity - carried * (omega * dt)) * decay;
        let change = velocity - self.velocity;
        let limit = max_accel.max(0.0) * dt;
        if change.length() > limit {
            velocity = self.velocity + change.normalize_or_zero() * limit;
            position = self.position + (self.velocity + velocity) * (0.5 * dt);
        }
        self.position = position;
        self.velocity = velocity;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 60.0;
    const OMEGA: f32 = 35.0;
    const NO_CAP: f32 = 1.0e9;

    #[test]
    fn a_step_in_the_target_is_reached_without_overshoot() {
        let mut point = Damped::at(Vec3::ZERO);
        let target = Vec3::new(10.0, 0.0, 0.0);
        let mut farthest = 0.0_f32;
        for _ in 0..120 {
            point.step(target, OMEGA, NO_CAP, DT);
            farthest = farthest.max(point.position().x);
        }
        assert!((point.position().x - 10.0).abs() < 1e-3, "{}", point.position());
        assert!(farthest <= 10.0 + 1e-4, "no overshoot: {farthest}");
        assert!(point.velocity().length() < 1e-2);
    }

    #[test]
    fn a_steady_speed_lags_by_two_speeds_over_omega() {
        let mut point = Damped::at(Vec3::ZERO);
        let mut target = Vec3::ZERO;
        for _ in 0..240 {
            target.x += 20.0 * DT;
            point.step(target, OMEGA, NO_CAP, DT);
        }
        let lag = target.x - point.position().x;
        assert!((lag - 2.0 * 20.0 / OMEGA).abs() < 0.25, "{lag} m behind at 20 m/s");
        assert!((point.velocity().x - 20.0).abs() < 1.0, "{}", point.velocity());
    }

    #[test]
    fn the_jitter_of_a_ragdoll_is_mostly_taken_out() {
        // A pelvis sliding along at 6 m/s with 6 cm of alternating noise in each axis, as the capsule turns on the road.
        let mut point = Damped::at(Vec3::new(0.0, 0.0, 0.3));
        let (mut raw, mut smooth) = (Vec::new(), Vec::new());
        for i in 0..300 {
            let noise = if i % 2 == 0 { 0.06 } else { -0.06 };
            let target = Vec3::new(6.0 * i as f32 * DT + noise, noise, 0.3 - noise);
            point.step(target, OMEGA, NO_CAP, DT);
            raw.push(target);
            smooth.push(point.position());
        }
        let accel = |path: &[Vec3]| {
            let sum: f32 = path.windows(3).skip(60).map(|w| (w[2] - 2.0 * w[1] + w[0]).length() / (DT * DT)).sum();
            sum / (path.len() - 62) as f32
        };
        let (before, after) = (accel(&raw), accel(&smooth));
        assert!(before > 400.0, "the raw point shakes: {before}");
        assert!(after < before / 8.0, "the smoothed one hardly does: {after} against {before}");
    }

    #[test]
    fn the_acceleration_is_capped() {
        let mut point = Damped::at(Vec3::ZERO);
        let mut last = point.velocity();
        let mut worst = 0.0_f32;
        for _ in 0..60 {
            point.step(Vec3::new(50.0, 0.0, 0.0), OMEGA, 300.0, DT);
            worst = worst.max((point.velocity() - last).length() / DT);
            last = point.velocity();
        }
        assert!(worst <= 300.0 + 1e-2, "{worst}");
        assert!(point.position().x > 1.0, "it still goes: {}", point.position());
    }

    #[test]
    fn a_slow_frame_is_as_stable_as_a_fast_one() {
        for dt in [1.0 / 240.0, 1.0 / 60.0, 0.25, 2.0] {
            let mut point = Damped::at(Vec3::ZERO);
            for _ in 0..((3.0 / dt) as usize).max(1) {
                point.step(Vec3::new(4.0, 2.0, 1.0), OMEGA, NO_CAP, dt);
            }
            assert!((point.position() - Vec3::new(4.0, 2.0, 1.0)).length() < 1e-2, "dt {dt}: {}", point.position());
        }
    }

    #[test]
    fn a_bad_step_or_target_changes_nothing() {
        let mut point = Damped::at(Vec3::ONE);
        for (target, dt) in [(Vec3::ZERO, 0.0), (Vec3::ZERO, f32::NAN), (Vec3::NAN, DT), (Vec3::ZERO, -1.0)] {
            point.step(target, OMEGA, NO_CAP, dt);
            assert_eq!((point.position(), point.velocity()), (Vec3::ONE, Vec3::ZERO));
        }
    }
}
