//! The head-on ram of a support SUV (`AIActionHeadOnRam`, spec `ai-pursuit-support.md` section 9 and the tactics spec
//! section 5.2). The SUV aims where it will meet the target if both hold their velocities, at a fixed 100 km/h, and does
//! not follow the road. Map plane throughout: `x` and `y` are the world's `x` and `z`, in metres.

use glam::Vec2;

/// Metres per second in 1 mile per hour.
const MPH: f32 = 0.44704;
/// The drive speed of the ram: 100 km/h (27.78 m/s, the binary constant).
pub const DRIVE_SPEED: f32 = 27.78;
/// The start speed cap: 60 mph, 26.82 m/s (the spec's cap of `GetDesiredSpeedToTarget` at the start).
pub const START_CAP: f32 = 26.82;
/// The target counts as stopped below this speed (m/s): the intercept is then the target's own position. Only a guard
/// against dividing by a zero velocity; the binary's behaviour there is not read.
const STILL: f32 = 1e-5;

/// The speed a ram car is placed at (`GetDesiredSpeedToTarget`, spec 5.2, a decompiled helper). `distance` is the signed
/// distance to the target (negative when the car is behind it), `target_speed` the target's speed (m/s). Capped at 26.82.
pub fn start_speed(distance: f32, target_speed: f32) -> f32 {
    let desired = match distance < 0.0 {
        true => (10.0 * MPH).max(0.5 * distance + target_speed),
        false => (25.0 * MPH).max(1.2 * target_speed).min(0.5 * distance + target_speed),
    };
    desired.min(START_CAP)
}

/// The intercept point (the spec's `Ram`): where a car at `me`, moving at `my_speed`, meets the target at `target` moving at
/// `target_vel`, if both keep their velocities. The earliest meeting time that is not negative is used. A target that is
/// not moving is met where it stands.
pub fn intercept(me: Vec2, my_speed: f32, target: Vec2, target_vel: Vec2) -> Vec2 {
    let speed = target_vel.length();
    if speed < STILL {
        return target;
    }
    let along = target_vel / speed;
    let across = Vec2::new(along.y, -along.x);
    let d = target - me;
    let d_along = along.dot(d);
    let d_across = across.dot(d);
    let a = speed * speed - my_speed * my_speed;
    let b = d_along * speed;
    let c = d_along * d_along + d_across * d_across;
    let t = match (a.abs() > STILL, b.abs() > STILL) {
        (true, _) => meeting_time(a, b, c),
        (false, true) => (-c / (2.0 * b)).max(0.0),
        (false, false) => 0.0,
    };
    target + target_vel * t
}

/// The desired velocity of the ram: the direction from `me` to the intercept, at `my_speed`.
pub fn heading(me: Vec2, my_speed: f32, target: Vec2, target_vel: Vec2) -> Vec2 {
    let point = intercept(me, my_speed, target, target_vel);
    (point - me).normalize_or_zero() * my_speed
}

/// The smaller non-negative root of `a t^2 + 2 b' t + c` in the spec's form (`t = (-b +- sqrt(b^2 - a c)) / a`), or the
/// larger when the smaller is negative, or 0 when neither is. Without a real root, 0 (unconfirmed: the spec is silent).
fn meeting_time(a: f32, b: f32, c: f32) -> f32 {
    let disc = b * b - a * c;
    if disc < 0.0 {
        return 0.0;
    }
    let root = disc.sqrt();
    let (r1, r2) = ((-b - root) / a, (-b + root) / a);
    let (lo, hi) = match r1 <= r2 {
        true => (r1, r2),
        false => (r2, r1),
    };
    match (lo >= 0.0, hi >= 0.0) {
        (true, _) => lo,
        (false, true) => hi,
        (false, false) => 0.0,
    }
}

/// One drive step of the ram: where the SUV steers and at what speed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Drive {
    /// The point in the map plane the SUV drives to.
    pub point: Vec2,
    /// The speed to drive at, m/s ([`DRIVE_SPEED`]).
    pub speed: f32,
}

/// The ram action of one SUV. It waits for its entry checks, then drives
/// at the intercept until the target is gone.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Ram {
    started: bool,
}

impl Ram {
    /// The action has started and has not been aborted.
    pub fn started(&self) -> bool {
        self.started
    }

    /// One think. `target` is its position and velocity; `me` and `my_speed` are the SUV's own. Returns
    /// the drive while it runs, or `None` while it waits to start.
    ///
    /// A missing target ends the action; a later target takes the entry checks again.
    pub fn think(&mut self, me: Vec2, my_speed: f32, target: Option<(Vec2, Vec2)>, eligible: bool) -> Option<Drive> {
        let Some((position, velocity)) = target else {
            self.started = false;
            return None;
        };
        if !self.started {
            if !eligible {
                return None;
            }
            self.started = true;
        }
        let step = heading(me, my_speed, position, velocity);
        Some(Drive { point: me + step, speed: DRIVE_SPEED })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Vec2, b: Vec2) -> bool {
        (a - b).length() < 1e-3
    }

    #[test]
    fn a_still_target_is_met_where_it_stands() {
        let point = intercept(Vec2::ZERO, 20.0, Vec2::new(100.0, 40.0), Vec2::ZERO);
        assert_eq!(point, Vec2::new(100.0, 40.0));
    }

    #[test]
    fn a_target_driving_away_down_the_road_is_met_further_on() {
        // The target runs +x at 10 m/s from 100 m out; the SUV at 20 m/s meets it at 200 m (10 s).
        let point = intercept(Vec2::ZERO, 20.0, Vec2::new(100.0, 0.0), Vec2::new(10.0, 0.0));
        assert!(close(point, Vec2::new(200.0, 0.0)), "{point:?}");
    }

    #[test]
    fn a_crossing_target_is_met_where_both_arrive_together() {
        // The target crosses +x at 10 m/s, 100 m up the y axis. Both reach the meeting point in the same time.
        let me = Vec2::ZERO;
        let target = Vec2::new(0.0, 100.0);
        let velocity = Vec2::new(10.0, 0.0);
        let point = intercept(me, 20.0, target, velocity);
        let my_time = (point - me).length() / 20.0;
        let target_time = (point - target).length() / 10.0;
        assert!((my_time - target_time).abs() < 1e-3, "{my_time} vs {target_time}");
        assert!(point.x > 0.0, "the meeting point is ahead of the target along its path: {point:?}");
    }

    #[test]
    fn the_heading_has_the_car_speed_and_points_at_the_intercept() {
        let step = heading(Vec2::ZERO, 20.0, Vec2::new(100.0, 0.0), Vec2::new(10.0, 0.0));
        assert!(close(step, Vec2::new(20.0, 0.0)), "{step:?}");
    }

    #[test]
    fn a_fast_target_is_not_a_start_veto() {
        let mut ram = Ram::default();
        let fast = Some((Vec2::new(100.0, 0.0), Vec2::new(45.0, 0.0)));
        assert!(ram.think(Vec2::ZERO, 20.0, fast, true).is_some());
    }

    #[test]
    fn the_start_speed_is_capped_at_26_82() {
        // A fast target: the desired speed is above the cap.
        assert_eq!(start_speed(340.0, 30.0), START_CAP);
        // A slow target with the car ahead: 1.2 times the target's speed, under the cap.
        assert!((start_speed(340.0, 20.0) - 24.0).abs() < 1e-4);
    }

    #[test]
    fn the_start_speed_behind_the_target_uses_the_low_floor() {
        // Behind (distance -10) with a stopped target: max(10 mph, -5) = 4.47 m/s.
        assert!((start_speed(-10.0, 0.0) - 10.0 * MPH).abs() < 1e-4);
    }

    #[test]
    fn a_ram_waits_for_drivability_without_waiting_for_the_target_to_slow() {
        let mut ram = Ram::default();
        let fast = Some((Vec2::new(100.0, 0.0), Vec2::new(45.0, 0.0)));
        assert!(ram.think(Vec2::ZERO, 20.0, fast, false).is_none());
        assert!(!ram.started());
        let drive = ram.think(Vec2::ZERO, 20.0, fast, true).expect("drives");
        assert_eq!(drive.speed, DRIVE_SPEED);
        assert!(ram.started());
    }

    #[test]
    fn a_started_ram_keeps_driving_when_entry_checks_fail() {
        let mut ram = Ram::default();
        ram.think(Vec2::ZERO, 20.0, Some((Vec2::new(100.0, 0.0), Vec2::ZERO)), true);
        assert!(ram.think(Vec2::ZERO, 20.0, Some((Vec2::new(100.0, 0.0), Vec2::new(30.0, 0.0))), false).is_some());
    }

    #[test]
    fn a_gone_target_aborts_the_ram() {
        let mut ram = Ram::default();
        ram.think(Vec2::ZERO, 20.0, Some((Vec2::new(100.0, 0.0), Vec2::ZERO)), true);
        assert!(ram.started());
        assert!(ram.think(Vec2::ZERO, 20.0, None, false).is_none());
        assert!(!ram.started(), "the start gate applies again");
    }
}
