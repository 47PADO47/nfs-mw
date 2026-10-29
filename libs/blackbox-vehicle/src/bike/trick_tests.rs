use super::*;
use crate::bike::BikeSpec;

fn spec() -> WheelieSpec {
    BikeSpec::example().wheelie.unwrap()
}

fn situation(lift_deg: f32) -> Situation {
    Situation { lift: lift_deg.to_radians(), rate: 0.0, speed: 20.0, weight: 0.0, throttle: 1.0, front_brake: 0.0 }
}

#[test]
fn a_lifting_front_cuts_the_throttle_through_the_engine_and_leaves_the_pedal() {
    let s = spec();
    let none = interventions(&s, &situation(0.0));
    assert_eq!((none.throttle, none.engine_cut), (1.0, 0.0));
    let some = interventions(&s, &situation(2.0));
    assert!(some.engine_cut > 0.3 && some.engine_cut < 0.7, "{}", some.engine_cut);
    // The cut is the engine's: the pedal, which sets the wheelie target, is untouched.
    assert_eq!(some.throttle, 1.0);
    assert_eq!(interventions(&s, &situation(5.0)).engine_cut, 1.0);
}

#[test]
fn a_lift_speed_cuts_the_throttle_before_the_angle_does() {
    let s = spec();
    let fast = Situation { rate: 0.5, ..situation(0.5) };
    let cut = interventions(&s, &fast).engine_cut;
    assert!(cut > 0.75 && cut <= 1.0, "{cut}");
    let slow = Situation { rate: 0.5, ..situation(0.0) };
    assert_eq!(interventions(&s, &slow).engine_cut, 0.0, "a level bike is not lifting");
    // A nose going down is not a wheelie.
    assert_eq!(interventions(&s, &Situation { rate: -0.5, ..situation(-1.0) }).engine_cut, 0.0);
}

#[test]
fn the_cut_relaxes_at_a_launch() {
    let s = spec();
    let standing = Situation { speed: 0.0, ..situation(20.0) };
    assert_eq!(interventions(&s, &standing).engine_cut, 0.0);
    let rolling = Situation { speed: s.help.launch_speed * 0.5, ..situation(20.0) };
    let mid = interventions(&s, &rolling).engine_cut;
    assert!(mid > 0.0 && mid < 1.0, "{mid}");
    assert_eq!(interventions(&s, &Situation { speed: s.help.launch_speed * 1.5, ..situation(20.0) }).engine_cut, 1.0);
}

#[test]
fn asking_for_a_wheelie_moves_the_cut_to_the_forced_range_and_onto_the_pedal() {
    let s = spec();
    let asked = Situation { weight: -1.0, ..situation(20.0) };
    let held = interventions(&s, &asked);
    assert_eq!((held.throttle, held.engine_cut), (1.0, 0.0));
    let past = interventions(&s, &Situation { weight: -1.0, ..situation(40.0) });
    assert!(past.throttle < 1.0 && past.engine_cut == 0.0, "{past:?}");
    // Without the throttle there is no wheelie to force.
    assert_eq!(interventions(&s, &Situation { weight: -1.0, throttle: 0.05, ..situation(20.0) }).throttle, 0.05);
}

#[test]
fn a_lifting_rear_eases_the_front_brake() {
    let s = spec();
    let braking = Situation { throttle: 0.0, front_brake: 1.0, ..situation(0.0) };
    assert_eq!(interventions(&s, &braking).brake, 1.0);
    let down = interventions(&s, &Situation { lift: -2.0_f32.to_radians(), ..braking });
    assert!(down.brake > 0.3 && down.brake < 0.7, "{}", down.brake);
    // The brake help acts on the pedal itself in the example, so the stoppie target eases with it.
    assert_eq!(down.brake_pedal, down.brake);
    assert_eq!(interventions(&s, &Situation { lift: -6.0_f32.to_radians(), ..braking }).brake, 0.0);
    // Asking for a stoppie hands the pedal over to the forced range, which starts much higher.
    let asked = Situation { weight: 1.0, lift: -6.0_f32.to_radians(), ..braking };
    assert_eq!(interventions(&s, &asked).brake, 1.0);
}

#[test]
fn the_auto_rider_shifts_the_weight_against_a_lift_and_rests_on_the_pedals() {
    let help = spec().help;
    let rest = |s: &Situation| rider_position(&help, s, 0.0, 1.0);
    // Hands off, throttle open: forward; the brake: back; both: in between.
    assert_eq!(rest(&Situation { throttle: 0.5, ..situation(0.0) }), 1.0);
    assert_eq!(rest(&Situation { throttle: 0.0, front_brake: 0.5, ..situation(0.0) }), -1.0);
    assert_eq!(rest(&Situation { throttle: 0.0, ..situation(0.0) }), 0.0);
    // A lifting front sends him forward, a lifting rear back, from any rest.
    assert_eq!(rest(&Situation { throttle: 0.0, ..situation(5.0) }), 1.0);
    assert_eq!(rest(&Situation { throttle: 0.0, ..situation(-5.0) }), -1.0);
    let part = rest(&Situation { throttle: 0.0, ..situation(3.8) });
    assert!(part > 0.2 && part < 0.8, "{part}");
}

#[test]
fn a_firm_lever_travel_takes_the_weight_from_the_auto_rider() {
    let help = spec().help;
    let s = Situation { weight: -0.9, ..situation(10.0) };
    assert_eq!(rider_position(&help, &s, -0.9, 0.0), -0.9);
    // Inside the dead zone the same lift sends him forward.
    let inside = Situation { weight: -0.5, ..situation(10.0) };
    assert_eq!(rider_position(&help, &inside, -0.5, 1.0), 1.0);
}

#[test]
fn the_weight_moves_at_a_square_root_rate() {
    let help = spec().help;
    let s = Situation { throttle: 0.5, ..situation(0.0) };
    let first = rider_position(&help, &s, 0.0, 1.0 / 60.0);
    assert!(first > 0.0 && first < 0.5, "{first}");
    let mut x = 0.0;
    for _ in 0..60 {
        x = rider_position(&help, &s, x, 1.0 / 60.0);
    }
    assert_eq!(x, 1.0);
}

#[test]
fn without_an_auto_rider_the_weight_is_the_players() {
    let mut help = spec().help;
    help.auto_rider = None;
    assert_eq!(rider_position(&help, &Situation { weight: 0.3, throttle: 1.0, ..situation(10.0) }, 0.0, 1.0), 0.3);
}

fn tuck_rule() -> AutoTuckSpec {
    // The base rider help of the reference: 90 km/h, lean limit 1, any throttle, brake under 0.1.
    AutoTuckSpec { min_speed: 90.0 / 3.6, max_normalised_lean: 1.0, min_throttle: 0.0, max_brake: 0.1 }
}

fn tucking() -> TuckSituation {
    TuckSituation { speed: 30.0, normalised_lean: 0.2, weight: 0.0, throttle: 0.5, front_brake: 0.0 }
}

#[test]
fn the_rider_tucks_in_when_fast_with_the_throttle_open_and_the_brake_released() {
    let rule = tuck_rule();
    assert!(tuck_requested(Some(&rule), 0.75, 0.0, &tucking()));
    let slow = TuckSituation { speed: 25.0, ..tucking() };
    assert!(!tuck_requested(Some(&rule), 0.75, 0.0, &slow), "90 km/h is the least");
    let closed = TuckSituation { throttle: 0.0, ..tucking() };
    assert!(!tuck_requested(Some(&rule), 0.75, 0.0, &closed), "the throttle must be open beyond its minimum");
    let braking = TuckSituation { front_brake: 0.1, ..tucking() };
    assert!(!tuck_requested(Some(&rule), 0.75, 0.0, &braking));
    let leaning = TuckSituation { normalised_lean: -1.0, ..tucking() };
    assert!(!tuck_requested(Some(&rule), 0.75, 0.0, &leaning), "the lean limit holds on either side");
    let weight = TuckSituation { weight: -0.8, ..tucking() };
    assert!(!tuck_requested(Some(&rule), 0.75, 0.0, &weight), "the player has the weight beyond the dead zone");
    let free = AutoTuckSpec { max_normalised_lean: -1.0, ..rule };
    assert!(tuck_requested(Some(&free), 0.75, 0.0, &leaning), "a negative lean limit is not tested");
}

#[test]
fn a_held_tuck_input_tucks_him_whatever_the_rule_says_and_without_a_rule_nothing_does() {
    let slow = TuckSituation { speed: 3.0, ..tucking() };
    assert!(tuck_requested(None, 0.75, 1.0, &slow));
    assert!(!tuck_requested(None, 0.75, 0.5, &slow), "one half is not above one half");
    assert!(!tuck_requested(None, 0.75, 0.0, &tucking()));
}

#[test]
fn the_tuck_level_follows_the_action_at_the_rate_of_the_input_handler() {
    let mut level = 0.0;
    for _ in 0..30 {
        level = tuck_level(level, true, TUCK_LEVEL_RATE, TUCK_LEVEL_RATE, 1.0 / 60.0);
    }
    assert!((level - 1.0).abs() < 1e-5, "half a second at 2 per second: {level}");
    level = tuck_level(level, false, 2.0, 2.0, 0.25);
    assert!((level - 0.5).abs() < 1e-6);
    assert_eq!(tuck_level(0.1, false, 2.0, 2.0, 1.0), 0.0);
    assert_eq!(tuck_level(0.9, true, 2.0, 2.0, 1.0), 1.0);
}

#[test]
fn a_bike_with_the_rule_tucks_by_itself_at_speed_and_comes_up_when_it_brakes() {
    use crate::bike::{Bike, BikeInput};
    use crate::ground::FlatGround;
    let mut spec = BikeSpec::example();
    spec.wheelie.as_mut().unwrap().help.auto_tuck = Some(tuck_rule());
    let ground = FlatGround::new(0.0);
    let mut bike = Bike::new(spec);
    assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, 40.0));
    let cruise = BikeInput { throttle: 0.6, ..Default::default() };
    for _ in 0..120 {
        bike.step(1.0 / 120.0, &cruise, &ground);
    }
    assert_eq!(bike.tuck_action(), 1.0);
    assert!(bike.tuck_level() > 0.9, "{}", bike.tuck_level());
    let brake = BikeInput { front_brake: 0.5, ..cruise };
    for _ in 0..120 {
        bike.step(1.0 / 120.0, &brake, &ground);
    }
    assert_eq!(bike.tuck_action(), 0.0);
    assert!(bike.tuck_level() < 0.2, "{}", bike.tuck_level());
}
