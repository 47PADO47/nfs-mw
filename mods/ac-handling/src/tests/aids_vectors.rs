//! The test vectors of docs/specs/ac-handling/aids.md section 6: TC1 to TC6, ABS1 to ABS4, BIAS and BIAS2, ESC1 to ESC7, FLAG.
//! Tolerance 1e-4 absolute. `dt = 1/60`, `driven = [F, F, T, T]`, `mass = 1400`, `track_f = 1.52`,
//! `w_static = [3740, 3740, 3000, 3000]`, `beta_base = beta_ref = 0.58` unless stated.

use blackbox_modding::sdk::{ControlsQuery, Role};

use super::assert_close;
use crate::aids::{AidCar, AidIn, AidOut, AidState, Levels, aids_step};
use crate::controls::{ControlsCfg, ControlsState, Feed, controls_step};
use crate::keys::SteeringKeys;
use crate::steering::Device;

const DT: f64 = 1.0 / 60.0;

fn car() -> AidCar {
    AidCar {
        mass: 1400.0,
        track_f: 1.52,
        w_static: [3740.0, 3740.0, 3000.0, 3000.0],
        beta_ref: 0.58,
        beta_base: 0.58,
        driven: [false, false, true, true],
    }
}

fn input(throttle: f64, brake: f64, v: f64, kappa: [f64; 4], alpha: [f64; 4]) -> AidIn {
    AidIn { dt: DT, throttle, brake, v, kappa, alpha, loaded: [true; 4] }
}

fn levels(tc: u32, abs: u32, esc: u32, bias_clicks: i32) -> Levels {
    Levels { tc, abs, esc, bias_clicks }
}

/// The traction control slip pattern of TC1: `s` per tick on the driven wheels.
fn tc_slip(tick: usize) -> f64 {
    match tick {
        1..=5 => 0.05,
        6..=11 => 0.20,
        _ => 0.05,
    }
}

fn run_tc(level: u32, v: f64) -> Vec<AidOut> {
    let mut state = AidState::new();
    (1..=15)
        .map(|tick| {
            let s = tc_slip(tick);
            aids_step(
                &car(),
                levels(level, 0, 0, 0),
                &mut state,
                &input(1.0, 0.0, v, [0.0, 0.0, s, s - 0.01], [0.0; 4]),
            )
        })
        .collect()
}

#[test]
fn tc1_level_3_cuts_from_the_check_of_tick_6_to_the_check_of_tick_12() {
    let out = run_tc(3, 10.0);
    for (index, step) in out.iter().enumerate() {
        let tick = index + 1;
        let cutting = (6..=11).contains(&tick);
        assert_close(&format!("TC1 throttle tick {tick}"), step.throttle, if cutting { 0.2 } else { 1.0 }, 0.0, 1e-4);
        assert_eq!(step.tc_active, cutting, "tc_cut at tick {tick}");
    }
    let mut state = AidState::new();
    for (tick, want) in [(1, 0.0167), (2, 0.0333), (3, 0.0), (4, 0.0167)] {
        aids_step(&car(), levels(3, 0, 0, 0), &mut state, &input(1.0, 0.0, 10.0, [0.0, 0.0, 0.05, 0.04], [0.0; 4]));
        assert_close(&format!("tc_acc after tick {tick}"), state.tc_acc, want, 0.0, 1e-4);
    }
}

#[test]
fn tc2_to_tc6() {
    let out = run_tc(5, 10.0);
    for (index, step) in out.iter().enumerate() {
        let cutting = (6..=11).contains(&(index + 1));
        assert_close(&format!("TC2 tick {}", index + 1), step.throttle, if cutting { 0.0 } else { 1.0 }, 0.0, 1e-4);
    }
    let mut state = AidState::new();
    for _ in 0..6 {
        let out =
            aids_step(&car(), levels(1, 0, 0, 0), &mut state, &input(0.8, 0.0, 10.0, [0.0, 0.0, 0.18, 0.18], [0.0; 4]));
        assert_close("TC3", out.throttle, 0.8, 0.0, 1e-4);
        assert!(!out.tc_active);
    }
    let mut state = AidState::new();
    for _ in 0..6 {
        let out =
            aids_step(&car(), levels(5, 0, 0, 0), &mut state, &input(1.0, 0.0, 0.5, [0.0, 0.0, 0.5, 0.5], [0.0; 4]));
        assert_close("TC4 below v_tc", out.throttle, 1.0, 0.0, 1e-4);
    }
    let mut state = AidState::new();
    for _ in 0..6 {
        let out = aids_step(&car(), levels(0, 0, 0, 0), &mut state, &input(1.0, 0.0, 10.0, [0.5; 4], [0.0; 4]));
        assert_close("TC5 level 0", out.throttle, 1.0, 0.0, 1e-4);
    }
    let reverse = run_tc(3, -10.0);
    for (index, step) in reverse.iter().enumerate() {
        let cutting = (6..=11).contains(&(index + 1));
        assert_close(&format!("TC6 tick {}", index + 1), step.throttle, if cutting { 0.2 } else { 1.0 }, 0.0, 1e-4);
    }
}

#[test]
fn abs1_level_3_releases_a_locking_wheel_for_the_held_period() {
    let mut state = AidState::new();
    for tick in 1..=12 {
        let fl = match tick {
            1..=2 => -0.05,
            3..=6 => -0.30,
            _ => -0.05,
        };
        let out = aids_step(
            &car(),
            levels(0, 3, 0, 0),
            &mut state,
            &input(0.0, 1.0, 20.0, [fl, -0.05, -0.05, -0.05], [0.0; 4]),
        );
        let releasing = (3..=8).contains(&tick);
        assert_close(
            &format!("ABS1 brake_out[0] tick {tick}"),
            out.brake[0],
            if releasing { 0.1 } else { 1.0 },
            0.0,
            1e-4,
        );
        assert_eq!(out.abs_active[0], releasing, "abs_rel[0] at tick {tick}");
        for other in &out.brake[1..] {
            assert_close("ABS1 other wheels", *other, 1.0, 0.0, 1e-4);
        }
    }
}

#[test]
fn abs2_to_abs4() {
    let mut state = AidState::new();
    let mut out = None;
    for _ in 0..6 {
        out = Some(aids_step(&car(), levels(0, 2, 0, 0), &mut state, &input(0.0, 1.0, 20.0, [-0.22; 4], [0.0; 4])));
    }
    assert_eq!(out.expect("ran").brake, [1.0; 4]);
    let mut state = AidState::new();
    for _ in 0..6 {
        out = Some(aids_step(
            &car(),
            levels(0, 1, 0, 0),
            &mut state,
            &input(0.0, 0.8, 20.0, [-0.35, -0.05, -0.35, -0.05], [0.0; 4]),
        ));
    }
    let got = out.expect("ran").brake;
    for (wheel, want) in [0.24, 0.8, 0.24, 0.8].into_iter().enumerate() {
        assert_close(&format!("ABS3 wheel {wheel}"), got[wheel], want, 0.0, 1e-4);
    }
    let mut state = AidState::new();
    for _ in 0..6 {
        out = Some(aids_step(&car(), levels(0, 5, 0, 0), &mut state, &input(0.0, 1.0, 5.0, [-0.5; 4], [0.0; 4])));
    }
    assert_eq!(out.expect("ran").brake, [1.0; 4], "ABS4: below 20 km/h");
}

#[test]
fn bias_moves_the_front_share_and_never_leaves_the_range() {
    let rows = [
        (0, [1.0, 1.0, 1.0, 1.0]),
        (10, [1.0862, 1.0862, 0.8810, 0.8810]),
        (-10, [0.9138, 0.9138, 1.1190, 1.1190]),
        (100, [1.3793, 1.3793, 0.4762, 0.4762]),
        (-100, [0.5172, 0.5172, 1.5, 1.5]),
    ];
    for (clicks, want) in rows {
        let mut state = AidState::new();
        // The clicks are clamped to +-20 by the levels; the spec's +-100 rows are its clamp of beta, so give beta directly.
        let mut car = car();
        car.beta_base = 0.58 + 0.005 * f64::from(clicks);
        let out = aids_step(&car, levels(0, 0, 0, 0), &mut state, &input(0.0, 1.0, 20.0, [0.0; 4], [0.0; 4]));
        for wheel in 0..4 {
            assert_close(&format!("BIAS {clicks} wheel {wheel}"), out.brake[wheel], want[wheel], 0.0, 1e-4);
        }
    }
    // Through the levels: +-10 clicks.
    for (clicks, want) in [(10, [1.0862, 1.0862, 0.8810, 0.8810]), (-10, [0.9138, 0.9138, 1.1190, 1.1190])] {
        let mut state = AidState::new();
        let out = aids_step(&car(), levels(0, 0, 0, clicks), &mut state, &input(0.0, 1.0, 20.0, [0.0; 4], [0.0; 4]));
        for wheel in 0..4 {
            assert_close(&format!("BIAS clicks {clicks} wheel {wheel}"), out.brake[wheel], want[wheel], 0.0, 1e-4);
        }
    }
    let mut state = AidState::new();
    let mut odd = car();
    odd.beta_base = 0.65;
    odd.beta_ref = 0.55;
    let out = aids_step(&odd, levels(0, 0, 0, 0), &mut state, &input(0.0, 0.6, 20.0, [0.0; 4], [0.0; 4]));
    for (wheel, want) in [0.7091, 0.7091, 0.4667, 0.4667].into_iter().enumerate() {
        assert_close(&format!("BIAS 0.65 over 0.55 wheel {wheel}"), out.brake[wheel], want, 0.0, 1e-4);
    }
}

#[test]
fn esc1_brakes_the_front_wheel_on_the_side_the_tail_slides_towards() {
    let mut state = AidState::new();
    let alpha = [0.02, 0.02, 0.10, 0.10];
    for (index, want) in [0.3408, 0.5654, 0.7135, 0.8111, 0.8755, 0.9179].into_iter().enumerate() {
        let out = aids_step(&car(), levels(0, 0, 4, 0), &mut state, &input(0.5, 0.0, 25.0, [0.0; 4], alpha));
        assert_close(&format!("ESC1 tick {}", index + 1), out.brake[1], want, 0.0, 1e-4);
        assert_eq!((out.brake[0], out.brake[2], out.brake[3]), (0.0, 0.0, 0.0));
        assert!(out.esc_active);
    }
    // ESC5: the slide ends; the brake fades on the wheel that was braked.
    for (index, want) in [0.6051, 0.3989, 0.2630, 0.1734, 0.1143, 0.0754].into_iter().enumerate() {
        let out = aids_step(&car(), levels(0, 0, 4, 0), &mut state, &input(0.5, 0.0, 25.0, [0.0; 4], [0.0; 4]));
        assert_close(&format!("ESC5 tick {}", index + 1), state.esc_y, want, 0.0, 1e-4);
        assert_close(&format!("ESC5 brake tick {}", index + 1), out.brake[1], want, 0.0, 1e-4);
        assert_eq!(out.brake[0], 0.0);
    }
}

#[test]
fn esc2_to_esc4() {
    let mut state = AidState::new();
    let mut out = None;
    for _ in 0..3 {
        out = Some(aids_step(
            &car(),
            levels(0, 0, 2, 0),
            &mut state,
            &input(0.5, 0.0, 25.0, [0.0; 4], [-0.01, -0.01, -0.06, -0.06]),
        ));
    }
    let got = out.expect("ran").brake;
    assert_close("ESC2 front left", got[0], 0.2636, 0.0, 1e-4);
    assert_eq!((got[1], got[2], got[3]), (0.0, 0.0, 0.0));
    let mut state = AidState::new();
    let out =
        aids_step(&car(), levels(0, 0, 4, 0), &mut state, &input(0.5, 0.0, 25.0, [0.0; 4], [0.10, 0.10, 0.02, 0.02]));
    assert_eq!(out.brake, [0.0; 4], "ESC3: understeer");
    assert_eq!(state.esc_y, 0.0);
    let mut state = AidState::new();
    let out =
        aids_step(&car(), levels(0, 0, 4, 0), &mut state, &input(0.5, 0.0, 1.0, [0.0; 4], [0.02, 0.02, 0.10, 0.10]));
    assert_eq!(out.brake, [0.0; 4], "ESC4: below 5 km/h");
}

#[test]
fn esc6_leaves_a_wheel_that_abs_is_releasing_alone() {
    let mut state = AidState::new();
    let mut out = None;
    for _ in 0..6 {
        out = Some(aids_step(
            &car(),
            levels(0, 3, 4, 0),
            &mut state,
            &input(0.0, 1.0, 25.0, [-0.05, -0.30, -0.05, -0.05], [0.02, 0.02, 0.10, 0.10]),
        ));
    }
    let out = out.expect("ran");
    for (wheel, want) in [1.0, 0.1, 1.0, 1.0].into_iter().enumerate() {
        assert_close(&format!("ESC6 wheel {wheel}"), out.brake[wheel], want, 0.0, 1e-4);
    }
    assert!(out.abs_active[1]);
    assert_close("ESC6 esc_y", state.esc_y, 0.9179, 0.0, 1e-4);
}

fn cfg(aids: bool, steering: bool, levels: Levels) -> ControlsCfg {
    ControlsCfg { aids, steering, levels, car: Some(car()), steer_keys: SteeringKeys::default(), device: Device::Pad }
}

/// A controls query: the slip angles in turns, as the host sends them.
fn query(throttle: f32, brake: f32, v: f32, kappa: [f32; 4], alpha_rad: [f64; 4]) -> ControlsQuery {
    ControlsQuery {
        role: Some(Role::Player),
        vehicle_revision: 1,
        dt: DT as f32,
        throttle,
        brake,
        forward_speed: v,
        wheel_loaded: [true; 4],
        slip_ratio: kappa,
        slip_angle: alpha_rad.map(|alpha| (alpha / std::f64::consts::TAU) as f32),
        ..ControlsQuery::default()
    }
}

#[test]
fn esc7_the_query_sends_turns_and_the_mod_converts_them_back() {
    let mut state = ControlsState::default();
    let q = query(0.5, 0.0, 25.0, [0.0; 4], [0.02, 0.02, 0.10, 0.10]);
    assert_close("0.02 rad in turns", f64::from(q.slip_angle[0]), 0.003_183_1, 0.0, 1e-6);
    let out = controls_step(&cfg(true, false, levels(0, 0, 4, 0)), &mut state, &q, &Feed::default()).expect("handled");
    assert_close("ESC7 tick 1", f64::from(out.brake[1]), 0.3408, 0.0, 1e-4);
}

#[test]
fn bias2_and_flag_the_answer_stays_inside_the_hosts_range_and_has_no_steering_flag() {
    // 100 clicks is beyond the menu's range: the levels clamp to 20, and a bias of the spec's +-100 is tested above on beta.
    for clicks in [100, -100, 20, -20] {
        let mut state = ControlsState::default();
        let q = query(1.0, 1.0, 20.0, [0.0; 4], [0.0; 4]);
        let out = controls_step(&cfg(true, false, levels(0, 0, 0, clicks)), &mut state, &q, &Feed::default())
            .expect("handled");
        assert!(out.brake.iter().all(|b| (0.0..=1.5).contains(b)), "{:?}", out.brake);
        assert!(out.throttle >= 0.0 && out.throttle <= 1.0);
        assert_eq!(out.steer_target_rad, None, "FLAG: the aids never set the steering flag");
    }
}

#[test]
fn the_controls_hook_declines_other_roles_and_bad_numbers() {
    let mut state = ControlsState::default();
    let config = cfg(true, true, levels(1, 1, 1, 0));
    let good = query(0.5, 0.0, 20.0, [0.0; 4], [0.0; 4]);
    assert!(controls_step(&config, &mut state, &good, &Feed::default()).is_some());
    for role in [Role::Traffic, Role::Cop, Role::Racer] {
        assert!(
            controls_step(&config, &mut state, &ControlsQuery { role: Some(role), ..good }, &Feed::default()).is_none()
        );
    }
    assert!(controls_step(&config, &mut state, &ControlsQuery { role: None, ..good }, &Feed::default()).is_none());
    assert!(
        controls_step(&config, &mut state, &ControlsQuery { throttle: f32::NAN, ..good }, &Feed::default()).is_none()
    );
    assert!(controls_step(&config, &mut state, &ControlsQuery { dt: 0.0, ..good }, &Feed::default()).is_none());
    assert!(controls_step(&config, &mut state, &ControlsQuery { dt: 0.5, ..good }, &Feed::default()).is_none());
    let mut slip = good;
    slip.slip_ratio[2] = f32::INFINITY;
    assert!(controls_step(&config, &mut state, &slip, &Feed::default()).is_none());
    assert!(controls_step(&cfg(false, false, Levels::default()), &mut state, &good, &Feed::default()).is_none());
    let no_car = ControlsCfg { car: None, ..config };
    assert!(controls_step(&no_car, &mut state, &good, &Feed::default()).is_none(), "aids need the car's constants");
}
