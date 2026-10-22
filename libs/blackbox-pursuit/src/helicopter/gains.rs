//! The flight and pursuit numbers a tuning file may change, each as the several values the controller uses. The
//! defaults are the game's own constants (`docs/specs/ai-helicopter-constants.md`); the file reader is
//! `crates/nfsmw/src/settings/helicopter.rs`.

/// The numbers of the follow, the approach brake and the flight model.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gains {
    /// How far ahead of the player the follow aims: `min(base + 0.4 * speed, cap)` metres. `[base, cap]`.
    pub leading: [f32; 2],
    /// The speed the follow flies at for a gap beyond 3 m, beyond 15 m and beyond 300 m, m/s. `[near, middle, far]`.
    pub closing_speed: [f32; 3],
    /// The acceleration limit `min(cap, 0.6 * speed + base)` m/s^2. `[cap, base]`.
    pub acceleration: [f32; 2],
    /// The braking: the divisor of the stopping distance and the share of the speed kept while braking hard.
    /// `[divisor, speed cut]`.
    pub brake: [f32; 2],
    /// The turn of the nose: the limit of the yaw rate (rad/s) and the rate per radian of heading error.
    /// `[limit, gain]`.
    pub turning: [f32; 2],
}

impl Default for Gains {
    fn default() -> Self {
        Self {
            leading: [30.0, 45.0],
            closing_speed: [20.0, 30.0, 50.0],
            acceleration: [80.0, 30.0],
            brake: [1.55, 0.4],
            turning: [1.3, 8.0],
        }
    }
}

#[cfg(test)]
mod tests {
    use glam::Vec3;

    use super::*;
    use crate::helicopter::behaviour::{Player, follow};
    use crate::helicopter::flight::{Chopper, Specs};

    fn player() -> Player {
        Player { position: Vec3::ZERO, velocity: Vec3::new(30.0, 0.0, 0.0), forward: Vec3::X }
    }

    #[test]
    fn the_leading_numbers_set_the_lead_and_its_cap() {
        let heli = Vec3::new(-100.0, 80.0, 0.0);
        let vanilla = follow(heli, &player(), -20.0, false, &Gains::default()).drive.x;
        assert!((vanilla - 42.0).abs() < 1e-4, "30 + 0.4 * 30");
        let near = Gains { leading: [10.0, 15.0], ..Gains::default() };
        let got = follow(heli, &player(), -20.0, false, &near).drive.x;
        assert!((got - 15.0).abs() < 1e-4, "capped at 15: {got}");
    }

    #[test]
    fn the_closing_speeds_set_the_pace_by_gap() {
        let p = Player { velocity: Vec3::ZERO, ..player() };
        let gains = Gains { closing_speed: [5.0, 6.0, 7.0], ..Gains::default() };
        let far = follow(Vec3::new(-500.0, 80.0, 0.0), &p, -20.0, false, &gains).speed;
        let mid = follow(Vec3::new(-100.0, 80.0, 0.0), &p, -20.0, false, &gains).speed;
        assert!((far - 7.0).abs() < 1e-3 && (mid - 6.0).abs() < 1e-3, "{far} {mid}");
    }

    #[test]
    fn the_acceleration_and_turning_numbers_reach_the_flight_model() {
        let specs =
            Specs::with_gains(100.0, &Gains { acceleration: [40.0, 10.0], turning: [0.5, 2.0], ..Gains::default() });
        assert_eq!((specs.accel_cap, specs.accel_base, specs.yaw_limit, specs.yaw_gain), (40.0, 10.0, 0.5, 2.0));
        let mut weak = Chopper::new(Vec3::new(0.0, 50.0, 0.0), 0.0);
        let mut strong = weak;
        let default = Specs::default();
        for _ in 0..60 {
            weak.step(1.0 / 60.0, Vec3::new(100.0, 0.0, 0.0), Vec3::X, false, &specs);
            strong.step(1.0 / 60.0, Vec3::new(100.0, 0.0, 0.0), Vec3::X, false, &default);
        }
        assert!(weak.velocity.x < strong.velocity.x, "a lower cap gives a slower build-up");
    }

    #[test]
    fn the_defaults_are_the_games_constants() {
        let g = Gains::default();
        assert_eq!((g.leading, g.closing_speed, g.acceleration), ([30.0, 45.0], [20.0, 30.0, 50.0], [80.0, 30.0]));
        assert_eq!((g.brake, g.turning), ([1.55, 0.4], [1.3, 8.0]));
    }
}
