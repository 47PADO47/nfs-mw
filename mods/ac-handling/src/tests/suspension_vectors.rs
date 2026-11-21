//! The test vectors V1 to V17 of docs/specs/ac-handling/suspension.md section 6, and its properties. Front-left wheel, default front
//! constants, `load_est = 3739.78`, `dt = 1/60`, `c_rest = 0.026`. Tolerance: 1e-4 relative for the closed forms, 1e-2 for the
//! dynamic lines (V9 to V13), which pass through the stop thresholds; they are preceded by one rest tick (the reseed).

use super::assert_close;
use crate::keys::SuspensionKeys;
use crate::suspension::{
    AxleCfg, AxleIn, AxleState, WheelIn, WheelOut, anti_roll, axle_call, camber, damper, roll_centre, spring, stops,
    substeps,
};

const C_REST: f64 = 0.026;
const LOAD: f64 = 3739.78;

fn cfg() -> AxleCfg {
    AxleCfg { keys: SuspensionKeys::default(), track: 1.52 }
}

fn wheel(u: f64, u_prev: f64, dt: f64) -> WheelIn {
    WheelIn { loaded: true, c: C_REST + u, c_dot: (u - u_prev) / dt, load_est: LOAD, ..WheelIn::default() }
}

fn axle(dt: f64, left: WheelIn, right: WheelIn) -> AxleIn {
    AxleIn { dt, c_rest: C_REST, c_max: 0.15, m_corner: 350.0, wheels: [left, right] }
}

fn rest(dt: f64) -> AxleIn {
    axle(dt, wheel(0.0, 0.0, dt), wheel(0.0, 0.0, dt))
}

/// A seeded state: one rest tick has run.
fn seeded(dt: f64) -> AxleState {
    let mut state = AxleState::new();
    axle_call(&cfg(), &mut state, &rest(dt)).expect("the rest tick is handled");
    state
}

fn call(state: &mut AxleState, input: &AxleIn) -> [WheelOut; 2] {
    axle_call(&cfg(), state, input).expect("handled")
}

#[test]
fn v1_one_tick_at_rest() {
    let mut state = AxleState::new();
    let out = call(&mut state, &rest(1.0 / 60.0));
    for wheel in out {
        assert_close("body_force", wheel.body_force, 3367.0, 1e-4, 1e-3);
        assert_close("tyre_load", wheel.tyre_load, 3739.78, 1e-4, 1e-3);
        assert_close("camber", wheel.camber, -0.035, 1e-4, 1e-6);
    }
    assert_close("x_L", state.hubs[0].x, 0.0, 0.0, 1e-12);
    assert_close("x_R", state.hubs[1].x, 0.0, 0.0, 1e-12);
}

#[test]
fn v2_to_v6_the_closed_forms() {
    let k = SuspensionKeys::default();
    assert_close("V2 +", spring(&k, 3367.0, 0.02), 3967.0, 1e-4, 1e-6);
    assert_close("V2 -", spring(&k, 3367.0, -0.02), 2767.0, 1e-4, 1e-6);
    let quad = SuspensionKeys { k2: 2.0e6, ..k.clone() };
    assert_close("V3 +", spring(&quad, 3367.0, 0.03), 6067.0, 1e-4, 1e-6);
    assert_close("V3 -", spring(&quad, 3367.0, -0.03), 667.0, 1e-4, 1e-6);
    for (v, want) in [(0.05, 95.0), (0.15, 285.0), (0.5, 530.0), (-0.05, -190.0), (-0.20, -760.0), (-0.60, -1320.0)] {
        assert_close(&format!("V4 F_d({v})"), damper(&k, v), want, 1e-4, 1e-6);
    }
    for (x, want) in [(0.03, 0.0), (0.07, 5000.0), (-0.05, 0.0), (-0.09, -10000.0)] {
        assert_close(&format!("V5 F_stop({x})"), stops(&k, x), want, 1e-4, 1e-6);
    }
    let arb = anti_roll(&k, [0.02, -0.01]);
    assert_close("V6 L", arb[0], 600.0, 1e-4, 1e-6);
    assert_close("V6 R", arb[1], -600.0, 1e-4, 1e-6);
}

#[test]
fn v7_camber_and_its_clamp() {
    let cfg = cfg();
    let got = camber(&cfg, [0.03, -0.02], [0.2, 0.2]);
    assert_close("V7 L", got[0], -0.034105, 1e-4, 1e-6);
    assert_close("V7 R", got[1], -0.039895, 1e-4, 1e-6);
    assert_close("V7b rest", camber(&cfg, [0.0, 0.0], [0.0, 0.0])[0], -0.035, 1e-4, 1e-6);
    let clamped = camber(&cfg, [0.0, 0.0], [0.0, 4.0]);
    assert_close("V7b clamp", clamped[1], 0.30, 1e-9, 1e-9);
    // Unclamped it would be gamma0 + sigma * kappa * steer = -0.035 + 0.1 * 4.0.
    assert_close("V7b unclamped", cfg.keys.gamma0 + cfg.keys.kappa * 4.0, 0.365, 1e-4, 1e-6);
}

#[test]
fn v8_the_roll_centre_pair() {
    let pair = roll_centre(&SuspensionKeys::default(), 1.52, -6000.0);
    assert_close("V8 L", pair[0], -236.842, 1e-4, 1e-3);
    assert_close("V8 R", pair[1], 236.842, 1e-4, 1e-3);
    assert_close("the pair sums to zero", pair[0] + pair[1], 0.0, 0.0, 1e-9);
}

#[test]
fn v9_a_road_step_on_the_left_wheel_only() {
    let dt = 1.0 / 60.0;
    let mut state = seeded(dt);
    // tick, x_L, v_L, body_force_L, tyre_load_L, x_R, body_force_R
    let rows = [
        (1, 0.0076, 0.9294, 3821.88, 6313.64, 0.0001, 3334.89),
        (2, 0.0236, 0.6743, 4991.79, 4783.04, 0.0008, 3152.16),
        (5, 0.0152, 0.0658, 3709.38, 4937.23, 0.0011, 2978.25),
        (10, 0.0167, 0.0023, 4176.85, 4563.86, 0.0011, 3077.93),
    ];
    for tick in 1..=10 {
        let left = wheel(0.02, if tick == 1 { 0.0 } else { 0.02 }, dt);
        let out = call(&mut state, &axle(dt, left, wheel(0.0, 0.0, dt)));
        let Some(row) = rows.iter().find(|row| row.0 == tick) else {
            continue;
        };
        assert_close(&format!("V9 tick {tick} x_L"), state.hubs[0].x, row.1, 1e-2, 1e-4);
        assert_close(&format!("V9 tick {tick} v_L"), state.hubs[0].v, row.2, 1e-2, 1e-3);
        assert_close(&format!("V9 tick {tick} body_force_L"), out[0].body_force, row.3, 1e-2, 0.0);
        assert_close(&format!("V9 tick {tick} tyre_load_L"), out[0].tyre_load, row.4, 1e-2, 0.0);
        assert_close(&format!("V9 tick {tick} x_R"), state.hubs[1].x, row.5, 1e-2, 1e-4);
        assert_close(&format!("V9 tick {tick} body_force_R"), out[1].body_force, row.6, 1e-2, 0.0);
    }
}

#[test]
fn v10_an_airborne_axle_falls_to_the_droop_stop() {
    let dt = 1.0 / 60.0;
    let mut state = seeded(dt);
    let air = WheelIn { loaded: false, c: 0.0, c_dot: 0.0, a_up: -9.81, load_est: LOAD, ..WheelIn::default() };
    let rows = [(1, -0.0103, -0.9569, 2181.82), (5, -0.0774, 0.1711, -2702.27), (30, -0.0724, 0.0, 0.0002)];
    for tick in 1..=30 {
        let out = call(&mut state, &axle(dt, air, air));
        assert_eq!(out[0].tyre_load, 0.0, "an airborne wheel has no tyre load");
        let Some(row) = rows.iter().find(|row| row.0 == tick) else {
            continue;
        };
        assert_close(&format!("V10 tick {tick} x"), state.hubs[0].x, row.1, 1e-2, 1e-4);
        assert_close(&format!("V10 tick {tick} v"), state.hubs[0].v, row.2, 1e-2, 1e-3);
        assert_close(&format!("V10 tick {tick} body_force"), out[0].body_force, row.3, 1e-2, 0.01);
    }
}

#[test]
fn v11_a_big_step_on_both_wheels_rings_between_tyre_and_bump_stop() {
    let dt = 1.0 / 60.0;
    let mut state = seeded(dt);
    let rows = [
        (1, 0.0316, 3.9014, 4686.65, 13954.53),
        (2, 0.0918, 1.0996, 13317.26, 7301.95),
        (3, 0.0534, -2.4272, 12102.26, 4434.11),
        (4, 0.0503, 1.2049, 3424.15, 12078.07),
        (5, 0.0747, 0.6470, 8808.63, 7909.40),
    ];
    for tick in 1..=5 {
        let u_prev = if tick == 1 { 0.0 } else { 0.08 };
        let both = wheel(0.08, u_prev, dt);
        let out = call(&mut state, &axle(dt, both, both));
        let row = rows[tick - 1];
        assert_close(&format!("V11 tick {tick} x"), state.hubs[0].x, row.1, 1e-2, 1e-4);
        assert_close(&format!("V11 tick {tick} v"), state.hubs[0].v, row.2, 1e-2, 1e-3);
        assert_close(&format!("V11 tick {tick} body_force"), out[0].body_force, row.3, 1e-2, 0.0);
        assert_close(&format!("V11 tick {tick} tyre_load"), out[0].tyre_load, row.4, 1e-2, 0.0);
    }
}

#[test]
fn v12_a_steady_lateral_force_moves_load_through_the_links() {
    let dt = 1.0 / 60.0;
    let mut state = seeded(dt);
    let side = WheelIn { fy: -3000.0, ..wheel(0.0, 0.0, dt) };
    let mut out = [WheelOut::default(); 2];
    for _ in 0..30 {
        out = call(&mut state, &axle(dt, side, side));
    }
    assert_close("V12 L body_force", out[0].body_force, 3181.97, 1e-2, 0.0);
    assert_close("V12 L tyre_load", out[0].tyre_load, 3554.75, 1e-2, 0.0);
    assert_close("V12 L x", state.hubs[0].x, 0.0007, 1e-2, 1e-4);
    assert_close("V12 R body_force", out[1].body_force, 3552.03, 1e-2, 0.0);
    assert_close("V12 R tyre_load", out[1].tyre_load, 3924.81, 1e-2, 0.0);
    assert_close("V12 R x", state.hubs[1].x, -0.0007, 1e-2, 1e-4);
}

#[test]
fn v13_the_step_converges_with_the_tick() {
    let result = |dt: f64, ticks: usize| {
        let mut state = seeded(dt);
        let mut out = [WheelOut::default(); 2];
        for tick in 0..ticks {
            let left = wheel(0.02, if tick == 0 { 0.0 } else { 0.02 }, dt);
            out = call(&mut state, &axle(dt, left, wheel(0.0, 0.0, dt)));
        }
        (state.hubs[0].x, out[0].body_force)
    };
    let (x60, f60) = result(1.0 / 60.0, 10);
    let (x144, f144) = result(1.0 / 144.0, 24);
    assert_close("x at 60 Hz", x60, 0.0167, 1e-2, 1e-4);
    assert_close("x at 144 Hz", x144, 0.0167, 1e-2, 1e-4);
    assert_close("body_force at 60 Hz", f60, 4176.9, 1e-2, 0.0);
    assert_close("body_force at 144 Hz", f144, 4185.7, 1e-2, 0.0);
    assert_close("within 0.5 %", f144, f60, 5e-3, 0.0);
}

#[test]
fn v14_the_substep_count() {
    for (dt, n) in [(1.0 / 60.0, 7), (1.0 / 144.0, 3), (1.0 / 240.0, 2), (0.1, 16)] {
        assert_eq!(substeps(dt), n, "dt {dt}");
    }
}

#[test]
fn v15_the_query_gives_the_road_input_and_a_car_at_rest_reads_zero_whatever_the_sag() {
    let dt = 1.0 / 60.0;
    // compression 0.046 over a sag of 0.026 is a road input of 0.020, moving at 1.2 m/s: u_prev = 0.020 - 1.2 / 60 = 0.
    let (c, c_rest, c_dot) = (0.046_f64, 0.026_f64, 1.2_f64);
    assert_close("u", c - c_rest, 0.020, 1e-9, 1e-9);
    assert_close("u_prev", (c - c_rest) - c_dot * dt, 0.0, 0.0, 1e-9);
    // Whatever the sag, the same road input gives the same answer: a shifted compression changes nothing.
    let run = |sag: f64| {
        let mut state = AxleState::new();
        let w = |u: f64| WheelIn { loaded: true, c: sag + u, c_dot: 0.0, load_est: LOAD, ..WheelIn::default() };
        let input = AxleIn { dt, c_rest: sag, c_max: 0.15, m_corner: 350.0, wheels: [w(0.01), w(0.0)] };
        let first = axle_call(&cfg(), &mut state, &input).expect("handled");
        let second = axle_call(&cfg(), &mut state, &input).expect("handled");
        (first, second)
    };
    assert_eq!(run(0.026), run(0.05));
    let (at_rest, _) = {
        let mut state = AxleState::new();
        let w = WheelIn { loaded: true, c: 0.04, c_dot: 0.0, load_est: LOAD, ..WheelIn::default() };
        let input = AxleIn { dt, c_rest: 0.04, c_max: 0.15, m_corner: 350.0, wheels: [w, w] };
        (axle_call(&cfg(), &mut state, &input).expect("handled"), ())
    };
    assert_close("a car at rest, any sag", at_rest[0].body_force, 3367.0, 1e-4, 1e-3);
}

#[test]
fn v16_touchdown_starts_without_road_velocity() {
    let dt = 1.0 / 60.0;
    let air = WheelIn { loaded: false, c: 0.0, c_dot: 0.0, load_est: LOAD, ..WheelIn::default() };
    let land = |c_dot: f64| {
        let mut state = seeded(dt);
        call(&mut state, &axle(dt, air, air));
        let down = WheelIn { loaded: true, c: C_REST, c_dot, load_est: LOAD, ..WheelIn::default() };
        let out = call(&mut state, &axle(dt, down, down));
        (out, state.hubs[0].x)
    };
    let (with_speed, x_fast) = land(1.56);
    let (without, x_slow) = land(0.0);
    assert_eq!(with_speed, without, "the first loaded tick uses u' = 0, not the host's 1.56");
    assert_eq!(x_fast, x_slow);
}

#[test]
fn v17_an_axle_call_at_rest_answers_both_wheels_alike() {
    let mut state = AxleState::new();
    let out = call(&mut state, &rest(1.0 / 60.0));
    assert_eq!(out[0], out[1]);
    assert_close("body_force_n", out[0].body_force, 3367.0, 1e-4, 1e-3);
    assert_close("tyre_load_n", out[0].tyre_load, 3739.78, 1e-4, 1e-3);
    assert_close("camber_rad", out[0].camber, -0.035, 1e-4, 1e-6);
}

#[test]
fn not_handled_rules_decline_both_wheels_and_reseed() {
    let dt = 1.0 / 60.0;
    let good = rest(dt);
    let mut bad_inputs = vec![
        AxleIn { dt: 0.0, ..good },
        AxleIn { dt: 0.5, ..good },
        AxleIn { dt: f64::NAN, ..good },
        AxleIn { c_rest: f64::INFINITY, ..good },
    ];
    let mut heavy_hub = good;
    heavy_hub.wheels[1].load_est = 300.0; // not above the unsprung weight
    bad_inputs.push(heavy_hub);
    let mut nan_wheel = good;
    nan_wheel.wheels[0].a_up = f64::NAN;
    bad_inputs.push(nan_wheel);
    for input in bad_inputs {
        let mut state = seeded(dt);
        assert!(axle_call(&cfg(), &mut state, &input).is_none());
        assert!(state.reseed, "a declined call reseeds both hubs at the next call");
    }
    let mut state = AxleState::new();
    assert!(axle_call(&cfg(), &mut state, &AxleIn { dt: 0.1, ..good }).is_some(), "0.1 s is handled");
}

#[test]
fn a_failure_declines_the_axle_instead_of_sending_a_refused_answer() {
    // A wheel pushed 5 m/s past a huge hub travel is not a real road: the model must decline, never answer out of range.
    let dt = 1.0 / 60.0;
    let mut state = seeded(dt);
    let wild = wheel(1.0, 0.0, dt);
    let mut declined = false;
    for _ in 0..30 {
        match axle_call(&cfg(), &mut state, &axle(dt, wild, wild)) {
            None => declined = true,
            Some(out) => assert!(
                out.iter().all(|w| w.body_force.abs() <= 1.0e6 && w.tyre_load <= 1.0e6 && w.camber.abs() <= 0.6)
            ),
        }
    }
    assert!(declined, "the runaway hub was declined");
}

#[test]
fn properties_hold_over_random_inputs() {
    let dt = 1.0 / 60.0;
    let mut seed = 99_u64;
    let mut next = move || {
        seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        ((seed >> 33) as f64) / f64::from(1_u32 << 31)
    };
    let mut state = seeded(dt);
    for _ in 0..500 {
        let left = WheelIn {
            loaded: next() > 0.1,
            c: C_REST + (next() - 0.5) * 0.06,
            c_dot: (next() - 0.5) * 2.0,
            load_est: LOAD,
            fy: (next() - 0.5) * 6000.0,
            ..WheelIn::default()
        };
        let right = WheelIn {
            loaded: next() > 0.1,
            c: C_REST + (next() - 0.5) * 0.06,
            c_dot: (next() - 0.5) * 2.0,
            load_est: LOAD,
            fy: (next() - 0.5) * 6000.0,
            ..WheelIn::default()
        };
        let input = axle(dt, left, right);
        let pair = roll_centre(&SuspensionKeys::default(), 1.52, left.fy + right.fy);
        assert!((pair[0] + pair[1]).abs() < 1e-9, "the pair sums to zero");
        let Some(out) = axle_call(&cfg(), &mut state, &input) else {
            continue;
        };
        for (wheel, result) in input.wheels.iter().zip(out) {
            assert!(result.tyre_load >= 0.0);
            if !wheel.loaded {
                assert_eq!(result.tyre_load, 0.0);
            }
        }
    }
    // With h_rc = 0, k_arb = 0 and a symmetric road input the two wheels give identical outputs.
    let plain = AxleCfg { keys: SuspensionKeys { h_rc: 0.0, k_arb: 0.0, ..SuspensionKeys::default() }, track: 1.52 };
    let mut state = AxleState::new();
    let same = wheel(0.01, 0.0, dt);
    let out = axle_call(&plain, &mut state, &axle(dt, same, same)).expect("handled");
    assert_eq!(out[0], out[1]);
    // One tick at rest returns the V1 values for any dt from 1/240 to 1/30.
    for dt in [1.0 / 240.0, 1.0 / 144.0, 1.0 / 60.0, 1.0 / 30.0] {
        let out = axle_call(&cfg(), &mut AxleState::new(), &rest(dt)).expect("handled");
        assert_close("rest body_force", out[0].body_force, 3367.0, 1e-4, 1e-3);
    }
}
