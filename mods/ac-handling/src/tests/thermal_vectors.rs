//! The test vectors T1 to T12 of docs/specs/ac-handling/tyre-thermal.md section 6, and its properties. Tolerances of the spec:
//! 1e-4 for the pure functions, 0.1 K for temperatures, 0.2 kPa for pressures, 0.002 for factors, 0.5 % for wear after a long run.

use super::assert_close;
use crate::grip::{GripInputs, GripScale, NoThermal, ThermalGrip};
use crate::thermal::{ThermalParams, ThermalState, ThermalTick};

fn params() -> ThermalParams {
    ThermalParams::default()
}

#[test]
fn t1_the_heat_shares() {
    let p = params();
    let rows = [
        (0.0, 230.0, [0.33333, 0.33333, 0.33333]),
        (-0.035, 230.0, [0.43833, 0.33333, 0.22833]),
        (-0.035, 260.0, [0.35257, 0.46377, 0.18366]),
        (0.05, 230.0, [0.18333, 0.33333, 0.48333]),
        (-0.20, 230.0, [0.66667, 0.33333, 0.00000]),
        (-0.035, 150.0, [0.56983, 0.13333, 0.29683]),
        (0.0, 400.0, [0.23333, 0.53333, 0.23333]),
    ];
    for (gamma, pressure, want) in rows {
        let got = p.shares(gamma, pressure);
        for i in 0..3 {
            assert_close(&format!("shares({gamma}, {pressure})[{i}]"), got[i], want[i], 0.0, 2e-5);
        }
        assert_close("sum", got.iter().sum::<f64>(), 1.0, 0.0, 1e-12);
    }
}

#[test]
fn t2_the_pressure_follows_the_core() {
    let p = params();
    for (core, want) in [(20.0, 190.0), (50.0, 219.8132), (80.0, 249.6265), (-10.0, 160.1868)] {
        assert_close(&format!("P({core})"), p.pressure(190.0, 20.0, core), want, 0.0, 1e-3);
    }
}

#[test]
fn t3_t4_t5_the_three_grip_factors() {
    let p = params();
    let temps = [-10.0, 20.0, 40.0, 63.0, 75.0, 87.0, 100.0, 130.0, 200.0];
    let g_t = [0.67072, 0.83506, 0.94490, 1.0, 1.0, 1.0, 0.98161, 0.83506, 0.56923];
    for (t, want) in temps.into_iter().zip(g_t) {
        assert_close(&format!("g_T({t})"), p.g_t(t), want, 0.0, 1e-4);
    }
    let pressures = [230.0, 200.0, 190.0, 260.0, 150.0, 400.0];
    let g_p = [1.0, 0.96525, 0.95420, 0.96525, 0.91241, 0.83056];
    for (pressure, want) in pressures.into_iter().zip(g_p) {
        assert_close(&format!("g_P({pressure})"), p.g_p(pressure), want, 0.0, 1e-4);
    }
    let wears = [0.0, 0.25, 1.0, 2.0, 3.0, 6.0, 9.0, 10.0, 15.0];
    let g_w = [1.0, 0.9925, 0.97333, 0.95, 0.915, 0.8, 0.735, 0.72, 0.72];
    for (wear, want) in wears.into_iter().zip(g_w) {
        assert_close(&format!("g_W({wear})"), p.g_w(wear), want, 0.0, 1e-4);
    }
}

#[test]
fn t6_the_multiplier_and_the_life() {
    let p = params();
    let rows = [
        (20.0, 190.0, 0.0, 0.79681),
        (75.0, 230.0, 0.0, 1.0),
        (75.0, 230.0, 10.0, 0.72),
        (110.0, 260.0, 4.0, 0.80262),
        (40.0, 150.0, 2.0, 0.81903),
    ];
    for (t, pressure, wear, want) in rows {
        assert_close(&format!("grip_scale({t}, {pressure}, {wear})"), p.grip_scale(t, pressure, wear), want, 0.0, 1e-4);
    }
    for (wear, want) in [(0.0, 100.0), (6.0, 28.6), (10.0, 0.0), (15.0, 0.0)] {
        assert_close(&format!("life({wear})"), p.life_percent(wear), want, 0.0, 0.05);
    }
}

fn tick(dt: f64, p_slide: f64, v_slide: f64, gamma: f64) -> ThermalTick {
    ThermalTick { dt, p_slide, v_slide, fz: 3500.0, v: 25.0, gamma, t_amb: 20.0, t_road: 30.0, loaded: true }
}

#[test]
fn t7_one_tick_from_cold() {
    let p = params();
    let mut corner = ThermalState::seed(&p, 20.0, 20.0);
    corner.step(&p, &tick(1.0 / 60.0, 7626.12, 2.17889, -0.035));
    for (got, want) in corner.t_s.iter().zip([20.022352, 20.006777, 20.011865]) {
        assert_close("cornering T_s", *got, want, 0.0, 1e-4);
    }
    assert_close("cornering T_c", corner.t_c, 20.001161, 0.0, 1e-4);
    assert_close("cornering w", corner.wear, 4.048e-5, 1e-3, 1e-8);
    let mut traction = ThermalState::seed(&p, 20.0, 20.0);
    traction.step(&p, &tick(1.0 / 60.0, 3750.0, 1.25, 0.0));
    for (got, want) in traction.t_s.iter().zip([20.009566, 20.003916, 20.009566]) {
        assert_close("traction T_s", *got, want, 0.0, 1e-4);
    }
    assert_close("traction T_c", traction.t_c, 20.001161, 0.0, 1e-4);
    assert_close("traction w", traction.wear, 2.5e-5, 1e-3, 1e-8);
}

#[test]
fn t8_one_tick_in_the_air() {
    let p = params();
    let mut state = ThermalState { t_s: [60.0; 3], t_c: 50.0, wear: 0.0, p_set: 190.0, t_set: 20.0 };
    let air = ThermalTick { loaded: false, p_slide: 0.0, v_slide: 0.0, ..tick(1.0 / 60.0, 0.0, 0.0, 0.0) };
    state.step(&p, &air);
    for got in state.t_s {
        assert_close("airborne T_s", got, 59.990556, 0.0, 1e-4);
    }
    assert_close("airborne T_c", state.t_c, 50.001162, 0.0, 1e-4);
    assert_eq!(state.wear, 0.0);
}

#[test]
fn t9_one_tick_hot() {
    let p = params();
    let mut state = ThermalState { t_s: [110.0; 3], t_c: 100.0, wear: 0.0, p_set: 190.0, t_set: 20.0 };
    let hot = ThermalTick { t_road: 25.0, ..tick(1.0 / 60.0, 7000.0, 2.0, 0.0) };
    state.step(&p, &hot);
    for (got, want) in state.t_s.iter().zip([109.98743, 109.996707, 109.98743]) {
        assert_close("hot T_s", *got, want, 0.0, 1e-3);
    }
    assert_close("hot T_c", state.t_c, 100.001573, 0.0, 1e-3);
    assert_close("hot w (m_hot 1.2)", state.wear, 4.5e-5, 1e-3, 1e-8);
}

/// The values of a tyre at one time of a long run: `(T_s, T_c, P, T_cp, T_eff, grip_scale, w)`.
fn row(p: &ThermalParams, state: &ThermalState, gamma: f64) -> ([f64; 3], f64, f64, f64, f64, f64, f64) {
    (
        state.t_s,
        state.t_c,
        state.pressure(p),
        state.t_cp(p, gamma),
        state.t_eff(p, gamma),
        state.grip(p, gamma),
        state.wear,
    )
}

#[test]
fn t10_warm_up_in_a_sustained_circle_and_t11_cool_down() {
    let p = params();
    let mut state = ThermalState::seed(&p, 20.0, 20.0);
    let circle = tick(1.0 / 60.0, 7626.12, 2.17889, -0.035);
    // t, T_s, T_c, P, T_cp, T_eff, grip_scale, w
    let warm = [
        (10, [31.267, 24.139, 26.106], 20.989, 190.98, 28.622, 22.897, 0.8138, 0.02429),
        (60, [55.284, 40.548, 41.621], 30.038, 199.98, 48.565, 34.670, 0.8833, 0.14573),
        (180, [69.648, 60.240, 56.422], 51.171, 220.98, 63.683, 54.299, 0.9682, 0.43720),
        (360, [79.104, 75.244, 67.517], 69.407, 239.10, 75.176, 70.849, 0.9657, 0.87440),
        (600, [84.664, 84.055, 74.040], 80.151, 249.78, 82.295, 80.687, 0.9404, 1.45734),
        (900, [87.105, 87.916, 76.901], 84.862, 254.46, 85.503, 85.023, 0.9166, 2.18600),
    ];
    let mut second = 0;
    for (t, ts, tc, pressure, t_cp, t_eff, grip, wear) in warm {
        while second < t * 60 {
            state.step(&p, &circle);
            second += 1;
        }
        let (got_ts, got_tc, got_p, got_cp, got_eff, got_grip, got_w) = row(&p, &state, -0.035);
        for i in 0..3 {
            assert_close(&format!("t {t} T_s[{i}]"), got_ts[i], ts[i], 0.0, 0.1);
        }
        assert_close(&format!("t {t} T_c"), got_tc, tc, 0.0, 0.1);
        assert_close(&format!("t {t} P"), got_p, pressure, 0.0, 0.2);
        assert_close(&format!("t {t} T_cp"), got_cp, t_cp, 0.0, 0.1);
        assert_close(&format!("t {t} T_eff"), got_eff, t_eff, 0.0, 0.1);
        assert_close(&format!("t {t} grip"), got_grip, grip, 0.0, 0.002);
        assert_close(&format!("t {t} w"), got_w, wear, 0.005, 1e-6);
    }
    // T11: the car stands, from the state of t = 900.
    let rest = ThermalTick { v: 0.0, p_slide: 0.0, v_slide: 0.0, t_road: 25.0, ..circle };
    let cool = [
        (10, [84.196, 83.929, 76.803], 84.496, 254.09, 84.036),
        (60, [74.784, 74.272, 73.309], 81.091, 250.71, 79.387),
        (180, [64.311, 64.296, 64.280], 71.216, 240.90, 69.487),
        (360, [53.836; 3], 59.029, 228.79, 57.731),
        (600, [43.892; 3], 47.393, 217.22, 46.517),
        (1800, [25.850; 3], 26.281, 196.24, 26.173),
    ];
    let mut second = 0;
    for (t, ts, tc, pressure, t_eff) in cool {
        while second < t * 60 {
            state.step(&p, &rest);
            second += 1;
        }
        for i in 0..3 {
            assert_close(&format!("cool {t} T_s[{i}]"), state.t_s[i], ts[i], 0.0, 0.1);
        }
        assert_close(&format!("cool {t} T_c"), state.t_c, tc, 0.0, 0.1);
        assert_close(&format!("cool {t} P"), state.pressure(&p), pressure, 0.0, 0.2);
        assert_close(&format!("cool {t} T_eff"), state.t_eff(&p, -0.035), t_eff, 0.0, 0.1);
        assert_close(&format!("cool {t} w"), state.wear, 2.186, 0.005, 1e-6);
    }
}

#[test]
fn t12_wear_to_the_end_of_life() {
    let p = params();
    let mut state = ThermalState::seed(&p, 60.0, 20.0);
    let drive = ThermalTick { v_slide: 3.0, ..tick(1.0 / 60.0, 0.0, 3.0, 0.0) };
    let mut ticks = 0_u32;
    while state.wear < 10.0 {
        state.t_s = [60.0; 3];
        state.t_c = 60.0;
        state.step(&p, &drive);
        ticks += 1;
    }
    assert!((i64::from(ticks) - 184_616).abs() <= 2, "{ticks} ticks");
    assert_close("g_W at 10 km", p.g_w(state.wear), 0.72, 0.0, 1e-6);
    state.step(&p, &drive);
    assert_close("g_W beyond", p.g_w(state.wear), 0.72, 0.0, 1e-6);
}

#[test]
fn a_tick_longer_than_half_a_second_is_split_and_agrees_with_the_small_steps() {
    let p = params();
    let mut one = ThermalState::seed(&p, 30.0, 20.0);
    let mut many = one.clone();
    let long = ThermalTick { dt: 2.0, ..tick(1.0, 4000.0, 2.0, 0.0) };
    one.step(&p, &long);
    let short = ThermalTick { dt: 0.5, ..long };
    for _ in 0..4 {
        many.step(&p, &short);
    }
    assert_eq!(one, many);
}

#[test]
fn properties_of_the_thermal_model() {
    let p = params();
    // grip_scale is in (0, 1]; P is monotonic in the core temperature.
    let mut last = f64::NEG_INFINITY;
    for core in -20..160 {
        let pressure = p.pressure(190.0, 20.0, f64::from(core));
        assert!(pressure > last);
        last = pressure;
        for wear in [0.0, 3.0, 12.0] {
            let grip = p.grip_scale(f64::from(core), pressure, wear);
            assert!(grip > 0.0 && grip <= 1.0, "{grip}");
        }
    }
    // With no input and equal air and road, every temperature converges to the ambient one, never overshooting.
    let mut state = ThermalState::seed(&p, 90.0, 20.0);
    let rest = ThermalTick {
        loaded: true,
        v: 0.0,
        p_slide: 0.0,
        v_slide: 0.0,
        t_road: 20.0,
        ..tick(1.0 / 60.0, 0.0, 0.0, 0.0)
    };
    let mut previous = f64::INFINITY;
    for _ in 0..(3600 * 60 * 2) {
        state.step(&p, &rest);
        assert!(state.t_c >= 20.0 - 1e-9 && state.t_c <= previous + 1e-9);
        previous = state.t_c;
    }
    assert!((state.t_c - 20.0).abs() < 0.1 && state.t_s.iter().all(|t| (t - 20.0).abs() < 0.1));
    // The core never reacts faster than the stripes at the start of a heating.
    let mut heated = ThermalState::seed(&p, 20.0, 20.0);
    heated.step(&p, &tick(1.0 / 60.0, 8000.0, 2.0, 0.0));
    assert!(heated.t_s[1] - 20.0 > heated.t_c - 20.0);
}

#[test]
fn the_seam_resets_cold_or_warm_and_the_off_state_answers_one() {
    let mut cold = ThermalGrip::new(params(), false);
    cold.reset(15.0);
    assert_eq!(cold.state(0).t_c, 15.0);
    assert_eq!(cold.state(0).t_set, 15.0);
    let mut warm = ThermalGrip::new(params(), true);
    warm.reset(15.0);
    assert_eq!(warm.state(3).t_c, 70.0);
    assert!(warm.state(3).pressure(&params()) > 230.0, "a warm start begins with a pressure above the cold one");
    // A cold tyre grips less than a warm one; the factor is held through grip_scale and moved by end_tick.
    let inputs = GripInputs { camber_rad: -0.035 };
    assert!(cold.grip_scale(0, &inputs) < warm.grip_scale(0, &inputs));
    assert_eq!(NoThermal.grip_scale(0, &inputs), 1.0);
    assert!(NoThermal.hud(0, 0.0).is_none());
    let hud = warm.hud(0, 0.0).expect("a thermal model has HUD values");
    assert_close("hud life", hud.life, 100.0, 0.0, 1e-9);
    cold.end_tick(0, &tick(1.0 / 60.0, 8000.0, 2.0, 0.0));
    assert!(cold.state(0).t_c > 15.0 && cold.state(1).t_c == 15.0);
}
