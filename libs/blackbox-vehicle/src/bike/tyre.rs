//! The force one tyre puts on the ground, and the spin of its wheel (`docs/specs/ride6-tyre.md`).
//!
//! The slip is the slip speed over the total speed of the contact patch (smoothed below 1 m/s); the two pure forces of the
//! Magic Formula are joined by the normalised-slip rule with the tyre's own peak slips, not by a friction ellipse.

use super::choices;
use super::peaks::{Peaks, peaks};
use super::spec::WheelSpec;
use crate::ground::SurfaceGrip;

/// The runtime multiplier of the slip correction. Recovered: the reference game's wheel holds 1 at all times except in a burnout,
/// when its input handler writes the bike's `BurnoutSlipCorrectionMultiplier` (0.9 for the supermoto handler) into the rear wheel;
/// wet roads and other surfaces act through the tyre's grip tables, not through this factor (`ride6-chassis-round6.md`).
const SLIP_CORRECTION_MULTIPLIER: f32 = 1.0;

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
    /// How far the tyre is squashed by the load (m).
    pub compression: f32,
    /// The efficiency of the tyre from its temperature and wear (1 = a tyre at its best).
    pub efficiency: f32,
    pub grip: SurfaceGrip,
}

/// What the tyre does about it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TyreOutput {
    /// Along the wheel's heading on the ground (N), positive forwards.
    pub longitudinal: f32,
    /// Across it (N), positive to the right.
    pub lateral: f32,
    /// The slip angle (rad, positive when the patch moves to the right) and the slip ratio (1 = 100 percent), both against
    /// the total speed of the patch.
    pub slip_angle: f32,
    pub slip_ratio: f32,
    /// The longitudinal slip normalised to the force peaks: -1 at the braking peak, 0 at none, 1 at the driving peak.
    pub normalised_x: f32,
    /// The same for the lateral slip.
    pub normalised_y: f32,
    /// The wheel's `Slip.y` (the sine of the slip angle).
    pub slip_y: f32,
    /// The self-aligning moment (N m); 0 for a wheel that does not steer.
    pub aligning: f32,
}

/// What does not change with the spin of the wheel: computed once for the solve of the spin.
#[derive(Clone, Copy, Debug)]
pub struct Prepared {
    fz_kn: f32,
    gamma_deg: f32,
    peaks: Peaks,
    radius: f32,
    correction: f32,
}

/// The effective rolling radius (m): the radius less what the lean takes off a round tread and what the load squashes.
pub fn effective_radius(wheel: &WheelSpec, camber: f32, compression: f32) -> f32 {
    let shoulder = 0.5 * (1.0 - camber.cos().abs()) * wheel.toroid_factor * wheel.width;
    (wheel.radius - shoulder - wheel.compression_shape_factor * compression).max(wheel.radius * 0.5)
}

/// The factor that takes the slip speed to the slip below 1 m/s of patch speed: a smoothstep from 0.0297 at rest to 1.
pub fn low_speed_factor(speed: f32) -> f32 {
    let u = 0.99 * speed.clamp(0.0, 1.0) + 0.01;
    (3.0 - 2.0 * u) * u
}

pub fn prepare(wheel: &WheelSpec, input: &TyreInput) -> Prepared {
    let fz_kn = input.load * 1e-3;
    // The formula's camber thrust follows the sign of the camber it is given: a bike leaning right is pushed right, so the
    // camber is negated.
    let gamma_deg = -input.camber.to_degrees();
    let speed = input.v_long.hypot(input.v_lat);
    let blend = match wheel.rest_speed_threshold > 0.0 {
        true => (speed / wheel.rest_speed_threshold).clamp(0.0, 1.0),
        false => 1.0,
    };
    Prepared {
        fz_kn,
        gamma_deg,
        peaks: peaks(&wheel.tyre, fz_kn, gamma_deg),
        radius: effective_radius(wheel, input.camber, input.compression),
        correction: wheel.rest_slip_correction + (wheel.slip_correction - wheel.rest_slip_correction) * blend,
    }
}

/// The sign of `x`, 0 for 0.
fn sign(x: f32) -> f32 {
    match x == 0.0 {
        true => 0.0,
        false => x.signum(),
    }
}

/// `1 + 2 min(share, 0.5) (e - 1)`: the efficiency of the tyre takes grip away (`ride6-tyre-mf94.md`, section 7: the `min` is
/// against the constant 0.5, not against the combined slip).
fn efficiency_factor(share: f32, efficiency: f32) -> f32 {
    1.0 + 2.0 * share.min(0.5) * (efficiency - 1.0)
}

/// The force of the tyre on the road at the spin `spin` of the wheel.
pub fn forces_at(wheel: &WheelSpec, input: &TyreInput, prepared: &Prepared, spin: f32, steered: bool) -> TyreOutput {
    if input.load <= 0.0 {
        return TyreOutput::default();
    }
    let slip_speed = prepared.radius * spin - input.v_long;
    let speed = input.v_long.hypot(input.v_lat);
    let (slip_x, slip_y) = match speed > 1.0 {
        true => (slip_speed / speed, input.v_lat / speed),
        false => {
            let k = low_speed_factor(speed);
            (k * slip_speed, k * input.v_lat)
        }
    };
    let alpha = slip_y.clamp(-1.0, 1.0).asin();
    let p = &prepared.peaks;
    let span_x = (p.x_positive - p.x_negative).max(1e-4);
    let span_y = (p.y_positive - p.y_negative).max(1e-4);
    let nx = 2.0 * (slip_x - p.x_negative) / span_x - 1.0;
    let ny = 2.0 * (slip_y - p.y_negative) / span_y - 1.0;
    let combined = nx.hypot(ny) / (nx.abs() * SLIP_CORRECTION_MULTIPLIER * prepared.correction + 1.0);
    let mut output = TyreOutput {
        slip_angle: alpha,
        slip_ratio: slip_x,
        normalised_x: nx,
        normalised_y: ny,
        slip_y,
        ..TyreOutput::default()
    };
    if combined <= 1e-8 {
        return output;
    }
    let tyre = &wheel.tyre;
    let effective_x = (combined * sign(nx) * 0.5 + 0.5) * span_x + p.x_negative;
    let effective_y = (combined * sign(ny) * 0.5 + 0.5) * span_y + p.y_negative;
    let fx0 = tyre.longitudinal(prepared.fz_kn, effective_x * 100.0) * input.grip.drive;
    // The wheel hands the model its `Slip.y` as if it were the slip angle in radians, and negates what comes back.
    let fy0 = -tyre.lateral(prepared.fz_kn, effective_y.to_degrees(), prepared.gamma_deg) * input.grip.lateral;
    let balance = choices::TYRE_EFFICIENCY_BALANCE;
    output.longitudinal = fx0 * (nx.abs() / combined).min(1.0) * efficiency_factor(balance, input.efficiency);
    output.lateral = fy0 * (ny.abs() / combined).min(1.0) * efficiency_factor(1.0 - balance, input.efficiency);
    if steered {
        output.aligning = tyre.aligning(prepared.fz_kn, alpha.to_degrees(), prepared.gamma_deg) * input.grip.lateral;
    }
    output
}

/// The force of the tyre on the road at the spin of the input (the tests' shortcut).
#[cfg(test)]
fn forces(wheel: &WheelSpec, input: &TyreInput) -> TyreOutput {
    forces_at(wheel, input, &prepare(wheel, input), input.spin, false)
}

/// The size (N) of the rolling resistance of a wheel with load `load` (N): `Fz (c0 + (c2 v + c1) v + cS |slip_y|^p)` with `v`
/// the speed of the whole vehicle (m/s) and `slip_y` the wheel's lateral slip. It acts against the vehicle's velocity, above
/// 1 m/s only (`ride6-tyre-mf94.md`, section 6).
pub fn rolling_resistance(wheel: &WheelSpec, load: f32, vehicle_speed: f32, slip_y: f32) -> f32 {
    if vehicle_speed <= 1.0 {
        return 0.0;
    }
    let [constant, linear, quadratic] = wheel.rolling;
    let slip = wheel.rolling_slip * slip_y.abs().powf(wheel.rolling_slip_linearity);
    load * (constant + (quadratic * vehicle_speed + linear) * vehicle_speed + slip)
}

/// How fast a wheel has to spin (rad/s) before the brake gives its full torque: below it the brake torque fades out, so a
/// stopped wheel stays stopped instead of chattering.
const BRAKE_FADE_SPIN: f32 = 0.5;
/// The spin range (rad/s) either side of the old spin that the solve searches, before the torques widen it.
const SPIN_SEARCH: f32 = 20.0;

/// Advance the wheel by `dt` and return its new spin with the tyre forces at that spin.
///
/// The wheel obeys `I dw/dt = drive - brake - R Fx(w)`, `I` being the wheel's inertia plus `extra_inertia` (the engine's, seen
/// through a locked clutch). At a crawl the slip is very sensitive to the spin, so an explicit step overshoots and flips the
/// force every step; the equation is solved for the new spin instead. The residual is negative far below the answer and
/// positive far above it (the tyre force is bounded), so a bisection always finds a root. `input.spin` is the spin at the
/// start of the step.
pub fn advance_wheel(
    wheel: &WheelSpec,
    input: &TyreInput,
    drive: f32,
    brake: f32,
    extra_inertia: f32,
    steered: bool,
    dt: f32,
) -> (f32, TyreOutput) {
    let inertia = (wheel.inertia + extra_inertia).max(1e-3);
    let prepared = prepare(wheel, input);
    let residual = |spin: f32| {
        let output = forces_at(wheel, input, &prepared, spin, steered);
        let braking = brake.abs() * (spin / BRAKE_FADE_SPIN).tanh();
        (spin - input.spin - dt / inertia * (drive - braking - output.longitudinal * prepared.radius), output)
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
pub fn spin_in_air(wheel: &WheelSpec, spin: f32, drive: f32, brake: f32, extra_inertia: f32, dt: f32) -> f32 {
    let inertia = (wheel.inertia + extra_inertia).max(1e-3);
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
        TyreInput {
            load: 2000.0,
            v_long,
            v_lat,
            spin,
            camber,
            compression: 0.0,
            efficiency: 1.0,
            grip: SurfaceGrip::DEFAULT,
        }
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
        let locked = forces(&wheel, &input(0.0, 20.0, 0.0, 0.0));
        assert!(locked.longitudinal < 0.0 && locked.normalised_x < -1.0);
    }

    #[test]
    fn a_bike_leaning_right_is_pushed_right_by_camber() {
        let mut wheel = BikeSpec::example().front;
        wheel.tyre.a[11] = 2.0;
        // A leaned tyre rolls on a smaller radius, so the spin for no longitudinal slip is the speed over that radius (at
        // the nominal radius the wheel would be braking, and the normalised combination would mix that into the sideways
        // force).
        let camber = 30.0_f32.to_radians();
        let rolling = 20.0 / effective_radius(&wheel, camber, 0.0);
        let out = forces(&wheel, &input(rolling, 20.0, 0.0, camber));
        assert!(out.lateral > 0.0, "{out:?}");
    }

    #[test]
    fn a_lateral_slip_takes_longitudinal_force_away() {
        let wheel = BikeSpec::example().rear;
        let spin = 22.0 / wheel.radius;
        let straight = forces(&wheel, &input(spin, 20.0, 0.0, 0.0));
        let sliding = forces(&wheel, &input(spin, 20.0, 3.0, 0.0));
        assert!(sliding.longitudinal.abs() < straight.longitudinal.abs(), "{straight:?} {sliding:?}");
        assert!(sliding.lateral.abs() > 100.0);
    }

    #[test]
    fn the_pure_forces_reach_the_peak_of_the_formula() {
        let wheel = BikeSpec::example().rear;
        let peak = wheel.tyre.longitudinal_peak(2.0);
        let driving = (1..300)
            .map(|n| forces(&wheel, &input((20.0 + n as f32 * 0.1) / wheel.radius, 20.0, 0.0, 0.0)).longitudinal)
            .fold(0.0, f32::max);
        assert!(driving > 0.9 * peak && driving < 1.05 * peak, "{driving} {peak}");
    }

    #[test]
    fn the_low_speed_factor_is_a_smoothstep_from_rest_to_one_metre_per_second() {
        assert!((low_speed_factor(0.0) - 0.0298).abs() < 1e-4);
        assert_eq!(low_speed_factor(1.0), 1.0);
        assert!(low_speed_factor(0.5) > low_speed_factor(0.2));
        assert_eq!(low_speed_factor(5.0), 1.0);
    }

    #[test]
    fn a_leaned_round_tyre_rolls_on_a_smaller_radius_and_a_squashed_one_too() {
        let wheel = BikeSpec::example().front;
        let upright = effective_radius(&wheel, 0.0, 0.0);
        assert!((upright - wheel.radius).abs() < 1e-6);
        assert!(effective_radius(&wheel, 0.6, 0.0) < upright);
        assert!((upright - effective_radius(&wheel, 0.0, 0.01) - 0.01 * wheel.compression_shape_factor).abs() < 1e-6);
    }

    #[test]
    fn only_a_steered_wheel_has_an_aligning_moment() {
        let mut wheel = BikeSpec::example().front;
        wheel.tyre.c[0] = 2.0;
        wheel.tyre.c[1] = -1.5;
        wheel.tyre.c[2] = -10.0;
        wheel.tyre.c[3] = -0.3;
        wheel.tyre.c[4] = -10.0;
        let i = input(20.0 / wheel.radius, 20.0, 1.0, 0.0);
        let prepared = prepare(&wheel, &i);
        assert_eq!(forces_at(&wheel, &i, &prepared, i.spin, false).aligning, 0.0);
        assert!(forces_at(&wheel, &i, &prepared, i.spin, true).aligning != 0.0);
    }

    #[test]
    fn rolling_resistance_follows_the_formula_above_one_metre_per_second() {
        let wheel = BikeSpec::example().rear;
        // The spec's example: 30 m/s at 1000 N is 1000 (0.02 + 0.012 + 0.0009) = 32.9 N without slip.
        assert!((rolling_resistance(&wheel, 1000.0, 30.0, 0.0) - 32.9).abs() < 0.01);
        // The slip term adds cS |slip|^p.
        assert!((rolling_resistance(&wheel, 1000.0, 30.0, 0.1) - 32.9 - 20.0).abs() < 0.01);
        assert_eq!(rolling_resistance(&wheel, 2000.0, 0.5, 0.0), 0.0);
    }

    #[test]
    fn a_brake_stops_a_free_wheel_and_does_not_reverse_it() {
        let wheel = BikeSpec::example().front;
        let mut spin = 30.0;
        for _ in 0..600 {
            spin = spin_in_air(&wheel, spin, 0.0, 200.0, 0.0, 1.0 / 60.0);
        }
        assert_eq!(spin, 0.0);
    }

    #[test]
    fn a_wheel_on_the_road_settles_at_the_rolling_speed_and_stays_there() {
        let wheel = BikeSpec::example().rear;
        let mut spin = 0.0;
        for _ in 0..120 {
            spin = advance_wheel(&wheel, &input(spin, 15.0, 0.0, 0.0), 0.0, 0.0, 0.0, false, 1.0 / 60.0).0;
        }
        assert!((spin * wheel.radius - 15.0).abs() < 0.3, "{}", spin * wheel.radius);
    }

    #[test]
    fn a_wheel_at_a_standstill_does_not_chatter() {
        let wheel = BikeSpec::example().front;
        let mut spin = 3.0;
        let mut worst: f32 = 0.0;
        for _ in 0..60 {
            let (next, out) = advance_wheel(&wheel, &input(spin, 0.05, 0.0, 0.0), 0.0, 0.0, 0.0, false, 1.0 / 60.0);
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
            (spin, last) = advance_wheel(&wheel, &input(spin, 20.0, 0.0, 0.0), 0.0, 900.0, 0.0, false, 1.0 / 60.0);
        }
        assert!(spin < 0.5 * 20.0 / wheel.radius, "{spin}");
        assert!(last.longitudinal < -500.0, "{last:?}");
    }

    #[test]
    fn extra_inertia_slows_the_spin_up() {
        let wheel = BikeSpec::example().rear;
        // Rolling at 20 m/s: against a held road (speed 0) the tyre force pulls a spinning wheel down, and the lighter wheel
        // falls faster, which is not the spin-up this tests.
        let rolling = input(20.0 / wheel.radius, 20.0, 0.0, 0.0);
        let light = advance_wheel(&wheel, &rolling, 100.0, 0.0, 0.0, false, 1.0 / 60.0).0;
        let heavy = advance_wheel(&wheel, &rolling, 100.0, 0.0, 3.0, false, 1.0 / 60.0).0;
        assert!(heavy < light);
    }
}
