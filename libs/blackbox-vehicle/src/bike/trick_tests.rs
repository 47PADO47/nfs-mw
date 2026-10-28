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
