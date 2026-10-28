//! The flames of a car a mod added: every pop and every up-shift under power shows (`trigger::MOD_*`).

use blackbox_particles::EmitterSpec;
use blackbox_render::BlendMode;
use glam::{Mat4, Vec3, Vec4};

use super::load::{lane, mod_timing};
use super::trigger::{MOD_BACKFIRE_INTENSITY, MOD_BACKFIRE_SECONDS, MOD_SHIFT_TIMING, ShiftTiming};
use super::{Active, CarState, ExhaustFlames, SpriteTexture, State};

const STEP: f32 = 1.0 / 60.0;

fn spec() -> EmitterSpec {
    EmitterSpec { rate: 300.0, life: 0.1, speed: 6.0, size: [0.4; 4], ..Default::default() }
}

/// A mod car (or a stock one) with two pipes at the rear, flaming from the stock `ecar` timing of the racers.
fn flames(mod_car: bool, timing: ShiftTiming, engine_upgrades: i32) -> ExhaustFlames {
    let pipe = |y: f32| {
        Mat4::from_cols(
            Vec4::new(0.0, 1.0, 0.0, 0.0),
            Vec4::new(0.0, 0.0, 1.0, 0.0),
            Vec4::new(-1.0, 0.0, 0.0, 0.0),
            Vec4::new(-2.0, y, 0.3, 1.0),
        )
    };
    let textures = vec![SpriteTexture { handle: None, blend: BlendMode::Additive }];
    let mut active =
        Active::new(vec![pipe(0.3), pipe(-0.3)], vec![lane(&spec(), 0, 2, 0)], textures, timing, engine_upgrades);
    active.mod_car = mod_car;
    ExhaustFlames { enabled: true, state: State::Ready(Box::new(active)), ..ExhaustFlames::default() }
}

fn active(fx: &ExhaustFlames) -> &Active {
    fx.active().expect("loaded")
}

fn car(gear: i32, throttle: f32) -> CarState {
    CarState {
        to_world: Mat4::from_translation(Vec3::new(100.0, 0.0, 0.0)),
        velocity: Vec3::new(40.0, 0.0, 0.0),
        gear,
        throttle,
        nitrous: false,
    }
}

fn run(fx: &mut ExhaustFlames, steps: usize, car: &CarState) {
    for _ in 0..steps {
        fx.step(STEP, car);
    }
}

/// Steps of the next 60 that flame.
fn flaming_steps(fx: &mut ExhaustFlames, car: &CarState) -> usize {
    (0..60)
        .filter(|_| {
            fx.step(STEP, car);
            active(fx).intensity > 0.0
        })
        .count()
}

#[test]
fn a_mod_car_without_shift_timing_falls_back_on_the_racers() {
    assert_eq!(mod_timing(0.0, 0.0), MOD_SHIFT_TIMING);
    assert_eq!(mod_timing(8.0, 0.0), MOD_SHIFT_TIMING);
    let own = ShiftTiming { speed: 8.0, angle: 0.5 };
    assert_eq!(mod_timing(own.speed, own.angle), own, "a record that has a timing keeps it");
}

#[test]
fn a_mod_car_blows_off_flames_at_an_up_shift_under_power_even_with_the_default_timing() {
    let mut fx = flames(true, MOD_SHIFT_TIMING, 0);
    run(&mut fx, 5, &car(3, 1.0));
    fx.step(STEP, &car(4, 1.0));
    assert_eq!(active(&fx).intensity, 1.0, "the full flame at once");
    fx.step(STEP, &car(4, 1.0));
    assert!(fx.live() > 0, "particles leave the pipe while the flame is on");
    assert!(flaming_steps(&mut fx, &car(4, 1.0)) >= 14, "for the pitch time of the racers, about 0.28 s");
}

#[test]
fn a_mod_car_flames_at_no_lift_off_shift_and_no_down_shift() {
    let mut lifted = flames(true, MOD_SHIFT_TIMING, 0);
    run(&mut lifted, 5, &car(3, 0.0));
    run(&mut lifted, 3, &car(4, 0.0));
    assert_eq!(lifted.live(), 0, "an up-shift with the pedal up has no blow-off");

    let mut down = flames(true, MOD_SHIFT_TIMING, 0);
    run(&mut down, 5, &car(4, 1.0));
    run(&mut down, 3, &car(3, 1.0));
    assert_eq!(down.live(), 0, "a down-shift has none either");
}

#[test]
fn a_stock_car_keeps_flaming_at_every_shift_it_allows() {
    let mut fx = flames(false, MOD_SHIFT_TIMING, 0);
    run(&mut fx, 5, &car(4, 1.0));
    run(&mut fx, 3, &car(3, 0.0));
    assert!(fx.live() > 0, "a down-shift off the pedal still flames on a stock car");
}

#[test]
fn every_pop_of_a_mod_car_flames_at_full_strength_for_longer_than_a_stock_pop() {
    let mut fx = flames(true, MOD_SHIFT_TIMING, 0);
    run(&mut fx, 5, &car(3, 0.0));
    fx.note_pops(1);
    fx.step(STEP, &car(3, 0.0));
    assert_eq!(active(&fx).intensity, MOD_BACKFIRE_INTENSITY);
    assert!(fx.live() > 0);
    let rest = flaming_steps(&mut fx, &car(3, 0.0));
    let wanted = (MOD_BACKFIRE_SECONDS / STEP).ceil() as usize - 1;
    assert!(rest >= wanted - 1 && rest <= wanted + 1, "{rest} more steps of flame, wanted about {wanted}");
}

#[test]
fn a_mod_car_pop_flames_even_if_the_pedal_is_down_again_by_the_time_the_bang_is_due() {
    let mut fx = flames(true, MOD_SHIFT_TIMING, 0);
    fx.note_pops(1);
    fx.step(STEP, &car(3, 0.8));
    assert_eq!(active(&fx).intensity, MOD_BACKFIRE_INTENSITY);
    // Not in reverse or neutral.
    let mut neutral = flames(true, MOD_SHIFT_TIMING, 0);
    neutral.note_pops(1);
    neutral.step(STEP, &car(0, 0.0));
    assert_eq!(neutral.live(), 0);
}

#[test]
fn a_stock_car_pop_with_the_pedal_down_is_still_not_shown() {
    let mut fx = flames(false, MOD_SHIFT_TIMING, 0);
    fx.note_pops(1);
    fx.step(STEP, &car(3, 0.8));
    assert_eq!(fx.live(), 0);
}

#[test]
fn only_the_nitrous_lanes_are_shrunk_the_pop_and_shift_lanes_keep_the_groups_sizes() {
    let fx = flames(true, MOD_SHIFT_TIMING, 0);
    let a = active(&fx);
    let orange = a.lanes[0].emitters[0].spec();
    let blue = a.blue[0].emitters[0].spec();
    assert_eq!((orange.size, orange.speed), ([0.4; 4], 6.0), "the orange lanes are the group's own");
    assert!(blue.size[1] < orange.size[1] && blue.speed < orange.speed, "the blue lanes are the compact nitrous");
}
