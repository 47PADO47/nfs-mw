//! The force one tyre puts on the ground, and the spin of its wheel.

use super::spec::WheelSpec;
use crate::ground::SurfaceGrip;

/// Slower than this (m/s) the slip is worked out against this speed, so a rolling wheel at a standstill does not divide
/// by zero. A numerical guard, not a tuning value.
const MIN_SLIP_SPEED: f32 = 1.0;

/// What the tyre sees this step.
#[derive(Clone, Copy, Debug)]
pub struct TyreInput {
    /// The load pressing the tyre on the ground (N).
    pub load: f32,
    /// The speed of the contact patch along the wheel's heading and across it, right positive (m/s).
    pub v_long: f32,
    pub v_lat: f32,
    /// The wheel's spin (rad/s).
    pub spin: f32,
    /// The lean of the wheel from the ground normal, positive towards the right (rad).
    pub camber: f32,
    pub grip: SurfaceGrip,
}

/// What the tyre does about it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TyreOutput {
    /// Along the wheel's heading on the ground (N), positive forwards.
    pub longitudinal: f32,
    /// Across it (N), positive to the right.
    pub lateral: f32,
    /// The slip angle (rad, positive when the patch moves to the right) and the slip ratio (1 = 100 percent).
    pub slip_angle: f32,
    pub slip_ratio: f32,
}

/// The force of the tyre on the road. Longitudinal and lateral forces come from the Magic Formula at the same load; the
/// pair is held inside the friction ellipse of their peaks, and the lateral force inside the tyre's lateral grip limit.
pub fn forces(wheel: &WheelSpec, input: &TyreInput) -> TyreOutput {
    if input.load <= 0.0 {
        return TyreOutput::default();
    }
    let fz = input.load * 1e-3;
    let reference = input.v_long.abs().max(MIN_SLIP_SPEED);
    let slip_ratio = (input.spin * wheel.radius - input.v_long) / reference;
    let slip_angle = input.v_lat.atan2(reference);
    // The formula's slip angle is positive for a force that opposes a rightward patch speed, and its camber thrust
    // follows the sign of the camber it is given: a bike leaning right is pushed right, so the camber is negated.
    let tyre = &wheel.tyre;
    let mut fx = tyre.longitudinal(fz, slip_ratio * 100.0) * input.grip.drive;
    let mut fy = -tyre.lateral(fz, slip_angle.to_degrees(), -input.camber.to_degrees()) * input.grip.lateral;
    let (peak_x, peak_y) = (tyre.longitudinal_peak(fz) * input.grip.drive, tyre.lateral_peak(fz) * input.grip.lateral);
    if peak_x > 1.0 && peak_y > 1.0 {
        let used = (fx / peak_x).powi(2) + (fy / peak_y).powi(2);
        if used > 1.0 {
            let scale = used.sqrt().recip();
            fx *= scale;
            fy *= scale;
        }
    }
    let limit = wheel.lateral_grip_limit * input.load;
    fy = fy.clamp(-limit, limit);
    TyreOutput { longitudinal: fx, lateral: fy, slip_angle, slip_ratio }
}

/// The rolling resistance (N), opposing the motion.
pub fn rolling_resistance(wheel: &WheelSpec, load: f32, v_long: f32) -> f32 {
    let [constant, linear, quadratic] = wheel.rolling;
    let speed = v_long.abs();
    let fade = (speed / 0.5).min(1.0);
    -v_long.signum() * fade * load * (constant + linear * speed + quadratic * speed * speed)
}

/// How fast a wheel has to spin (rad/s) before the brake gives its full torque: below it the brake torque fades out, so a
/// stopped wheel stays stopped instead of chattering.
const BRAKE_FADE_SPIN: f32 = 0.5;
/// The spin range (rad/s) either side of the old spin that the solve searches, before the torques widen it.
const SPIN_SEARCH: f32 = 20.0;

/// Advance the wheel by `dt` and return its new spin with the tyre forces at that spin.
///
/// The wheel obeys `I dw/dt = drive - brake - R Fx(w)`. At a crawl the slip ratio is very sensitive to the spin, so an
/// explicit step overshoots and flips the force every step; the equation is solved for the new spin instead. The residual
/// is negative far below the answer and positive far above it (the tyre force is bounded), so a bisection always finds a
/// root. `input.spin` is the spin at the start of the step.
pub fn advance_wheel(wheel: &WheelSpec, input: &TyreInput, drive: f32, brake: f32, dt: f32) -> (f32, TyreOutput) {
    let inertia = wheel.inertia.max(1e-3);
    let residual = |spin: f32| {
        let output = forces(wheel, &TyreInput { spin, ..*input });
        let braking = brake.abs() * (spin / BRAKE_FADE_SPIN).tanh();
        (spin - input.spin - dt / inertia * (drive - braking - output.longitudinal * wheel.radius), output)
    };
    let reach = SPIN_SEARCH
        + dt / inertia
            * (drive.abs() + brake.abs() + wheel.radius * input.load * wheel.lateral_grip_limit.max(2.0) * 2.0);
    let (mut low, mut high) = (input.spin - reach, input.spin + reach);
    for _ in 0..40 {
        let middle = 0.5 * (low + high);
        match residual(middle).0 < 0.0 {
            true => low = middle,
            false => high = middle,
        }
    }
    let spin = 0.5 * (low + high);
    (spin, residual(spin).1)
}

/// The spin of a wheel with no ground under it: the drive and the brake only.
pub fn spin_in_air(wheel: &WheelSpec, spin: f32, drive: f32, brake: f32, dt: f32) -> f32 {
    let inertia = wheel.inertia.max(1e-3);
    let free = spin + drive / inertia * dt;
    let braking = brake.abs() / inertia * dt;
    if free.abs() <= braking {
        return 0.0;
    }
    free - braking * free.signum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bike::BikeSpec;

    fn input(spin: f32, v_long: f32, v_lat: f32, camber: f32) -> TyreInput {
        TyreInput { load: 2000.0, v_long, v_lat, spin, camber, grip: SurfaceGrip::DEFAULT }
    }

    #[test]
    fn a_rolling_tyre_without_slip_gives_no_force() {
        let wheel = BikeSpec::example().rear;
        let out = forces(&wheel, &input(20.0 / wheel.radius, 20.0, 0.0, 0.0));
        assert!(out.longitudinal.abs() < 1.0 && out.lateral.abs() < 1.0, "{out:?}");
    }

    #[test]
    fn a_patch_sliding_right_is_pushed_left() {
        let wheel = BikeSpec::example().rear;
        let out = forces(&wheel, &input(20.0 / wheel.radius, 20.0, 1.0, 0.0));
        assert!(out.lateral < 0.0 && out.slip_angle > 0.0);
    }

    #[test]
    fn a_spinning_wheel_drives_forwards_and_a_locked_one_brakes() {
        let wheel = BikeSpec::example().rear;
        assert!(forces(&wheel, &input(25.0 / wheel.radius, 20.0, 0.0, 0.0)).longitudinal > 0.0);
        assert!(forces(&wheel, &input(0.0, 20.0, 0.0, 0.0)).longitudinal < 0.0);
    }

    #[test]
    fn a_bike_leaning_right_is_pushed_right_by_camber() {
        let mut wheel = BikeSpec::example().front;
        wheel.tyre.a[11] = 2.0;
        let out = forces(&wheel, &input(20.0 / wheel.radius, 20.0, 0.0, 30.0_f32.to_radians()));
        assert!(out.lateral > 0.0, "{out:?}");
    }

    #[test]
    fn the_combined_force_stays_inside_the_ellipse() {
        let wheel = BikeSpec::example().rear;
        let out = forces(&wheel, &input(40.0 / wheel.radius, 20.0, 4.0, 0.0));
        let fz = 2.0;
        let (px, py) = (wheel.tyre.longitudinal_peak(fz), wheel.tyre.lateral_peak(fz));
        assert!((out.longitudinal / px).powi(2) + (out.lateral / py).powi(2) <= 1.001);
    }

    #[test]
    fn rolling_resistance_opposes_the_motion() {
        let wheel = BikeSpec::example().rear;
        assert!(rolling_resistance(&wheel, 2000.0, 10.0) < 0.0);
        assert!(rolling_resistance(&wheel, 2000.0, -10.0) > 0.0);
        assert_eq!(rolling_resistance(&wheel, 2000.0, 0.0), 0.0);
    }

    #[test]
    fn a_brake_stops_a_free_wheel_and_does_not_reverse_it() {
        let wheel = BikeSpec::example().front;
        let mut spin = 30.0;
        for _ in 0..600 {
            spin = spin_in_air(&wheel, spin, 0.0, 200.0, 1.0 / 60.0);
        }
        assert_eq!(spin, 0.0);
    }

    #[test]
    fn a_wheel_on_the_road_settles_at_the_rolling_speed_and_stays_there() {
        let wheel = BikeSpec::example().rear;
        let mut spin = 0.0;
        for _ in 0..120 {
            spin = advance_wheel(&wheel, &input(spin, 15.0, 0.0, 0.0), 0.0, 0.0, 1.0 / 60.0).0;
        }
        assert!((spin * wheel.radius - 15.0).abs() < 0.3, "{}", spin * wheel.radius);
    }

    #[test]
    fn a_wheel_at_a_standstill_does_not_chatter() {
        let wheel = BikeSpec::example().front;
        let mut spin = 3.0;
        let mut worst: f32 = 0.0;
        for _ in 0..60 {
            let (next, out) = advance_wheel(&wheel, &input(spin, 0.05, 0.0, 0.0), 0.0, 0.0, 1.0 / 60.0);
            spin = next;
            worst = worst.max(spin.abs());
            assert!(out.longitudinal.is_finite());
        }
        assert!(worst <= 3.0 && spin.abs() < 0.5, "{spin}");
    }

    #[test]
    fn a_braked_wheel_on_the_road_locks_and_pulls_back() {
        let wheel = BikeSpec::example().front;
        let (mut spin, mut last) = (20.0 / wheel.radius, TyreOutput::default());
        for _ in 0..120 {
            (spin, last) = advance_wheel(&wheel, &input(spin, 20.0, 0.0, 0.0), 0.0, 900.0, 1.0 / 60.0);
        }
        assert!(spin < 0.5 * 20.0 / wheel.radius, "{spin}");
        assert!(last.longitudinal < -500.0, "{last:?}");
    }
}
