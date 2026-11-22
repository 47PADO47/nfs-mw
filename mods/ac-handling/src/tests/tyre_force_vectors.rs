//! The test vectors T1 to T14 of docs/specs/ac-handling/tyre-force.md section 6, and its properties.
//! Common inputs: radius 0.33, the `sport` defaults with a 0.225 m tyre, camber 0, surfaces and grip 1, slide speed 0, speed 20.
//! Tolerance of the spec: relative 1e-4, absolute 1e-3 on forces and on `mz`; `n` is given to three decimals.

use super::assert_close;
use crate::params::TyreParams;
use crate::tyre_force::{ForceIn, ForceOut, curve, tyre_force};
use crate::wheel::WheelInput;

fn run(p: &TyreParams, input: ForceIn) -> ForceOut {
    tyre_force(p, &input)
}

fn at(fz: f64, sx: f64, sy: f64) -> ForceIn {
    ForceIn { fz, sx, sy, ..ForceIn::default() }
}

/// Check one row of the table.
#[track_caller]
fn row(name: &str, got: ForceOut, fx: f64, fy: f64, mz: f64, n: f64) {
    assert_close(&format!("{name} fx"), got.fx, fx, 1e-4, 1e-3);
    assert_close(&format!("{name} fy"), got.fy, fy, 1e-4, 1e-3);
    assert_close(&format!("{name} mz"), got.mz, mz, 1e-4, 1e-3);
    assert_close(&format!("{name} n"), got.n, n, 1e-4, 1e-3);
}

#[test]
fn t1_zero_load_and_t2_zero_slip_give_zero() {
    let p = TyreParams::default();
    row("T1", run(&p, at(0.0, 0.05, 0.05)), 0.0, 0.0, 0.0, 0.0);
    row("T1 negative load", run(&p, at(-5.0, 0.05, 0.05)), 0.0, 0.0, 0.0, 0.0);
    row("T2", run(&p, at(4000.0, 0.0, 0.0)), 0.0, 0.0, 0.0, 0.0);
}

#[test]
fn t3_lateral_before_the_peak_and_its_mirror() {
    let p = TyreParams::default();
    let input = ForceIn { slide_speed: 1.0, ..at(4000.0, 0.0, 0.05) };
    row("T3", run(&p, input), 0.0, -2816.267, 58.172, 0.296);
    let mirror = ForceIn { slide_speed: 1.0, ..at(4000.0, 0.0, -0.05) };
    row("T3m", run(&p, mirror), 0.0, 2816.267, -58.172, 0.296);
}

#[test]
fn t4_lateral_at_the_peak_and_t5_far_past_it() {
    let p = TyreParams::default();
    row("T4", run(&p, at(4000.0, 0.0, 0.168_649_001_6)), 0.0, -4326.002, 1.525, 1.000);
    row("T5", run(&p, at(4000.0, 0.0, 0.6)), 0.0, -3836.567, -9.393, 3.558);
}

#[test]
fn t6_drive_brake_and_lock_up() {
    let p = TyreParams::default();
    row("T6a", run(&p, at(4000.0, 0.05, 0.0)), 3886.208, 0.0, 0.0, 0.415);
    row("T6b", run(&p, at(4000.0, 0.15, 0.0)), 4635.104, 0.0, 0.0, 1.245);
    row("T6c", run(&p, at(4000.0, -0.05, 0.0)), -3886.208, 0.0, 0.0, 0.415);
    row("T6d", run(&p, at(4000.0, -5.0, 0.0)), -3999.199, 0.0, 0.0, 41.506);
}

#[test]
fn t6e_brake_gain_scales_only_the_braking_side() {
    let mut keys = p_keys();
    keys.brake_gain = 1.1;
    let p = TyreParams::new(keys, 0.225);
    assert_close("T6e drive", run(&p, at(4000.0, 0.05, 0.0)).fx, 3886.208, 1e-4, 1e-3);
    assert_close("T6e brake", run(&p, at(4000.0, -0.05, 0.0)).fx, -4274.829, 1e-4, 1e-3);
}

fn p_keys() -> crate::keys::Keys {
    crate::keys::Keys::default()
}

#[test]
fn t7_combined_slip_and_the_variants_of_it() {
    let p = TyreParams::default();
    row("T7", run(&p, at(4000.0, 0.06, 0.08)), 3411.249, -2892.681, 26.015, 0.688);
    let surface = ForceIn { surf_lat: 0.6, surf_drive: 0.8, ..at(4000.0, 0.06, 0.08) };
    row("T9", run(&p, surface), 2539.467, -1961.155, 5.514, 0.896);
    let grip = ForceIn { grip: 0.8, ..at(4000.0, 0.06, 0.08) };
    row("T14", run(&p, grip), 2795.081, -2370.181, 12.412, 0.809);
    let mut keys = p_keys();
    keys.comb_exp = 3.0;
    let cubic = TyreParams::new(keys, 0.225);
    row("T13", run(&cubic, at(4000.0, 0.06, 0.08)), 3719.075, -3153.712, 35.971, 0.613);
}

#[test]
fn t8_load_sensitivity_at_the_peak_of_each_load() {
    let p = TyreParams::default();
    let cases = [(2000.0, 0.140_540_8, -2400.000), (4000.0, 0.168_649_0, -4326.002), (6000.0, 0.196_757_2, -6106.106)];
    for (fz, sy, fy) in cases {
        let got = run(&p, at(fz, 0.0, sy));
        assert_close(&format!("T8 fy at {fz}"), got.fy, fy, 1e-4, 1e-3);
        assert_close(&format!("T8 n at {fz}"), got.n, 1.0, 1e-5, 1e-5);
    }
    let ratio =
        (run(&p, at(4000.0, 0.0, 0.168_649_0)).fy / 4000.0) / (run(&p, at(2000.0, 0.0, 0.140_540_8)).fy / 2000.0);
    assert_close("T8 mu ratio", ratio, 2f64.powf(-0.15), 1e-4, 1e-5);
}

#[test]
fn t10_camber_thrust_and_grip() {
    let p = TyreParams::default();
    let case = |gamma: f64, sy: f64| run(&p, ForceIn { gamma, ..at(4000.0, 0.0, sy) });
    let flat = case(0.0, -0.03);
    assert_close("T10 flat fy", flat.fy, 1922.272, 1e-4, 1e-3);
    assert_close("T10 flat n", flat.n, 0.17788, 1e-4, 1e-4);
    let into = case(0.05, -0.03);
    assert_close("T10 into fy", into.fy, 2238.145, 1e-4, 1e-3);
    assert_close("T10 into n", into.n, 0.20752, 1e-4, 1e-4);
    let away = case(-0.05, -0.03);
    assert_close("T10 away fy", away.fy, 1512.503, 1e-4, 1e-3);
    assert_close("T10 away n", away.n, 0.14825, 1e-4, 1e-4);
    let mirror = case(0.05, 0.03);
    assert_close("T10 mirror fy", mirror.fy, -1512.503, 1e-4, 1e-3);
    assert_close("T10 mirror n", mirror.n, 0.14825, 1e-4, 1e-4);
}

#[test]
fn t11_the_aligning_torque_fades_in_at_walking_pace() {
    let p = TyreParams::default();
    let got = run(&p, ForceIn { speed: 0.5, ..at(4000.0, 0.0, 0.02) });
    row("T11", got, 0.0, -1363.754, 16.229, 0.119);
}

#[test]
fn t12_sliding_speed_costs_grip() {
    let p = TyreParams::default();
    row("T12", run(&p, ForceIn { slide_speed: 30.0, ..at(4000.0, 0.0, 0.3) }), 0.0, -3873.087, -9.482, 1.779);
}

#[test]
fn the_curve_has_its_peak_and_its_tail() {
    let want = [(0.0, 0.0), (0.5, 0.875), (1.0, 1.0), (2.0, 0.918_182), (5.0, 0.875_862)];
    for (n, f) in want {
        assert_close(&format!("f({n})"), curve(n, 0.85, 1.2), f, 1e-4, 1e-6);
    }
}

#[test]
fn the_conversions_from_slip_ratio_and_angle_match_the_wheel_slips() {
    // The spec's two conversion examples, computed from the definitions.
    let conv = |kappa: f64, alpha: f64| (kappa / (1.0 + kappa), alpha.tan() / (1.0 + kappa));
    let (sx, sy) = conv(0.1, 0.1);
    assert_close("sx 1", sx, 0.090_909, 1e-4, 1e-6);
    assert_close("sy 1", sy, 0.091_213, 1e-4, 1e-6);
    let (sx, sy) = conv(-0.2, 0.05);
    assert_close("sx 2", sx, -0.25, 1e-4, 1e-6);
    assert_close("sy 2", sy, 0.062_552, 1e-4, 1e-6);
    // The wheel's slips (wheel-dynamics.md 4.1) agree for a wheel that rolls: R w = V (1 + kappa), Vy = V tan(alpha).
    let p = TyreParams::default();
    let (v, kappa, alpha) = (20.0_f64, 0.1_f64, 0.1_f64);
    let input = WheelInput { vx: v, vy: v * alpha.tan(), spin: v * (1.0 + kappa) / 0.33, ..WheelInput::default() };
    let frame = crate::substep::Frame { p: &p, i: &input, inertia: 3.0, speed: 20.0, h: 1.0 / 600.0 };
    let slips = frame.slips(input.spin);
    assert_close("wheel sx", slips.sx, conv(kappa, alpha).0, 1e-9, 1e-9);
    assert_close("wheel sy", slips.sy, conv(kappa, alpha).1, 1e-9, 1e-9);
}

#[test]
fn property_zero_at_rest_and_signs_follow_the_slip() {
    let p = TyreParams::default();
    for sx in [-0.3, -0.05, 0.0, 0.05, 0.3] {
        for sy in [-0.3, -0.05, 0.0, 0.05, 0.3] {
            let got = run(&p, at(3000.0, sx, sy));
            assert!(got.fx * sx >= 0.0, "fx has the sign of sx at ({sx}, {sy})");
            assert!(got.fy * sy <= 0.0, "fy opposes sy at ({sx}, {sy})");
            assert!(got.fx.is_finite() && got.fy.is_finite() && got.mz.is_finite());
        }
    }
    assert_eq!(run(&p, at(3000.0, 0.0, 0.0)), ForceOut::default());
}

#[test]
fn property_odd_symmetry_without_camber() {
    let p = TyreParams::default();
    for (sx, sy) in [(0.04, 0.07), (0.2, 0.01), (0.01, 0.3), (0.5, 0.5)] {
        let a = run(&p, ForceIn { slide_speed: 3.0, ..at(3500.0, sx, sy) });
        let b = run(&p, ForceIn { slide_speed: 3.0, ..at(3500.0, -sx, -sy) });
        assert_close("fx", b.fx, -a.fx, 1e-9, 1e-9);
        assert_close("fy", b.fy, -a.fy, 1e-9, 1e-9);
        assert_close("mz", b.mz, -a.mz, 1e-9, 1e-9);
        assert_close("n", b.n, a.n, 1e-9, 1e-9);
    }
}

#[test]
fn property_the_force_vector_is_on_the_ellipse_at_the_peak() {
    let p = TyreParams::default();
    let fz = 2000.0;
    let (mu_x, mu_y) = (p.mu_x0(), p.mu_y0);
    let s0 = p.peak_angle_deg.to_radians().tan();
    // Peak slips at fz0 (flexf 1, grip 1): s_py = s0, s_px = s0 / cx_ratio. Any direction with n = 1.
    for angle in [0.1_f64, 0.6, 1.0, 1.4] {
        let sx = angle.cos() * s0 / p.cx_ratio;
        let sy = angle.sin() * s0;
        let got = run(&p, at(fz, sx, sy));
        assert_close("n", got.n, 1.0, 1e-9, 1e-9);
        let sum = (got.fx / (mu_x * fz)).powi(2) + (got.fy / (mu_y * fz)).powi(2);
        assert_close("ellipse", sum, 1.0, 1e-6, 1e-6);
    }
}

#[test]
fn property_the_force_never_exceeds_the_friction_budget_and_is_continuous() {
    let p = TyreParams::default();
    let fz = 2000.0;
    let budget = p.mu_x0().max(p.mu_y0) * fz * 1.001;
    let mut previous: Option<ForceOut> = None;
    for step in 0..=600 {
        let slip = f64::from(step) * 0.001;
        let got = run(&p, at(fz, slip * 0.7, slip));
        assert!(got.fx.hypot(got.fy) <= budget, "budget at slip {slip}");
        if let Some(before) = previous {
            assert!((got.fx - before.fx).abs() < 200.0 && (got.fy - before.fy).abs() < 200.0, "jump at slip {slip}");
        }
        previous = Some(got);
    }
}

#[test]
fn property_without_load_sensitivity_the_peak_is_proportional_to_the_load() {
    let mut keys = p_keys();
    keys.exp_y = 1.0;
    keys.flex = 0.0;
    let p = TyreParams::new(keys, 0.225);
    let s0 = p.peak_angle_deg.to_radians().tan();
    let peak = |fz: f64| run(&p, at(fz, 0.0, s0)).fy;
    assert_close("ratio", peak(4000.0) / peak(2000.0), 2.0, 1e-9, 1e-9);
}
