//! The flames of a car a mod added: every pop and every up-shift under power shows (`trigger::MOD_*`).

use blackbox_particles::EmitterSpec;
use blackbox_render::BlendMode;
use glam::{Mat4, Vec3, Vec4};

use super::load::{lane, mod_timing, shift_timing};
use super::scale::compact_bike;
use super::trigger::{
    BIKE_BACKFIRE_SCALE, BIKE_BACKFIRE_SCALE_OVER_SHIFT, BIKE_FLAME_SCALE, BIKE_SHIFT_TIMING, MOD_BACKFIRE_INTENSITY,
    MOD_BACKFIRE_SECONDS, MOD_SHIFT_TIMING, ShiftTiming,
};
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

/// A bike: a mod car that is a bike, with the shift flame's lanes at the bike's size and the pop lanes a quarter bigger.
fn bike_flames() -> ExhaustFlames {
    let mut fx = flames(true, BIKE_SHIFT_TIMING, 0);
    if let State::Ready(active) = &mut fx.state {
        active.bike = true;
        active.lanes = vec![lane(&compact_bike(&spec(), BIKE_FLAME_SCALE), 0, 2, 0)];
        active.pop = vec![lane(&compact_bike(&spec(), BIKE_BACKFIRE_SCALE), 1, 2, 0)];
    }
    fx
}

/// Live particles of the shift lanes and of the pop lanes.
fn particles(fx: &ExhaustFlames) -> (usize, usize) {
    let count = |lanes: &[super::Lane]| lanes.iter().flat_map(|l| &l.emitters).map(|e| e.len()).sum();
    (count(&active(fx).lanes), count(&active(fx).pop))
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

#[test]
fn the_timing_a_car_flames_with_is_a_bikes_own_a_mod_cars_or_the_records() {
    assert_eq!(shift_timing(true, true, (0.0, 0.0)), BIKE_SHIFT_TIMING);
    assert_eq!(shift_timing(true, true, (8.0, 2.25)), BIKE_SHIFT_TIMING, "a bike ignores the class record");
    assert_eq!(shift_timing(false, true, (0.0, 0.0)), MOD_SHIFT_TIMING);
    assert_eq!(shift_timing(false, false, (3.0, 4.0)), ShiftTiming { speed: 3.0, angle: 4.0 });
}

#[test]
fn every_pop_of_a_bike_flames_at_its_pipe() {
    let mut fx = bike_flames();
    run(&mut fx, 5, &car(3, 0.0));
    fx.note_pops(1);
    fx.step(STEP, &car(3, 0.0));
    assert_eq!(active(&fx).intensity, MOD_BACKFIRE_INTENSITY);
    fx.step(STEP, &car(3, 0.0));
    assert!(fx.live() > 0, "particles leave the pipe");
    let mut neutral = bike_flames();
    neutral.note_pops(1);
    neutral.step(STEP, &car(0, 0.0));
    assert_eq!(neutral.live(), 0, "not in neutral");
}

#[test]
fn a_bike_flicks_at_a_down_shift_off_the_pedal_but_not_under_power() {
    let mut lifted = bike_flames();
    run(&mut lifted, 5, &car(4, 0.0));
    lifted.step(STEP, &car(3, 0.0));
    assert_eq!(active(&lifted).intensity, 1.0, "the blip flicks the pipe at once");
    lifted.step(STEP, &car(3, 0.0));
    assert!(lifted.live() > 0);

    let mut driven = bike_flames();
    run(&mut driven, 5, &car(4, 1.0));
    run(&mut driven, 3, &car(3, 1.0));
    assert_eq!(driven.live(), 0, "a down-shift under power is silent");
}

#[test]
fn a_bike_flicks_at_an_up_shift_under_power_not_off_the_pedal() {
    let mut driven = bike_flames();
    run(&mut driven, 5, &car(3, 1.0));
    driven.step(STEP, &car(4, 1.0));
    assert_eq!(active(&driven).intensity, 1.0);

    let mut lifted = bike_flames();
    run(&mut lifted, 5, &car(3, 0.0));
    run(&mut lifted, 3, &car(4, 0.0));
    assert_eq!(lifted.live(), 0);
}

#[test]
fn the_shift_flick_of_a_bike_is_shorter_than_a_mod_cars_blow_off() {
    let mut bike = bike_flames();
    run(&mut bike, 5, &car(4, 0.0));
    bike.step(STEP, &car(3, 0.0));
    let flick = flaming_steps(&mut bike, &car(3, 0.0));
    assert!((4..=9).contains(&flick), "{flick} more steps of flame, about 0.125 s");
    let mut car_fx = flames(true, MOD_SHIFT_TIMING, 0);
    run(&mut car_fx, 5, &car(3, 1.0));
    car_fx.step(STEP, &car(4, 1.0));
    assert!(flaming_steps(&mut car_fx, &car(4, 1.0)) > flick);
}

/// Rising edges of the flame over `steps` steps, with a pop noted at each step of `pops_at`: the flicks that were drawn, and
/// the steps each one lasted.
fn flicks(fx: &mut ExhaustFlames, steps: usize, pops_at: &[usize]) -> Vec<usize> {
    let mut lengths: Vec<usize> = Vec::new();
    let mut on = false;
    for step in 0..steps {
        if pops_at.contains(&step) {
            fx.note_pops(1);
        }
        fx.step(STEP, &car(3, 0.0));
        let lit = active(fx).intensity > 0.0;
        match (lit, on) {
            (true, false) => lengths.push(1),
            (true, true) => *lengths.last_mut().expect("a flick") += 1,
            (false, _) => {}
        }
        on = lit;
    }
    lengths
}

#[test]
fn a_bike_backfire_is_the_shift_flame_a_quarter_bigger() {
    assert_eq!(BIKE_BACKFIRE_SCALE_OVER_SHIFT, 1.25);
    const { assert!(BIKE_BACKFIRE_SCALE_OVER_SHIFT >= 1.2 && BIKE_BACKFIRE_SCALE_OVER_SHIFT <= 1.3) };
    let fx = bike_flames();
    let (shift, pop) = (active(&fx).lanes[0].emitters[0].spec(), active(&fx).pop[0].emitters[0].spec());
    for (a, b) in shift.size.iter().zip(pop.size) {
        assert!((b - a * 1.25).abs() < 1e-6, "{a} to {b}");
    }
    assert!((pop.speed / shift.speed - 1.25).abs() < 1e-6);
    assert!((pop.volume_extent - shift.volume_extent * 1.25).length() < 1e-6);
    assert_eq!(
        (pop.rate, pop.life, pop.keys, pop.colors),
        (shift.rate, shift.life, shift.keys, shift.colors),
        "same emitter"
    );
    assert_eq!(BIKE_BACKFIRE_SCALE, BIKE_FLAME_SCALE * BIKE_BACKFIRE_SCALE_OVER_SHIFT);
}

#[test]
fn a_bike_pop_draws_the_bigger_lanes_and_a_shift_the_smaller_ones() {
    let mut popped = bike_flames();
    run(&mut popped, 5, &car(3, 0.0));
    popped.note_pops(1);
    run(&mut popped, 3, &car(3, 0.0));
    let (shift, pop) = particles(&popped);
    assert!(pop > 0 && shift == 0, "a pop spawns only the backfire lanes: {shift} and {pop}");

    let mut shifted = bike_flames();
    run(&mut shifted, 5, &car(3, 1.0));
    run(&mut shifted, 3, &car(4, 1.0));
    let (shift, pop) = particles(&shifted);
    assert!(shift > 0 && pop == 0, "a shift spawns only the shift lanes: {shift} and {pop}");
}

#[test]
fn a_bike_pop_flicks_for_exactly_as_long_as_its_shift_flame() {
    let mut shifted = bike_flames();
    run(&mut shifted, 5, &car(4, 0.0));
    let shift = flicks_of_shift(&mut shifted);
    let mut popped = bike_flames();
    run(&mut popped, 5, &car(3, 0.0));
    let pop = flicks(&mut popped, 40, &[0]);
    assert_eq!(pop, vec![shift], "one pop is one flick of the shift's length");
    assert!(shift >= 7, "{shift} steps is about 0.125 s");
}

/// The steps the flame of a down-shift off the pedal lasts, counted from the shift.
fn flicks_of_shift(fx: &mut ExhaustFlames) -> usize {
    let mut steps = 0;
    for _ in 0..40 {
        fx.step(STEP, &car(3, 0.0));
        steps += usize::from(active(fx).intensity > 0.0);
    }
    steps
}

#[test]
fn a_burst_of_pops_is_that_many_flicks_in_step_with_the_pops() {
    // Three pops of the sound, 0.2 s apart (more than a flick): three separate flicks, each starting in the step its pop is due.
    let mut fx = bike_flames();
    run(&mut fx, 5, &car(3, 0.0));
    let each = flicks(&mut fx, 60, &[0, 12, 24]);
    assert_eq!(each.len(), 3, "{each:?}");
    assert!(each.iter().all(|&steps| steps == each[0]), "every flick is the same length: {each:?}");
    // Two pops closer than a flick run together, as one flame restarted.
    let mut close = bike_flames();
    run(&mut close, 5, &car(3, 0.0));
    let merged = flicks(&mut close, 60, &[0, 3]);
    assert_eq!(merged.len(), 1);
    assert_eq!(merged[0], each[0] + 3, "restarted at the second pop");
}

#[test]
fn a_plain_lift_off_without_a_pop_does_not_flame_a_bike() {
    let mut fx = bike_flames();
    run(&mut fx, 30, &car(3, 1.0));
    run(&mut fx, 60, &car(3, 0.0));
    assert_eq!(fx.live(), 0);
    assert_eq!(active(&fx).intensity, 0.0);
}
