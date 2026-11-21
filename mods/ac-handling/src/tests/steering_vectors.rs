//! The test vectors S1 to S10 and F1 to F6 of docs/specs/ac-handling/steering.md section 6, its properties, and the steering flag of
//! the controls hook. Tolerance 1e-4 absolute. Defaults of section 5, `dt = 1/60`, `device = pad` unless stated.

use blackbox_modding::sdk::{ControlsQuery, Role};

use std::f64::consts::FRAC_PI_6;

use super::assert_close;
use crate::aids::Levels;
use crate::controls::{ControlsCfg, ControlsState, Feed, controls_step};
use crate::keys::SteeringKeys;
use crate::steering::{Device, SteerState, ffb_step, lock_factor, rate_out, ratio, steer_step};

const DT: f64 = 1.0 / 60.0;

fn keys() -> SteeringKeys {
    SteeringKeys::default()
}

/// `ticks` ticks of the steering stage from `delta`; the angle after each.
fn run(k: &SteeringKeys, device: Device, delta: f64, steer_in: f64, v: f64, alpha_rear: f64, ticks: usize) -> Vec<f64> {
    let mut state = SteerState::seeded(delta);
    (0..ticks).map(|_| steer_step(k, device, &mut state, steer_in, v, alpha_rear, DT)).collect()
}

#[test]
fn s1_lock_factor_and_rate() {
    let k = keys();
    let rows = [
        (0.0, 1.00000, 30.000, 3.0000),
        (10.0, 0.86301, 25.890, 2.3538),
        (22.0, 0.60000, 18.000, 1.5664),
        (40.0, 0.38580, 11.574, 1.1589),
        (80.0, 0.25625, 7.687, 0.9713),
    ];
    for (v, factor, limit_deg, rate) in rows {
        assert_close(&format!("lock_factor({v})"), lock_factor(&k, v), factor, 0.0, 1e-4);
        assert_close(&format!("limit({v})"), lock_factor(&k, v) * k.delta_max_deg, limit_deg, 0.0, 1e-3);
        assert_close(&format!("rate_out({v})"), rate_out(&k, v), rate, 0.0, 1e-4);
    }
    assert_close("ratio", ratio(&k), 15.0, 0.0, 1e-9);
}

#[test]
fn s2_to_s4_rate_limit_and_return() {
    let k = keys();
    let angles = run(&k, Device::Pad, 0.0, 1.0, 0.0, 0.0, 12);
    for (tick, want) in [(1, 0.05), (2, 0.10), (5, 0.25), (10, 0.50), (11, FRAC_PI_6), (12, FRAC_PI_6)] {
        assert_close(&format!("S2 tick {tick}"), angles[tick - 1], want, 0.0, 1e-4);
    }
    let fast = run(&k, Device::Pad, 0.0, 1.0, 30.0, 0.0, 10);
    for (tick, want) in [(1, 0.0220), (2, 0.0440), (10, 0.2200)] {
        assert_close(&format!("S3 tick {tick}"), fast[tick - 1], want, 0.0, 1e-4);
    }
    assert_close("S3 limit at 30 m/s", lock_factor(&k, 30.0) * k.delta_max_deg.to_radians(), 0.2512, 0.0, 1e-4);
    assert_close("S4", run(&k, Device::Pad, 0.4, 0.0, 10.0, 0.0, 5)[4], 0.08615, 0.0, 1e-4);
    assert_close("S4 return rate", k.rho_ret * rate_out(&k, 10.0), 3.7662, 0.0, 1e-4);
}

#[test]
fn s5_to_s8_cross_shrink_shaping_and_counter_steer() {
    let k = keys();
    assert_close("S5", run(&k, Device::Pad, 0.2, -1.0, 10.0, 0.0, 1)[0], 0.13723, 0.0, 1e-4);
    assert_close("S6", run(&k, Device::Pad, 0.45, 1.0, 40.0, 0.0, 1)[0], 0.4191, 0.0, 1e-4);
    assert_close("S7", run(&k, Device::Pad, 0.0, 0.5, 0.0, 0.0, 60)[59], 0.21265, 0.0, 1e-4);
    assert_close("S8 into the slide", run(&k, Device::Pad, 0.0, 1.0, 40.0, 0.30, 60)[59], 0.3000, 0.0, 1e-4);
    assert_close("S8 wrong side", run(&k, Device::Pad, 0.0, 1.0, 40.0, -0.30, 60)[59], 0.2020, 0.0, 1e-4);
}

#[test]
fn s9_s10_a_wheel_device_is_trusted() {
    let k = keys();
    assert_close("S9", run(&k, Device::Wheel, 0.0, 0.5, 40.0, 0.0, 1)[0], 0.2618, 0.0, 1e-4);
    assert_close("S9 clamp", run(&k, Device::Wheel, 0.0, 1.7, 40.0, 0.0, 1)[0], FRAC_PI_6, 0.0, 1e-4);
    let filtered = SteeringKeys { tau_f: 0.05, ..keys() };
    let angles = run(&filtered, Device::Wheel, 0.0, 1.0, 0.0, 0.0, 3);
    for (got, want) in angles.iter().zip([0.14842, 0.25477, 0.33098]) {
        assert_close("S10", *got, want, 0.0, 1e-4);
    }
}

fn ffb(k: &SteeringKeys, state: &mut SteerState, mz: f64, v: f64) -> (f64, f64) {
    ffb_step(k, state, [mz, mz], [60.0, 60.0], v, DT)
}

#[test]
fn f1_the_force_feedback_ramps_up() {
    let k = keys();
    let mut state = SteerState::default();
    let wants = [(0.25129, 3.0155), (0.36050, 4.3260), (0.40796, 4.8955), (0.42859, 5.1431), (0.43755, 5.2506)];
    for (tick, (want, y)) in wants.into_iter().enumerate() {
        let (value, _) = ffb(&k, &mut state, 40.0, 20.0);
        assert_close(&format!("F1 ffb tick {}", tick + 1), value, want, 0.0, 1e-4);
        assert_close(&format!("F1 y_mz tick {}", tick + 1), state.y_mz, y, 0.0, 1e-3);
    }
}

#[test]
fn f2_to_f6_steady_state_clip_low_speed_gyro_and_curve() {
    let k = keys();
    let mut state = SteerState { y_mz: 80.0 / 15.0, ..SteerState::default() };
    let (value, col) = ffb(&k, &mut state, 40.0, 20.0);
    assert_close("F2", value, 0.44444, 0.0, 1e-4);
    assert_close("F2 column torque", col, 5.33333, 0.0, 1e-4);
    let mut state = SteerState { y_mz: -80.0 / 15.0, ..SteerState::default() };
    assert_close("F2 negative", ffb(&k, &mut state, -40.0, 20.0).0, -0.44444, 0.0, 1e-4);
    let mut state = SteerState { y_mz: 100.0, ..SteerState::default() };
    assert_close("F3 clip", ffb(&k, &mut state, 300.0, 20.0).0, 1.0, 0.0, 1e-9);
    let mut state = SteerState { y_mz: 80.0 / 15.0, ..SteerState::default() };
    assert_close("F4 low speed", ffb(&k, &mut state, 40.0, 1.4).0, 0.26667, 0.0, 1e-4);
    let mut state = SteerState { delta: 0.5 * DT, delta_prev: 0.0, y_mz: 0.0 };
    let (value, col) = ffb(&k, &mut state, 0.0, 20.0);
    assert_close("F5 column torque", col, -1.28, 0.0, 1e-4);
    assert_close("F5", value, -0.10667, 0.0, 1e-4);
    let curve = SteeringKeys { p_curve: 0.5, ..keys() };
    let mut state = SteerState { y_mz: 80.0 / 15.0, ..SteerState::default() };
    assert_close("F6", ffb(&curve, &mut state, 40.0, 20.0).0, 0.66667, 0.0, 1e-4);
}

#[test]
fn properties_of_the_steering_model() {
    let k = keys();
    // ffb is odd in (Mz, delta_dot) and bounded; with nothing it decays to zero.
    for mz in [5.0, 40.0, 400.0] {
        let (mut up, mut down) = (SteerState::default(), SteerState::default());
        let (a, b) = (ffb(&k, &mut up, mz, 20.0).0, ffb(&k, &mut down, -mz, 20.0).0);
        assert_close("odd", a, -b, 0.0, 1e-12);
        assert!(a.abs() <= 1.0);
    }
    let mut fading = SteerState { y_mz: 5.0, ..SteerState::default() };
    for _ in 0..600 {
        ffb_step(&k, &mut fading, [0.0; 2], [0.0; 2], 20.0, DT);
    }
    assert!(fading.y_mz.abs() < 1e-6);
    // s_sens = 0 keeps the full lock at every speed.
    let flat = SteeringKeys { s_sens: 0.0, ..keys() };
    for v in [0.0, 10.0, 40.0, 80.0] {
        assert_close("lock", lock_factor(&flat, v), 1.0, 0.0, 1e-12);
    }
    // The pad angle never changes by more than rho_ret * r_hi * dt per tick.
    let mut state = SteerState::default();
    let mut previous = 0.0;
    for tick in 0..200 {
        let input = if tick % 40 < 20 { 1.0 } else { -1.0 };
        let angle = steer_step(&k, Device::Pad, &mut state, input, 5.0, 0.0, DT);
        assert!((angle - previous).abs() <= k.rho_ret * k.r_hi * DT + 1e-12);
        previous = angle;
    }
}

fn query(steer: f32, v: f32) -> ControlsQuery {
    ControlsQuery {
        role: Some(Role::Player),
        vehicle_revision: 1,
        dt: DT as f32,
        steer,
        forward_speed: v,
        wheel_loaded: [true; 4],
        ..ControlsQuery::default()
    }
}

fn config(steering: bool) -> ControlsCfg {
    ControlsCfg { aids: false, steering, levels: Levels::default(), car: None, steer_keys: keys(), device: Device::Pad }
}

#[test]
fn the_steering_flag_is_set_only_while_the_steering_part_is_on() {
    let mut state = ControlsState::default();
    assert!(
        controls_step(&config(false), &mut state, &query(1.0, 10.0), &Feed::default()).is_none(),
        "nothing on: not handled"
    );
    let out = controls_step(&config(true), &mut state, &query(1.0, 0.0), &Feed::default()).expect("handled");
    assert_close("first tick of S2", f64::from(out.steer_target_rad.expect("the flag is set")), 0.05, 0.0, 1e-5);
    assert_eq!(out.brake, [0.0; 4], "the pedals pass through with the aids off");
    let both = ControlsCfg {
        aids: true,
        car: crate::aids::AidCar::from_spec(&crate::mapping::CarSpec::ref_sport()),
        ..config(false)
    };
    let out =
        controls_step(&both, &mut ControlsState::default(), &query(1.0, 10.0), &Feed::default()).expect("handled");
    assert_eq!(out.steer_target_rad, None, "aids alone leave the steering to the host");
}

#[test]
fn the_steering_stage_seeds_from_the_suspension_angle_and_feeds_on_the_rear_slip() {
    let mut state = ControlsState::default();
    let feed = Feed { seed_delta: 0.3, ..Feed::default() };
    let out = controls_step(&config(true), &mut state, &query(0.0, 20.0), &feed).expect("handled");
    // Centred at 20 m/s: the angle returns from the seed at the return rate, not by a snap.
    let first = f64::from(out.steer_target_rad.expect("flag"));
    assert!(first < 0.3 && first > 0.0, "{first}");
    // The rear slip angle in turns widens the lock when steering into the slide.
    let mut slide = ControlsState::default();
    let mut q = query(1.0, 40.0);
    q.slip_angle = [0.0, 0.0, 0.3 / std::f32::consts::TAU, 0.3 / std::f32::consts::TAU];
    let mut last = 0.0;
    for _ in 0..90 {
        last = f64::from(
            controls_step(&config(true), &mut slide, &q, &Feed::default())
                .expect("handled")
                .steer_target_rad
                .expect("flag"),
        );
    }
    assert_close("S8 through the query", last, 0.3, 0.0, 2e-4);
}
