use super::*;
use crate::engine::EngineSpec;
use crate::induction::InductionSpec;
use crate::nos::NosSpec;

pub(crate) fn engine() -> EngineSpec {
    EngineSpec {
        torque: vec![180.0, 215.0, 245.0, 260.0, 255.0, 235.0, 205.0, 170.0, 130.0],
        idle: 900.0,
        red_line: 7000.0,
        max_rpm: 8000.0,
        flywheel_mass: 20.0,
        engine_braking: vec![0.15],
        speed_limiter: [0.0, 0.0],
    }
}

pub(crate) fn trans() -> TransmissionSpec {
    TransmissionSpec {
        gear_ratio: vec![3.5, 0.0, 3.6, 2.2, 1.55, 1.15, 0.9, 0.75],
        gear_efficiency: vec![0.9; 8],
        final_gear: 3.7,
        torque_split: 0.0,
        differential: [0.0, 0.3, 0.0],
        torque_converter: 0.0,
        clutch_slip: 0.3,
        shift_speed: 0.1,
        optimal_shift: 0.0,
    }
}

fn powertrain() -> Powertrain {
    Powertrain::new(engine(), trans(), InductionSpec::default(), NosSpec::default(), 0.33)
}

#[test]
fn shift_points_are_ordered_and_in_range() {
    let p = powertrain();
    let sp = p.shift_points();
    let e = engine();
    for g in GEAR_FIRST..p.top_gear() {
        assert!(sp.up[g] > e.idle && sp.up[g] <= e.red_line, "gear {g}: {}", sp.up[g]);
        assert!(sp.down[g + 1] < sp.up[g], "down {} up {}", sp.down[g + 1], sp.up[g]);
    }
    assert_eq!(sp.up[p.top_gear()], e.red_line);
}

#[test]
fn downshift_drops_one_gear_at_a_time() {
    let p = powertrain();
    let sp = p.shift_points();
    let t = trans();
    for g in (GEAR_FIRST + 2)..=p.top_gear() {
        // The engine just below the gear's downshift point: the next lower gear is the target.
        let rpm = sp.down[g] - 10.0;
        assert_eq!(sp.downshift_target(&t, g, rpm), g - 1, "from gear {g} at {rpm} rpm");
    }
    // Far below every downshift point the box may skip gears, but never past first.
    assert_eq!(sp.downshift_target(&t, p.top_gear(), 1.0), GEAR_FIRST);
}

#[test]
fn speedometer_formula() {
    let p = powertrain();
    // At the red line in first gear the wheel turns at redline / (ratio) rpm.
    let v = p.speed_at(7000.0, GEAR_FIRST);
    let wheel_rpm = (7000.0 - 900.0) / (3.6 * 3.7) / (7000.0 - 900.0) * 7000.0;
    let expect = wheel_rpm / 9.549_296 * 0.33;
    assert!((v - expect).abs() < 1e-3, "{v} vs {expect}");
    assert!(p.max_speed() > 40.0 && p.max_speed() < 120.0, "{}", p.max_speed());
}

#[test]
fn shift_delay_depends_on_direction() {
    let mut p = powertrain();
    assert!(p.shift(GEAR_FIRST + 1));
    assert!((p.shift_timer - 0.1 * 2.2).abs() < 1e-6);
    assert!(!p.shift(GEAR_FIRST + 1));
    assert!(p.shift(GEAR_FIRST));
    assert!((p.shift_timer - 0.1 * 3.6 * 0.25).abs() < 1e-6);
    assert!(!p.shift(99));
}

/// A point mass with ideal grip standing in for the chassis.
struct Dyno {
    pt: Powertrain,
    v: f32,
    av: [f32; 4],
    radius: f32,
    mass: f32,
    automatic: bool,
}

impl Dyno {
    fn new() -> Self {
        Self { pt: powertrain(), v: 0.0, av: [0.0; 4], radius: 0.33, mass: 1500.0, automatic: true }
    }

    fn step(&mut self, gas: f32) {
        let dt = crate::FIXED_STEP;
        self.av = [self.v / self.radius; 4];
        let mut input = TickInput {
            dt,
            gas,
            nos_held: false,
            blown: false,
            automatic: self.automatic,
            speed: self.v,
            wheel_av: &mut self.av,
            grounded: [true; 4],
            drive_slip: 0.0,
            max_wheel_slip: 0.0,
            induction_tuning: 0.0,
            nos_tuning: 0.0,
        };
        let torque = self.pt.tick(&mut input);
        let drag = 0.4 * self.v * self.v + 150.0;
        let a = (torque / self.radius - drag) / self.mass;
        self.v = (self.v + a * dt).max(0.0);
    }
}

#[test]
fn accelerates_and_shifts_up_through_the_gears() {
    let mut d = Dyno::new();
    let mut max_gear = d.pt.gear();
    let mut max_rpm = 0.0_f32;
    for _ in 0..(60 * 40) {
        d.step(1.0);
        max_gear = max_gear.max(d.pt.gear());
        max_rpm = max_rpm.max(d.pt.rpm());
        assert!(d.pt.rpm().is_finite() && d.v.is_finite());
    }
    assert!(d.pt.gear() >= GEAR_FIRST + 4, "ended in gear {}", d.pt.gear());
    assert_eq!(max_gear, d.pt.gear());
    assert!(d.v > 35.0, "speed after 40 s: {}", d.v);
    assert!(max_rpm <= 7001.0, "rpm {max_rpm}");
}

#[test]
fn zero_to_sixty_is_sensible() {
    let mut d = Dyno::new();
    let mut t = 0.0;
    while d.v < 26.8 && t < 30.0 {
        d.step(1.0);
        t += crate::FIXED_STEP;
    }
    assert!(t > 3.0 && t < 14.0, "0-60 mph in {t} s");
}

#[test]
fn idles_without_throttle() {
    let mut d = Dyno::new();
    for _ in 0..120 {
        d.step(0.0);
    }
    assert!((d.pt.rpm() - 900.0).abs() < 60.0, "idle rpm {}", d.pt.rpm());
    assert!(d.v < 0.5);
}

#[test]
fn rev_limiter_holds_the_engine_in_neutral() {
    let mut d = Dyno::new();
    d.automatic = false;
    assert!(d.pt.shift(GEAR_NEUTRAL));
    for _ in 0..240 {
        d.step(1.0);
    }
    assert!(d.pt.rpm() <= 7001.0 && d.pt.rpm() > 6500.0, "rpm {}", d.pt.rpm());
}

#[test]
fn speed_limiter_tapers_the_throttle() {
    let mut e = engine();
    e.speed_limiter = [60.0, 10.0];
    let mut d = Dyno::new();
    d.pt = Powertrain::new(e, trans(), InductionSpec::default(), NosSpec::default(), 0.33);
    for _ in 0..(60 * 60) {
        d.step(1.0);
    }
    let mph = d.v * 2.2369;
    assert!(mph > 55.0 && mph < 72.0, "speed {mph} mph");
}

#[test]
fn match_speed_picks_a_gear_and_rpm() {
    let mut p = powertrain();
    p.match_speed(30.0);
    assert!(p.gear() > GEAR_FIRST);
    assert!(p.rpm() >= 900.0 && p.rpm() <= 7000.0);
    p.match_speed(-3.0);
    assert_eq!(p.gear(), GEAR_REVERSE);
}
