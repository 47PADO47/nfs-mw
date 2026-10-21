use super::*;

const TUNING: Tuning = Tuning {
    init_min_period: 1.0,
    init_variation: 1.0,
    wail_period: 11.0,
    max_yelp_time: 6.0,
    scream_period: 12.0,
    max_scream_time: 3.0,
};

fn chase() -> Input {
    Input { in_pursuit: true, lights_on: true, ..Input::default() }
}

/// Runs `machine` for `seconds` at 10 Hz from `from` and returns the states seen, one per tick.
fn run(machine: &mut Machine, gap: &mut OpeningGap, from: f64, seconds: f64, input: Input) -> Vec<State> {
    let ticks = (seconds * 10.0) as usize;
    (1..=ticks).map(|i| machine.update(from + i as f64 * 0.1, &input, &TUNING, gap, 0.5)).collect()
}

#[test]
fn state_words_are_the_originals() {
    assert_eq!([State::Off, State::Wail, State::Yelp, State::Scream, State::Die].map(State::word), [-1, 0, 1, 2, 3]);
}

#[test]
fn a_car_not_in_pursuit_is_silent() {
    let mut machine = Machine::new(0.0);
    let mut gap = OpeningGap::default();
    let seen = run(&mut machine, &mut gap, 0.0, 30.0, Input { in_pursuit: false, lights_on: true, ..chase() });
    assert!(seen.iter().all(|&s| s == State::Off));
}

#[test]
fn a_new_chase_yelps_for_six_seconds_then_wails_and_yelps_in_turn() {
    let mut machine = Machine::new(0.0);
    let mut gap = OpeningGap::default();
    let first = run(&mut machine, &mut gap, 0.0, 5.9, chase());
    assert!(first.iter().all(|&s| s == State::Yelp), "{first:?}");
    // Cool chase: after the start the siren wails, and yelps from 11 s after the wail began until 17 s.
    let mut states = run(&mut machine, &mut gap, 5.9, 40.0, chase());
    states.dedup();
    assert_eq!(states, [State::Wail, State::Yelp, State::Wail, State::Yelp, State::Wail]);
}

#[test]
fn the_yelp_interlude_is_six_seconds_after_eleven_of_wail() {
    let mut machine = Machine::new(0.0);
    let mut gap = OpeningGap::default();
    run(&mut machine, &mut gap, 0.0, 6.5, chase());
    let wail_began = 6.1;
    let at = |t: f64, machine: &mut Machine, gap: &mut OpeningGap| machine.update(t, &chase(), &TUNING, gap, 0.5);
    assert_eq!(at(wail_began + 10.5, &mut machine, &mut gap), State::Wail);
    assert_eq!(at(wail_began + 11.5, &mut machine, &mut gap), State::Yelp);
    assert_eq!(at(wail_began + 16.5, &mut machine, &mut gap), State::Yelp);
    assert_eq!(at(wail_began + 17.5, &mut machine, &mut gap), State::Wail);
}

#[test]
fn a_hot_chase_screams_three_seconds_then_yelps_then_yelps_on() {
    let mut machine = Machine::new(0.0);
    let mut gap = OpeningGap::default();
    let hot = Input { high_intensity: true, ..chase() };
    let mut seen = run(&mut machine, &mut gap, 0.0, 6.5, hot);
    seen.dedup();
    assert_eq!(seen, [State::Scream, State::Yelp]);
    // After the start a hot chase keeps the yelp until the wail comes round (it does not wail at once).
    assert_eq!(machine.state(), State::Yelp);
}

#[test]
fn a_pressing_formation_screams_when_hot_for_up_to_three_seconds_and_yelps_when_cool() {
    let mut machine = Machine::new(0.0);
    let mut gap = OpeningGap::default();
    run(&mut machine, &mut gap, 0.0, 7.0, chase());
    let cool = Input { pressing: true, in_formation: true, ..chase() };
    assert_eq!(machine.update(8.0, &cool, &TUNING, &mut gap, 0.5), State::Yelp);
    let hot = Input { high_intensity: true, ..cool };
    assert_eq!(machine.update(13.0, &hot, &TUNING, &mut gap, 0.5), State::Scream);
    assert_eq!(machine.update(14.0, &hot, &TUNING, &mut gap, 0.5), State::Scream);
    assert_eq!(machine.update(16.5, &hot, &TUNING, &mut gap, 0.5), State::Yelp, "the scream is at most three seconds");
    // The yelp re-arms the clock every three seconds, so a scream comes back only after a pause in the pressing.
    assert_eq!(machine.update(20.0, &hot, &TUNING, &mut gap, 0.5), State::Yelp);
    assert_eq!(machine.update(40.0, &hot, &TUNING, &mut gap, 0.5), State::Scream);
}

#[test]
fn a_pressing_car_outside_the_formation_keeps_to_its_wail() {
    let mut machine = Machine::new(0.0);
    let mut gap = OpeningGap::default();
    run(&mut machine, &mut gap, 0.0, 8.0, chase());
    let outside = Input { pressing: true, in_formation: false, ..chase() };
    assert_eq!(machine.update(8.5, &outside, &TUNING, &mut gap, 0.5), State::Wail);
}

#[test]
fn the_opening_exchange_is_silent_but_for_half_second_yelps() {
    let mut machine = Machine::new(0.0);
    let mut gap = OpeningGap::default();
    let opening = Input { phase: Phase::Opening, ..chase() };
    let seen = run(&mut machine, &mut gap, 0.0, 4.9, opening);
    // The gap is 1.5 s with random 0.5: quiet to 1.5 s, a yelp of 0.5 s, quiet 1.5 s, the next yelp.
    let yelp_ticks = seen.iter().filter(|&&s| s == State::Yelp).count();
    assert!((10..=14).contains(&yelp_ticks), "two yelps of about half a second each: {seen:?}");
    assert!(seen.iter().all(|&s| s == State::Off || s == State::Yelp));
    assert_eq!(seen[..14], [State::Off; 14]);
    assert_eq!(gap.current(), Some(1.5));
}

#[test]
fn every_car_shares_the_opening_gap() {
    let tuning = TUNING;
    let mut gap = OpeningGap::default();
    let mut first = Machine::new(0.0);
    let mut second = Machine::new(0.0);
    let opening = Input { phase: Phase::Opening, ..chase() };
    first.update(0.1, &opening, &tuning, &mut gap, 0.0);
    assert_eq!(gap.current(), Some(1.0));
    // A different random number does not change the gap that was drawn.
    second.update(0.1, &opening, &tuning, &mut gap, 1.0);
    assert_eq!(gap.current(), Some(1.0));
}

#[test]
fn lost_contact_silences_the_siren_and_restarts_the_starting_yelp() {
    let mut machine = Machine::new(0.0);
    let mut gap = OpeningGap::default();
    run(&mut machine, &mut gap, 0.0, 20.0, chase());
    let lost = Input { phase: Phase::Lost, ..chase() };
    assert_eq!(machine.update(21.0, &lost, &TUNING, &mut gap, 0.5), State::Off);
    assert_eq!(machine.update(30.0, &lost, &TUNING, &mut gap, 0.5), State::Off);
    // Contact returns: six seconds of starting yelp again.
    assert_eq!(machine.update(31.0, &chase(), &TUNING, &mut gap, 0.5), State::Yelp);
    assert_eq!(machine.update(35.9, &chase(), &TUNING, &mut gap, 0.5), State::Yelp);
    assert_eq!(machine.update(36.5, &chase(), &TUNING, &mut gap, 0.5), State::Wail);
}

#[test]
fn a_roadblock_car_always_wails() {
    let mut machine = Machine::new(0.0);
    let mut gap = OpeningGap::default();
    let block = Input { in_pursuit: false, in_roadblock: true, lights_on: true, ..Input::default() };
    let seen = run(&mut machine, &mut gap, 0.0, 30.0, block);
    assert!(seen.iter().all(|&s| s == State::Wail));
    let hot = Input { high_intensity: true, in_pursuit: true, ..block };
    assert_eq!(machine.update(31.0, &hot, &TUNING, &mut gap, 0.5), State::Wail);
}

#[test]
fn a_dark_light_bar_is_silent_and_a_wrecked_car_dies() {
    let mut machine = Machine::new(0.0);
    let mut gap = OpeningGap::default();
    run(&mut machine, &mut gap, 0.0, 10.0, chase());
    let dark = Input { lights_on: false, ..chase() };
    assert_eq!(machine.update(10.5, &dark, &TUNING, &mut gap, 0.5), State::Off);
    let wrecked = Input { destroyed: true, ..dark };
    assert_eq!(machine.update(11.0, &wrecked, &TUNING, &mut gap, 0.5), State::Die);
    let block = Input { in_roadblock: true, ..dark };
    assert_eq!(machine.update(12.0, &block, &TUNING, &mut gap, 0.5), State::Off, "a roadblock car with a dark bar");
    // The lights come back on: the siren returns.
    assert_ne!(machine.update(13.0, &chase(), &TUNING, &mut gap, 0.5), State::Off);
}

#[test]
fn a_new_voice_wails_until_it_adopts_the_cars_state() {
    let mut voice = Voice::new(0.0, 0.0);
    // Six seconds of silence about the car's state: the voice plays the wail, even when the car yelps.
    for i in 0..59 {
        assert_eq!(voice.update(f64::from(i) * 0.1, 0.1, State::Yelp, 0.5), Some(0));
    }
    assert_eq!(voice.held(), State::Off);
    assert_eq!(voice.update(6.1, 0.2, State::Yelp, 0.5), Some(1));
    assert_eq!(voice.held(), State::Yelp);
}

#[test]
fn a_scream_is_adopted_at_once() {
    let mut voice = Voice::new(0.0, 1.0);
    assert_eq!(voice.update(0.1, 0.1, State::Scream, 0.0), Some(2));
}

#[test]
fn a_voice_follows_the_car_after_a_random_wait() {
    let mut voice = Voice::new(0.0, 0.0);
    voice.update(6.5, 7.0, State::Wail, 1.0);
    assert_eq!(voice.held(), State::Wail);
    // The wait is now three seconds (random 1.0): a change of the car's state is not seen for that long.
    assert_eq!(voice.update(7.0, 1.0, State::Yelp, 0.0), Some(0));
    assert_eq!(voice.update(8.0, 1.0, State::Yelp, 0.0), Some(0));
    assert_eq!(voice.update(10.1, 2.2, State::Yelp, 0.0), Some(1));
}

#[test]
fn a_voice_is_off_while_the_car_is_off() {
    let mut voice = Voice::new(0.0, 0.0);
    assert_eq!(voice.update(0.1, 0.1, State::Off, 0.5), None);
    assert_eq!(voice.update(0.2, 0.1, State::Scream, 0.5), Some(2));
    assert_eq!(voice.update(0.3, 0.1, State::Off, 0.5), None);
}

#[test]
fn a_dying_siren_is_switched_off_after_ten_seconds() {
    let mut voice = Voice::new(0.0, 0.0);
    voice.update(7.0, 8.0, State::Wail, 0.0);
    voice.update(7.1, 0.1, State::Die, 0.0);
    assert_eq!(voice.held(), State::Die);
    assert_eq!(voice.update(8.0, 0.9, State::Die, 0.0), Some(3));
    assert_eq!(voice.update(17.0, 9.0, State::Die, 0.0), Some(3));
    assert_eq!(voice.update(17.2, 0.2, State::Die, 0.0), None);
    // The voice stays off until it is attached again.
    assert_eq!(voice.update(18.0, 0.8, State::Wail, 0.0), None, "a wrecked siren stays off");
}
