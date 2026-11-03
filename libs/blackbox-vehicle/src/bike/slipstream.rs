//! The wake of a vehicle ahead (`docs/specs/ride6-aero-drag.md`, section 3): how much it cuts the air drag of the vehicle that
//! follows. The game computes it in a world-level element; this is the formula, for a host that has several vehicles.

use glam::Vec3;

/// The parameters of the slipstream element.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Slipstream {
    /// The longest wake in m when `persistence` is 0, or the wake in seconds of the leader's travel.
    pub max_distance: f32,
    pub persistence: f32,
    pub distance_factor: f32,
    pub distance_exponent: f32,
    pub side_distance_factor: f32,
    pub side_distance_exponent: f32,
}

impl Default for Slipstream {
    /// The constructor defaults of the element, with the distances of the element asset the reference ships (70 m, 0.085, 2).
    fn default() -> Self {
        Self {
            max_distance: 70.0,
            persistence: 0.0,
            distance_factor: 0.085,
            distance_exponent: 1.0,
            side_distance_factor: 2.0,
            side_distance_exponent: 2.0,
        }
    }
}

/// The slowest leader (m/s) that has a wake.
const MIN_LEADER_SPEED: f32 = 3.0;

impl Slipstream {
    /// The share (0 to 1) of the air drag the follower at `follower` (m) saves behind a leader at `leader` moving with
    /// `velocity` (m/s). `half_width` is half the leader's width, taken off the lateral distance (0 for none).
    pub fn contribution(&self, follower: Vec3, leader: Vec3, velocity: Vec3, half_width: f32) -> f32 {
        let speed = velocity.length();
        if speed < MIN_LEADER_SPEED {
            return 0.0;
        }
        let unit = velocity / speed;
        let offset = follower - leader;
        let mut along = offset.dot(unit);
        let reach = match self.persistence > 0.0 {
            true => self.persistence * speed,
            false => self.max_distance,
        };
        if along >= 0.0 || along <= -reach {
            return 0.0;
        }
        let side = (offset - unit * along).length();
        let side = (side - half_width).max(0.0);
        if self.persistence > 0.0 {
            along *= 100.0 / reach;
        }
        let behind = (-along * self.distance_factor).powf(self.distance_exponent);
        let across = side.powf(self.side_distance_exponent) * self.side_distance_factor;
        (-(behind + across)).exp().clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_wake_is_the_values_the_spec_computes() {
        let wake = Slipstream::default();
        let leader = Vec3::new(0.0, 0.0, 10.0);
        let velocity = Vec3::new(0.0, 0.0, 30.0);
        let ten_behind = wake.contribution(Vec3::ZERO, leader, velocity, 0.0);
        assert!((ten_behind - 0.43).abs() < 0.01, "{ten_behind}");
        let beside = wake.contribution(Vec3::new(0.5, 0.0, 0.0), leader, velocity, 0.0);
        assert!((beside - 0.26).abs() < 0.01, "{beside}");
        assert!(
            wake.contribution(Vec3::ZERO, leader, velocity, 0.0)
                > wake.contribution(Vec3::new(0.0, 0.0, -40.0), leader, velocity, 0.0)
        );
    }

    #[test]
    fn nothing_ahead_of_the_leader_and_nothing_behind_a_slow_one() {
        let wake = Slipstream::default();
        let velocity = Vec3::new(0.0, 0.0, 30.0);
        assert_eq!(wake.contribution(Vec3::new(0.0, 0.0, 20.0), Vec3::ZERO, velocity, 0.0), 0.0);
        assert_eq!(wake.contribution(Vec3::ZERO, Vec3::new(0.0, 0.0, 10.0), Vec3::new(0.0, 0.0, 2.0), 0.0), 0.0);
        assert_eq!(wake.contribution(Vec3::new(0.0, 0.0, -100.0), Vec3::ZERO, velocity, 0.0), 0.0);
    }
}
