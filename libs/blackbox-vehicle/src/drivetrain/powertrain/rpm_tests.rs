use super::super::*;
use crate::engine::ClutchState;

fn powertrain() -> Powertrain {
    Powertrain::new(tests::engine(), tests::trans(), Default::default(), Default::default(), 0.33)
}

fn tick_with_open_pedal(p: &mut Powertrain, dt: f32) {
    let mut wheels = [0.0; 4];
    p.tick(&mut TickInput {
        dt,
        gas: 0.0,
        nos_held: false,
        can_recharge_nos: true,
        blown: false,
        automatic: false,
        speed: 0.0,
        wheel_av: &mut wheels,
        grounded: [true; 4],
        drifting: false,
        drive_slip: 0.0,
        max_wheel_slip: 0.0,
        induction_tuning: 0.0,
        nos_tuning: 0.0,
        clutch_pedal: Some(1.0),
    });
}

#[test]
fn open_clutch_keeps_fast_higher_gear_display_decay_after_the_shift_ends() {
    for dt in [1.0 / 60.0, 1.0 / 144.0, 1.0 / 576.0] {
        let mut p = powertrain();
        p.gear = GEAR_FIRST + 1;
        p.rpm_display = 6000.0;
        p.shift_timer = dt * 0.5;
        let inertia = p.engine.inertia(false);
        let mut expected = p.rpm_display;
        for _ in 0..8 {
            expected = 0.55 * (expected - 15_000.0 / inertia * dt) + 0.45 * expected;
            tick_with_open_pedal(&mut p, dt);
            assert!(!p.shifting(), "the expired shift cannot supply the fast-decay flag");
            assert_eq!(p.clutch.state, ClutchState::Disengaged);
            assert!((p.rpm_display - expected).abs() < 0.002, "dt {dt}: {} vs {expected}", p.rpm_display);
        }
        p.clutch.engage(0.25);
        assert_eq!(p.clutch.state, ClutchState::Engaging);
        let before = p.rpm_display;
        p.smooth_rpm(dt);
        let expected = 0.55 * (before - 2500.0 / inertia * dt) + 0.45 * before;
        assert!((p.rpm_display - expected).abs() < 0.002, "engaging alone keeps the ordinary decay");
    }
}

#[test]
fn staged_first_gear_keeps_ordinary_display_decay_with_an_open_clutch() {
    for dt in [1.0 / 60.0, 1.0 / 144.0, 1.0 / 576.0] {
        let mut p = powertrain();
        p.staging = true;
        p.rpm_display = 6000.0;
        let inertia = p.engine.inertia(false);
        let mut expected = p.rpm_display;
        for _ in 0..8 {
            expected = 0.55 * (expected - 2500.0 / inertia * dt) + 0.45 * expected;
            tick_with_open_pedal(&mut p, dt);
            assert_eq!(p.gear(), GEAR_FIRST);
            assert_eq!(p.clutch.state, ClutchState::Disengaged);
            assert!((p.rpm_display - expected).abs() < 0.002, "staged dt {dt}: {} vs {expected}", p.rpm_display);
        }
    }
}
