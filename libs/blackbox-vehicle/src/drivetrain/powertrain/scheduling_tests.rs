use super::super::*;

fn powertrain() -> Powertrain {
    Powertrain::new(tests::engine(), tests::trans(), Default::default(), Default::default(), 0.33)
}

fn tick(p: &mut Powertrain, dt: f32) {
    let mut av = [0.0; 4];
    p.tick(&mut TickInput {
        dt,
        gas: 1.0,
        nos_held: false,
        can_recharge_nos: true,
        blown: false,
        automatic: true,
        speed: 10.0,
        wheel_av: &mut av,
        grounded: [true; 4],
        drifting: false,
        drive_slip: 0.0,
        max_wheel_slip: 0.0,
        induction_tuning: 0.0,
        nos_tuning: 0.0,
        clutch_pedal: None,
    });
}

#[test]
fn cached_request_is_consumed_before_shift_timer_and_clutch_advance() {
    for dt in [1.0 / 60.0, 1.0 / 144.0, 1.0 / 240.0, 1.0 / 576.0] {
        let mut blocked = powertrain();
        blocked.shift_potential = ShiftPotential::Up;
        blocked.shift_timer = dt * 0.5;
        tick(&mut blocked, dt);
        assert_eq!(blocked.gear(), GEAR_FIRST, "an existing shift still blocks this callback");
        assert_eq!(blocked.shift_timer, 0.0);

        let mut ready = powertrain();
        // The current RPM would not ask for an upshift. The previous request decides.
        ready.shift_potential = ShiftPotential::Up;
        let delay = ready.trans.shift_speed * ready.trans.ratio(GEAR_FIRST + 1);
        tick(&mut ready, dt);
        assert_eq!(ready.gear(), GEAR_FIRST + 1);
        assert_eq!(ready.shift_timer, delay - dt, "a newly created shift timer advances this callback");
        assert_eq!(ready.shift_potential, ShiftPotential::None);
    }
}

#[test]
fn neutral_exit_has_no_shift_delay_and_sport_hold_counts_only_a_pending_wish() {
    let dt = crate::FIXED_STEP;
    let mut neutral = powertrain();
    neutral.shift(GEAR_NEUTRAL);
    neutral.shift_timer = 0.0;
    tick(&mut neutral, dt);
    assert_eq!(neutral.gear(), GEAR_FIRST);
    assert_eq!(neutral.shift_timer, 0.0);

    let mut sport = powertrain();
    sport.sport_timer = 1.25;
    tick(&mut sport, dt);
    assert_eq!(sport.sport_timer, 1.25);
    sport.shift_potential = ShiftPotential::Up;
    tick(&mut sport, dt);
    assert_eq!(sport.sport_timer, 1.25 - dt);
    sport.sport_timer = dt * 0.5;
    sport.shift_potential = ShiftPotential::Up;
    tick(&mut sport, dt);
    assert_eq!(sport.sport_timer, 0.0);
}

#[test]
fn normalizing_transmission_span_first_preserves_the_redline_cut() {
    let engine = crate::engine::EngineSpec { idle: 800.0, red_line: 6000.0, torque: vec![128.0; 5], ..tests::engine() };
    let trans = TransmissionSpec { gear_ratio: vec![3.0, 0.0, 20.0, 10.0], final_gear: 1.0, ..tests::trans() };
    let mut p = Powertrain::new(engine, trans, Default::default(), Default::default(), 0.33);
    let max = p.engine.rev_limit_rad();
    p.omega = max;
    p.omega_trans = max;
    p.throttle = 1.0;
    let mut av = [f32::from_bits(0x41fb_53d2); 4];
    let torque = p.torque_loop(&mut super::super::torque_loop::LoopCtx {
        dt: 1.0 / 144.0,
        av: &mut av,
        grounded: [true; 4],
        loaded_wheels: 4,
        drive_slip: 0.0,
        speed: 10.0,
    });
    assert_eq!(torque, 0.0);
    assert_eq!(p.omega_trans, max);
    assert_eq!(p.wheel_writes(), [false, false, true, true]);
}

#[test]
fn nos_uses_its_own_speed_conversion_at_activation_and_recharge_thresholds() {
    let spec = crate::nos::NosSpec {
        nos_capacity: 5.0,
        torque_boost: 1.0,
        nos_disengage: 2.0,
        recharge_min: 50.0,
        recharge_max: 30.0,
        recharge_min_speed: 50.0,
        recharge_max_speed: 100.0,
    };
    for (speed_bits, held, expected_capacity, expected_boost) in
        [(0x408f0ca3, true, 0.5 - (1.0 / 144.0) / 5.0, 2.0), (0x41b2cfcc, false, 0.5 + (1.0 / 144.0) / 50.0, 1.0)]
    {
        let mut p = Powertrain::new(tests::engine(), tests::trans(), Default::default(), spec, 0.33);
        p.nos.capacity = 0.5;
        let mut av = [0.0; 4];
        p.tick(&mut TickInput {
            dt: 1.0 / 144.0,
            gas: 1.0,
            nos_held: held,
            can_recharge_nos: true,
            blown: false,
            automatic: false,
            speed: f32::from_bits(speed_bits),
            wheel_av: &mut av,
            grounded: [true; 4],
            drifting: false,
            drive_slip: 0.0,
            max_wheel_slip: 0.0,
            induction_tuning: 0.0,
            nos_tuning: 0.0,
            clutch_pedal: None,
        });
        assert_eq!(p.nos.capacity, expected_capacity);
        assert_eq!(p.nos.boost, expected_boost);
    }
}
