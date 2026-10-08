//! Brake torque data and the per-wheel brake commands. Spec section 9 of
//! `docs/specs/vehicle-input-induction-brakes.md` and the brake part of `TuneWheelParams`.

use crate::math::{FT_LB_TO_NM, ms_to_mph};

/// Global scale on `BRAKES` (the data values are not plain N m).
pub const BRAKING_TORQUE_SCALE: f32 = 4.0;
/// Global scale on `EBRAKE`.
pub const EBRAKING_TORQUE_SCALE: f32 = 10.0;

/// Brake parameters (the attribute class `brakes`), torques in ft*lb.
#[derive(Clone, Copy, Debug)]
pub struct BrakeSpec {
    /// Maximum brake torque of the front `[0]` and rear `[1]` axle.
    pub brakes: [f32; 2],
    /// Fraction of that torque that can lock the wheel.
    pub brake_lock: [f32; 2],
    /// Handbrake torque (rear wheels only).
    pub ebrake: f32,
}

impl BrakeSpec {
    /// Brake torque of an axle (0 front, 1 rear) in N m.
    pub fn brake_torque(&self, axle: usize) -> f32 {
        self.brakes[axle] * FT_LB_TO_NM * BRAKING_TORQUE_SCALE
    }

    /// Torque at which the brakes of an axle can lock a wheel, in N m.
    pub fn lock_torque(&self, axle: usize) -> f32 {
        self.brake_lock[axle] * self.brake_torque(axle)
    }

    /// Handbrake torque in N m.
    pub fn ebrake_torque(&self) -> f32 {
        self.ebrake * FT_LB_TO_NM * EBRAKING_TORQUE_SCALE
    }
}

/// What the chassis knows when it sets a wheel's brakes.
#[derive(Clone, Copy, Debug)]
pub struct BrakeContext {
    /// Pedal brake 0..1.
    pub brake: f32,
    /// Handbrake 0..1.
    pub ebrake: f32,
    pub gas: f32,
    /// Body speed (m/s).
    pub speed: f32,
    /// Body slip angle in radians (+ = sliding right).
    pub slip_angle: f32,
    /// Brake-balance tuning slider: front x (1 + 0.5 b), rear x (1 - 0.5 b).
    pub tuning: f32,
}

/// Brake and handbrake command of one wheel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BrakeCommand {
    pub brake: f32,
    pub ebrake: f32,
}

/// The brake command for wheel `wheel` (0 front left, 1 front right, 2 rear left, 3 rear right).
pub fn wheel_command(ctx: &BrakeContext, wheel: usize, driven: bool, blown: bool) -> BrakeCommand {
    if blown {
        return BrakeCommand { brake: 1.0, ebrake: 0.0 };
    }
    let rear = wheel >= 2;
    let t = ctx.tuning;
    let mut brake = ctx.brake * if rear { 1.0 - 0.5 * t } else { 1.0 + 0.5 * t };
    let speed_mph = ms_to_mph(ctx.speed);
    // Launch-control stand: gas and brake together at a crawl on a driven wheel.
    if ctx.gas > 0.8 && ctx.brake > 0.5 && speed_mph.abs() < 10.0 && driven {
        brake = speed_mph.abs() * 0.05;
    }
    let mut ebrake = 0.0;
    if rear {
        ebrake = ctx.ebrake;
        if ctx.ebrake > 0.2 && ctx.slip_angle > 0.3 && speed_mph < 80.0 {
            ebrake += 0.5;
        }
    }
    BrakeCommand { brake, ebrake }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> BrakeContext {
        BrakeContext { brake: 1.0, ebrake: 0.0, gas: 0.0, speed: 20.0, slip_angle: 0.0, tuning: 0.0 }
    }

    #[test]
    fn spec_converts_to_newton_metres() {
        let b = BrakeSpec { brakes: [100.0, 50.0], brake_lock: [0.5, 0.5], ebrake: 20.0 };
        assert!((b.brake_torque(0) - 100.0 * 1.3558 * 4.0).abs() < 1e-3);
        assert!((b.lock_torque(1) - 0.5 * 50.0 * 1.3558 * 4.0).abs() < 1e-3);
        assert!((b.ebrake_torque() - 20.0 * 1.3558 * 10.0).abs() < 1e-3);
    }

    #[test]
    fn bias_tuning_moves_brake_force_forward() {
        let c = BrakeContext { tuning: 1.0, ..ctx() };
        assert_eq!(wheel_command(&c, 0, false, false).brake, 1.5);
        assert_eq!(wheel_command(&c, 3, false, false).brake, 0.5);
    }

    #[test]
    fn handbrake_only_on_the_rear() {
        let c = BrakeContext { brake: 0.0, ebrake: 1.0, ..ctx() };
        assert_eq!(wheel_command(&c, 1, false, false).ebrake, 0.0);
        assert_eq!(wheel_command(&c, 2, false, false).ebrake, 1.0);
        let slide = BrakeContext { slip_angle: 0.5, ..c };
        assert_eq!(wheel_command(&slide, 2, false, false).ebrake, 1.5);
    }

    #[test]
    fn launch_stand_releases_a_driven_wheel() {
        let c = BrakeContext { gas: 1.0, speed: 2.0, ..ctx() };
        let driven = wheel_command(&c, 2, true, false);
        assert!((driven.brake - 2.0 * 2.2369 * 0.05).abs() < 1e-4);
        assert_eq!(wheel_command(&c, 0, false, false).brake, 1.0);
    }

    #[test]
    fn blown_tire_locks_on() {
        let c = BrakeContext { brake: 0.0, ebrake: 1.0, ..ctx() };
        assert_eq!(wheel_command(&c, 2, true, true), BrakeCommand { brake: 1.0, ebrake: 0.0 });
    }
}
