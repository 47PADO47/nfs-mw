//! The test vectors of docs/specs/ac-handling/wheel-dynamics.md section 6 (revision of 2026-11-20): one substep with every
//! intermediate, the whole ticks W1 to W9b, the `inertia_scale 0.3` check, the substep convergence W10, and the properties.
//! Common inputs: `R = 0.33`, host inertia 10 with the default `inertia_scale` 1 (so `I = 10`), `dt = 1/60`, the `sport` tyre with
//! a 0.225 m tyre, `corner_mass_kg = Fz / 9.81` (350 in the airborne rows), fresh state at the first call. Tolerances of the
//! spec: forces relative 1e-3 or 0.5 N, spin 1e-3 rad/s, slip 1e-3 m/s, traction 1e-3, slip angle 1e-5 turns.

use super::assert_close;
use crate::keys::Keys;
use crate::params::TyreParams;
use crate::substep::Frame;
use crate::wheel::{WheelInput, WheelOutput, WheelState, substep_count, tick_n, valid, wheel_tick};

const R: f64 = 0.33;

fn input(fz: f64, vx: f64, vy: f64, spin: f64, drive: f64, brake: f64) -> WheelInput {
    WheelInput {
        fz,
        vx,
        vy,
        spin,
        drive_torque: drive,
        brake_torque: brake,
        corner_mass: fz / 9.81,
        ..WheelInput::default()
    }
}

/// `ticks` ticks from a fresh wheel on `params`, the spin carried as the host would; the output of every tick.
fn run_with(p: &TyreParams, base: &WheelInput, ticks: usize, n: Option<u32>) -> Vec<WheelOutput> {
    let mut state = WheelState::fresh();
    let mut current = *base;
    let mut out = Vec::new();
    for _ in 0..ticks {
        let result = match n {
            Some(n) => tick_n(p, &mut state, &current, n),
            None => wheel_tick(p, &mut state, &current, 0),
        };
        let result = result.expect("handled");
        current.spin = result.spin;
        out.push(result);
    }
    out
}

fn run_ticks(base: &WheelInput, ticks: usize, n: Option<u32>) -> Vec<WheelOutput> {
    run_with(&TyreParams::default(), base, ticks, n)
}

/// The light wheel of the first draft: `inertia_scale 0.3` (`I = 3`).
fn light() -> TyreParams {
    TyreParams::new(Keys { inertia_scale: 0.3, ..Keys::default() }, 0.225)
}

#[track_caller]
fn tick(name: &str, got: &WheelOutput, want: [f64; 7]) {
    assert_close(&format!("{name} force_long"), got.fx, want[0], 1e-3, 0.5);
    assert_close(&format!("{name} force_lat"), got.fy, want[1], 1e-3, 0.5);
    assert_close(&format!("{name} aligning"), got.mz, want[2], 1e-3, 0.05);
    assert_close(&format!("{name} wheel_spin"), got.spin, want[3], 0.0, 1e-3);
    assert_close(&format!("{name} traction"), got.traction, want[4], 0.0, 1e-3);
    assert_close(&format!("{name} slip"), got.slip, want[5], 0.0, 1e-3);
    assert_close(&format!("{name} slip_angle"), got.slip_angle, want[6], 0.0, 1e-5);
}

/// The first substep of a fresh wheel, as section 4.8 does it: seed, then one step of `h = 1/600`.
fn one_substep(i: &WheelInput) -> (Frame<'static>, WheelState, f64, crate::tyre_force::ForceOut) {
    let p: &'static TyreParams = Box::leak(Box::new(TyreParams::default()));
    let i: &'static WheelInput = Box::leak(Box::new(*i));
    let frame = Frame { p, i, inertia: 10.0, speed: i.vx.hypot(i.vy), h: 1.0 / 600.0 };
    let slips = frame.slips(i.spin);
    let mut state = WheelState { sx_r: slips.sx, sy_r: slips.sy, n_prev: 0.0, fresh: false };
    let (spin, end) = frame.step(&mut state, i.spin);
    (frame, state, spin, end)
}

#[test]
fn one_substep_of_the_brake_case() {
    let i = input(4000.0, 30.0, 0.0, 90.9091, 0.0, 2500.0);
    let (frame, state, spin, end) = one_substep(&i);
    let slips = frame.slips(i.spin);
    assert_close("Wx", slips.wx, 30.0, 1e-5, 1e-5);
    assert_close("a", frame.relaxation(0.0, R * i.spin), 0.101_626, 1e-4, 1e-6);
    assert_close("S", frame.slope(0.0, 0.0, i.spin), 119_995.69, 1e-4, 1.0);
    assert_close("w'", spin, 90.489_487, 0.0, 1e-3);
    assert_close("sx_r'", state.sx_r, -0.000_469, 0.0, 1e-6);
    assert_close("sy_r'", state.sy_r, 0.0, 0.0, 1e-9);
    assert_close("F_end fx", end.fx, -56.5260, 1e-3, 0.5);
    assert_close("n", end.n, 0.00389, 1e-3, 1e-5);
}

#[test]
fn one_substep_of_the_launch_case() {
    let i = input(4500.0, 0.0, 0.0, 0.0, 1800.0, 0.0);
    let (frame, state, spin, end) = one_substep(&i);
    let slips = frame.slips(0.0);
    assert_close("Wx", slips.wx, 1.0, 0.0, 1e-9);
    assert_close("a", frame.relaxation(0.0, 0.0), 0.020_408, 1e-4, 1e-6);
    assert_close("S", frame.slope(0.0, 0.0, 0.0), 127_819.09, 1e-4, 1.0);
    assert_close("w'", spin, 0.283_469, 0.0, 1e-3);
    assert_close("sx_r'", state.sx_r, 0.001_909, 0.0, 1e-6);
    assert_close("F_end fx", end.fx, 242.2207, 1e-3, 0.5);
    assert_close("n", end.n, 0.01521, 1e-3, 1e-5);
}

#[test]
fn one_substep_of_the_cornering_case_with_drive() {
    let i = input(4000.0, 20.0, 3.0, 60.6061, 600.0, 0.0);
    let (frame, state, spin, end) = one_substep(&i);
    let slips = frame.slips(i.spin);
    assert_close("sy_i", slips.sy, 0.15, 1e-5, 1e-6);
    assert_close("a", frame.relaxation(0.0, R * i.spin), 0.070_855, 1e-4, 1e-6);
    let f0 = frame.force_at(0.0, 0.15, i.spin);
    assert_close("F0 fy", f0.fy, -4304.6562, 1e-4, 0.5);
    assert_close("S", frame.slope(0.0, 0.15, i.spin), 45_121.19, 1e-4, 1.0);
    assert_close("w'", spin, 60.702_999, 0.0, 1e-3);
    assert_close("sx_r'", state.sx_r, 0.000_113, 0.0, 1e-6);
    assert_close("sy_r'", state.sy_r, 0.15, 0.0, 1e-6);
    assert_close("F_end fx", end.fx, 5.1139, 1e-3, 0.5);
    assert_close("F_end fy", end.fy, -4304.6530, 1e-4, 0.5);
    assert_close("F_end mz", end.mz, 12.8347, 1e-3, 0.05);
    assert_close("n", end.n, 0.88942, 1e-3, 1e-5);
}

#[test]
fn w1_free_rolling_and_w5_standstill() {
    let rolling = input(4000.0, 30.0, 0.0, 30.0 / R, 0.0, 0.0);
    tick("W1", &run_ticks(&rolling, 1, None)[0], [-5.903, 0.0, 0.0, 90.883, 1.0, -0.0086, 0.0]);
    let standing = input(4000.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    tick("W5", &run_ticks(&standing, 1, None)[0], [0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
}

#[test]
fn w2_launch() {
    let launch = input(4500.0, 0.0, 0.0, 0.0, 1800.0, 0.0);
    let ticks = run_ticks(&launch, 6, None);
    tick("W2 tick 1", &ticks[0], [2452.414, 0.0, 0.0, 1.468, 1.000, 0.485, 0.0]);
    tick("W2 tick 2", &ticks[1], [5256.930, 0.0, 0.0, 1.520, 0.857, 0.501, 0.0]);
    tick("W2 tick 6", &ticks[5], [4639.845, 0.0, 0.0, 2.834, 0.267, 0.935, 0.0]);
}

#[test]
fn w3_brake_and_w4_lock_up() {
    let brake = input(4000.0, 30.0, 0.0, 30.0 / R, 0.0, 2500.0);
    let ticks = run_ticks(&brake, 12, None);
    tick("W3 tick 1", &ticks[0], [-792.789, 0.0, 0.0, 87.176, 1.000, -1.232, 0.0]);
    tick("W3 tick 2", &ticks[1], [-3031.551, 0.0, 0.0, 84.685, 1.000, -2.054, 0.0]);
    tick("W3 tick 3", &ticks[2], [-4358.273, 0.0, 0.0, 82.900, 1.000, -2.643, 0.0]);
    tick("W3 tick 6", &ticks[5], [-4674.815, 0.0, 0.0, 78.171, 0.792, -4.203, 0.0]);
    tick("W3 tick 12", &ticks[11], [-4215.627, 0.0, 0.0, 67.380, 0.363, -7.764, 0.0]);
    let lock = input(4000.0, 30.0, 0.0, 30.0 / R, 0.0, 6000.0);
    let ticks = run_ticks(&lock, 12, None);
    tick("W4 tick 1", &ticks[0], [-1709.085, 0.0, 0.0, 81.876, 1.000, -2.981, 0.0]);
    tick("W4 tick 2", &ticks[1], [-4588.763, 0.0, 0.0, 74.407, 0.739, -5.446, 0.0]);
    tick("W4 tick 3", &ticks[2], [-4354.903, 0.0, 0.0, 66.783, 0.408, -7.962, 0.0]);
    tick("W4 tick 8", &ticks[7], [-3940.644, 0.0, 0.0, 27.748, 0.063, -20.843, 0.0]);
    tick("W4 tick 12", &ticks[11], [-3850.839, 0.0, 0.0, 0.0, 0.004, -30.0, 0.0]);
}

#[test]
fn the_light_wheel_of_inertia_scale_0_3_is_still_honoured() {
    let p = light();
    let launch = run_with(&p, &input(4500.0, 0.0, 0.0, 0.0, 1800.0, 0.0), 1, None);
    tick("I=3 W2 tick 1", &launch[0], [3733.630, 0.0, 0.0, 2.396, 0.972, 0.791, 0.0]);
    let brake = run_with(&p, &input(4000.0, 30.0, 0.0, 30.0 / R, 0.0, 2500.0), 1, None);
    tick("I=3 W3 tick 1", &brake[0], [-2059.031, 0.0, 0.0, 80.872, 1.000, -3.312, 0.0]);
    let lock = run_with(&p, &input(4000.0, 30.0, 0.0, 30.0 / R, 0.0, 6000.0), 8, None);
    tick("I=3 W4 tick 1", &lock[0], [-3173.431, 0.0, 0.0, 63.562, 0.539, -9.024, 0.0]);
    tick("I=3 W4 tick 8", &lock[7], [-3848.027, 0.0, 0.0, 0.0, 0.004, -30.0, 0.0]);
}

#[test]
fn w6_creeping_sideways_is_held_by_the_momentum_limit() {
    let cases =
        [(0.01, 122.324, 1.000, 0.003_978), (0.05, 611.621, 1.000, 0.019_792), (0.3, 3669.725, 0.225, 0.102_416)];
    for (vy, cap, traction, angle) in cases {
        let creep = input(4000.0, 0.0, vy, 0.0, 0.0, 0.0);
        let got = run_ticks(&creep, 1, None)[0];
        tick(&format!("W6 Vy {vy}"), &got, [0.0, -cap, got.mz, 0.0, traction, 0.0, angle]);
    }
}

#[test]
fn w7_the_airborne_wheel_decays_and_locks() {
    // The airborne rows send corner_mass_kg = 350: a zero mass would make the call not handled.
    let free = WheelInput { loaded: false, fz: 0.0, corner_mass: 350.0, ..input(0.0, 0.0, 0.0, 60.0, 0.0, 0.0) };
    let a = run_ticks(&free, 1, None)[0];
    tick("W7a", &a, [0.0, 0.0, 0.0, 59.995, 0.0, 0.0, 0.0]);
    let braked = WheelInput { brake_torque: 300.0, ..free };
    assert_close("W7b spin", run_ticks(&braked, 1, None)[0].spin, 59.495, 0.0, 1e-3);
    let slow = WheelInput { spin: 0.5, brake_torque: 300.0, ..free };
    assert_close("W7c spin", run_ticks(&slow, 1, None)[0].spin, 0.0, 0.0, 1e-9);
}

#[test]
fn w8_cornering_builds_a_steady_lateral_force() {
    let corner = input(4000.0, 20.0, 3.0, 20.0 / R, 0.0, 0.0);
    let ticks = run_ticks(&corner, 30, None);
    tick("W8 tick 1", &ticks[0], [-3.577, -4304.663, 12.828, 60.580, 1.0, -0.0085, 0.023_701]);
    tick("W8 tick 2", &ticks[1], [-16.273, -4304.677, 12.801, 60.562, 1.0, -0.0146, 0.023_708]);
    tick("W8 tick 5", &ticks[4], [-44.353, -4304.616, 12.732, 60.541, 1.0, -0.0216, 0.023_720]);
    tick("W8 tick 30", &ticks[29], [-50.644, -4304.584, 12.715, 60.538, 1.0, -0.0224, 0.023_723]);
}

#[test]
fn w9_reversing_mirrors_forward_driving() {
    let brake = input(4000.0, -30.0, 0.0, -30.0 / R, 0.0, 2500.0);
    tick("W9", &run_ticks(&brake, 1, None)[0], [792.789, 0.0, 0.0, -87.176, 1.0, 1.232, 0.0]);
    let launch = input(4500.0, 0.0, 0.0, 0.0, -1800.0, 0.0);
    tick("W9b", &run_ticks(&launch, 1, None)[0], [-2452.414, 0.0, 0.0, -1.468, 1.0, -0.485, 0.0]);
}

/// One row of the W10 table: `(substeps, fx, spin)` for the case.
struct Convergence {
    name: &'static str,
    base: WheelInput,
    tick: usize,
    rows: &'static [(u32, f64, f64)],
}

#[test]
fn w10_substeps_converge_to_the_table() {
    let cases = [
        Convergence {
            name: "brake onset",
            base: input(4000.0, 30.0, 0.0, 90.909, 0.0, 2500.0),
            tick: 1,
            rows: &[
                (1, -1199.17, 86.7131),
                (2, -1003.07, 87.0892),
                (5, -846.06, 87.1896),
                (10, -792.79, 87.1755),
                (20, -766.66, 87.1542),
                (40, -753.80, 87.1391),
                (96, -746.37, 87.1288),
            ],
        },
        Convergence {
            name: "launch tick 1",
            base: input(4500.0, 0.0, 0.0, 0.0, 1800.0, 0.0),
            tick: 1,
            rows: &[
                (1, 1596.09, 0.5703),
                (2, 2203.26, 0.9008),
                (5, 2456.14, 1.2905),
                (10, 2452.41, 1.4684),
                (20, 2424.93, 1.5641),
                (40, 2404.07, 1.6130),
                (96, 2389.63, 1.6417),
            ],
        },
        Convergence {
            name: "launch tick 6",
            base: input(4500.0, 0.0, 0.0, 0.0, 1800.0, 0.0),
            tick: 6,
            rows: &[
                (1, 5385.19, 0.5323),
                (2, 4901.57, 1.3843),
                (5, 4681.26, 2.4610),
                (10, 4639.85, 2.8338),
                (20, 4623.39, 3.0144),
                (40, 4616.10, 3.1024),
                (96, 4612.14, 3.1527),
            ],
        },
        Convergence {
            name: "cornering tick 1",
            base: input(4000.0, 20.0, 3.0, 60.606, 0.0, 0.0),
            tick: 1,
            rows: &[
                (1, -4.46, 0.0),
                (2, -3.83, 0.0),
                (5, -3.70, 0.0),
                (10, -3.58, 0.0),
                (20, -3.49, 0.0),
                (40, -3.44, 0.0),
                (96, -3.40, 0.0),
            ],
        },
        Convergence {
            name: "lock-up tick 3",
            base: input(4000.0, 30.0, 0.0, 90.909, 0.0, 6000.0),
            tick: 3,
            rows: &[
                (1, -4418.05, 65.9910),
                (2, -4387.30, 66.7368),
                (5, -4364.55, 66.8547),
                (10, -4354.90, 66.7829),
                (20, -4349.39, 66.7161),
                (40, -4346.41, 66.6736),
                (96, -4344.60, 66.6458),
            ],
        },
    ];
    for case in &cases {
        for (n, fx, spin) in case.rows {
            let got = run_ticks(&case.base, case.tick, Some(*n))[case.tick - 1];
            let name = format!("{} N={n}", case.name);
            assert_close(&format!("{name} fx"), got.fx, *fx, 1e-3, 0.5);
            if *spin != 0.0 {
                assert_close(&format!("{name} spin"), got.spin, *spin, 0.0, 2e-3);
            }
        }
    }
}

#[test]
fn w10_the_automatic_substep_count_is_inside_the_acceptance_bounds() {
    // The cases of the spec's table, at its ticks: 2.5 % of the peak force (120 N) from N = 96, 1.5 % (72 N) from N to 2N, and
    // 0.4 m/s of wheel speed. The spec's worst cases at N = 10: 62.8 N, 27.5 N and 0.11 m/s.
    let cases = [
        (input(4000.0, 30.0, 0.0, 90.909, 0.0, 2500.0), 1),
        (input(4500.0, 0.0, 0.0, 0.0, 1800.0, 0.0), 1),
        (input(4500.0, 0.0, 0.0, 0.0, 1800.0, 0.0), 6),
        (input(4000.0, 20.0, 3.0, 60.606, 0.0, 0.0), 1),
        (input(4000.0, 30.0, 0.0, 90.909, 0.0, 6000.0), 3),
    ];
    for (base, tick) in cases {
        let auto_n = substep_count(base.dt, 0);
        let at_n = run_ticks(&base, tick, Some(auto_n))[tick - 1];
        let doubled = run_ticks(&base, tick, Some(2 * auto_n))[tick - 1];
        let reference = run_ticks(&base, tick, Some(96))[tick - 1];
        assert!((at_n.fx - reference.fx).abs() <= 120.0, "N to 96: {}", at_n.fx - reference.fx);
        assert!((at_n.fx - doubled.fx).abs() <= 72.0, "N to 2N: {}", at_n.fx - doubled.fx);
        assert!(R * (at_n.spin - reference.spin).abs() <= 0.4, "spin at tick {tick}");
    }
}

#[test]
fn w10_the_other_host_rates_are_inside_the_bound_too() {
    // dt = 1/144 (N = 5) and 1/240 (N = 3), brake onset tick 1.
    let rows =
        [(1.0 / 144.0, 5, -199.08, 89.2093, -163.01, 89.1986), (1.0 / 240.0, 3, -89.18, 89.8720, -62.07, 89.8687)];
    for (dt, n, fx, spin, fx_ref, spin_ref) in rows {
        let base = WheelInput { dt, ..input(4000.0, 30.0, 0.0, 90.909, 0.0, 2500.0) };
        assert_eq!(substep_count(dt, 0), n);
        let got = run_ticks(&base, 1, None)[0];
        assert_close("fx", got.fx, fx, 1e-3, 0.5);
        assert_close("spin", got.spin, spin, 0.0, 2e-3);
        let reference = run_ticks(&base, 1, Some(96))[0];
        assert_close("fx 96", reference.fx, fx_ref, 1e-3, 0.5);
        assert_close("spin 96", reference.spin, spin_ref, 0.0, 2e-3);
        assert!((got.fx - reference.fx).abs() <= 120.0);
    }
}

#[test]
fn the_substep_count_rule() {
    let rows = [(1.0 / 60.0, 10), (1.0 / 144.0, 5), (1.0 / 240.0, 3), (1.0 / 30.0, 20), (0.1, 24), (1.0 / 1000.0, 1)];
    for (dt, n) in rows {
        assert_eq!(substep_count(dt, 0), n, "dt {dt}");
    }
    assert_eq!(substep_count(1.0 / 60.0, 4), 4, "the setting overrides");
    assert_eq!(substep_count(1.0 / 60.0, 99), 24, "and is capped");
}

#[test]
fn not_handled_rules_leave_the_wheel_fresh() {
    let p = TyreParams::default();
    let good = input(4000.0, 20.0, 1.0, 60.0, 0.0, 0.0);
    let bad = [
        WheelInput { corner_mass: 0.0, ..good },
        WheelInput { corner_mass: -3.0, ..good },
        WheelInput { dt: 0.0, ..good },
        WheelInput { dt: 0.5, ..good },
        WheelInput { radius: 0.01, ..good },
        WheelInput { inertia: 0.0, ..good },
        WheelInput { fz: -1.0, ..good },
        WheelInput { vx: f64::NAN, ..good },
        WheelInput { spin: f64::INFINITY, ..good },
        WheelInput { grip: f64::NAN, ..good },
    ];
    for query in bad {
        assert!(!valid(&query));
        let mut state = WheelState { sx_r: 0.1, sy_r: 0.1, n_prev: 0.5, fresh: false };
        assert!(wheel_tick(&p, &mut state, &query, 0).is_none());
        assert!(state.fresh, "a not-handled call marks the wheel fresh");
    }
    assert!(valid(&good));
    let mut state = WheelState::fresh();
    assert!(wheel_tick(&p, &mut state, &good, 0).is_some());
    assert!(!state.fresh);
}

#[test]
fn an_airborne_tick_leaves_the_wheel_fresh_so_touch_down_starts_from_an_unloaded_patch() {
    let p = TyreParams::default();
    let mut state = WheelState::fresh();
    let ground = input(4000.0, 20.0, 3.0, 20.0 / R, 0.0, 0.0);
    wheel_tick(&p, &mut state, &ground, 0).expect("handled");
    assert!(!state.fresh);
    let air = WheelInput { loaded: false, fz: 0.0, vx: 0.0, vy: 0.0, ..ground };
    wheel_tick(&p, &mut state, &air, 0).expect("handled");
    assert!(state.fresh);
    assert_eq!((state.sx_r, state.sy_r, state.n_prev), (0.0, 0.0, 0.0));
}

#[test]
fn property_the_outputs_stay_finite_over_the_whole_envelope() {
    let p = TyreParams::default();
    let mut state = WheelState::fresh();
    let mut seed = 12345_u64;
    let mut next = move || {
        seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        ((seed >> 33) as f64) / f64::from(1_u32 << 31)
    };
    for step in 0..2000 {
        let fz = if step % 7 == 0 { 0.0 } else { next() * 9000.0 };
        let query = WheelInput {
            fz,
            loaded: fz > 0.0,
            vx: (next() - 0.5) * 120.0,
            vy: (next() - 0.5) * 120.0,
            spin: (next() - 0.5) * 500.0,
            drive_torque: (next() - 0.5) * 30_000.0,
            brake_torque: next() * 15_000.0,
            dt: [1.0 / 60.0, 1.0 / 144.0, 1.0 / 240.0][step % 3],
            inertia: 1.2 + next() * 28.0,
            corner_mass: 100.0 + next() * 800.0,
            ..WheelInput::default()
        };
        let out = wheel_tick(&p, &mut state, &query, 0).expect("handled");
        let all = [out.fx, out.fy, out.mz, out.spin, out.traction, out.slip, out.slip_angle];
        assert!(all.iter().all(|value| value.is_finite()), "step {step}: {out:?}");
        assert!((0.0..=1.0).contains(&out.traction));
    }
}

#[test]
fn property_mirroring_the_longitudinal_motion_mirrors_the_forces() {
    let p = TyreParams::default();
    let forward = input(4000.0, 25.0, 2.0, 25.0 / R, 900.0, 300.0);
    let mirrored = WheelInput { vx: -forward.vx, spin: -forward.spin, drive_torque: -forward.drive_torque, ..forward };
    let (mut a, mut b) = (WheelState::fresh(), WheelState::fresh());
    let one = wheel_tick(&p, &mut a, &forward, 0).expect("handled");
    let two = wheel_tick(&p, &mut b, &mirrored, 0).expect("handled");
    assert_close("fx", two.fx, -one.fx, 1e-4, 1e-3);
    assert_close("spin", two.spin, -one.spin, 1e-4, 1e-3);
    assert_close("fy", two.fy, one.fy, 1e-4, 1e-3);
    assert_close("mz", two.mz, one.mz, 1e-4, 1e-3);
}

#[test]
fn property_a_standing_wheel_stays_put_and_a_braked_wheel_locks_and_stays_locked() {
    let p = TyreParams::default();
    let mut state = WheelState::fresh();
    let standing = input(4000.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    for _ in 0..30 {
        let out = wheel_tick(&p, &mut state, &standing, 0).expect("handled");
        assert_eq!((out.fx, out.fy, out.spin), (0.0, 0.0, 0.0));
    }
    let lock = input(4000.0, 30.0, 0.0, 30.0 / R, 0.0, 6000.0);
    let ticks = run_ticks(&lock, 24, None);
    assert!(ticks[12..].iter().all(|out| out.spin == 0.0));
}
