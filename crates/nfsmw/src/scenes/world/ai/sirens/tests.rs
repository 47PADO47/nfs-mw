use super::*;

fn chase(secs: f32) -> Chase {
    Chase { on: true, heat: 1, secs, unseen: 0.0, pressing: false }
}

fn cop(id: u32) -> CarFacts {
    CarFacts {
        id,
        position: Vec3::ZERO,
        velocity: Vec3::ZERO,
        cop: true,
        in_formation: false,
        roadblock: false,
        lights_on: true,
        wrecked: false,
    }
}

fn patrol(id: u32) -> CarFacts {
    CarFacts { cop: false, lights_on: false, ..cop(id) }
}

fn states(sources: &[SirenSource]) -> Vec<State> {
    sources.iter().map(|s| s.state).collect()
}

#[test]
fn the_phase_is_the_opening_then_lost_or_running() {
    assert_eq!(phase(&chase(0.0)), Phase::Opening);
    assert_eq!(phase(&chase(4.9)), Phase::Opening);
    assert_eq!(phase(&chase(5.0)), Phase::Running);
    assert_eq!(phase(&Chase { unseen: 5.0, ..chase(60.0) }), Phase::Lost);
    assert_eq!(phase(&Chase { unseen: 4.9, ..chase(60.0) }), Phase::Running);
    assert_eq!(phase(&Chase { unseen: 9.0, ..chase(2.0) }), Phase::Opening, "the opening comes first");
}

#[test]
fn a_patrol_car_and_a_chase_that_is_off_are_silent() {
    let mut sirens = Sirens::default();
    let quiet = Chase { on: false, ..chase(0.0) };
    for _ in 0..600 {
        let out = sirens.tick(0.1, &quiet, &[patrol(1), cop(2)]);
        assert_eq!(states(&out), [State::Off, State::Off]);
    }
}

#[test]
fn a_cop_of_a_new_chase_yelps_then_wails() {
    let mut sirens = Sirens::default();
    // Born at the chase's start: the starting yelp runs 6 s from its creation, then the wail.
    let first = sirens.tick(0.1, &Chase { secs: 5.0, ..chase(5.0) }, &[cop(1)]);
    assert_eq!(first[0].state, State::Yelp);
    let mut last = State::Off;
    for i in 0..80 {
        last = sirens.tick(0.1, &Chase { secs: 5.1 + i as f32 * 0.1, ..chase(5.0) }, &[cop(1)])[0].state;
    }
    assert_eq!(last, State::Wail, "after the starting period, a cool chase wails");
}

#[test]
fn a_roadblock_car_wails_with_no_chase_and_a_dark_bar_silences_it() {
    let mut sirens = Sirens::default();
    let block = CarFacts { cop: false, roadblock: true, ..cop(1) };
    let none = Chase { on: false, ..chase(0.0) };
    assert_eq!(sirens.tick(0.1, &none, &[block])[0].state, State::Wail);
    let dark = CarFacts { lights_on: false, ..block };
    assert_eq!(sirens.tick(0.1, &none, &[dark])[0].state, State::Off);
}

#[test]
fn a_wrecked_cop_dies_and_a_cop_with_a_dark_bar_is_off() {
    let mut sirens = Sirens::default();
    let wrecked = CarFacts { cop: false, lights_on: false, wrecked: true, ..cop(1) };
    let dark = CarFacts { lights_on: false, ..cop(2) };
    let out = sirens.tick(0.1, &chase(10.0), &[wrecked, dark]);
    assert_eq!(states(&out), [State::Die, State::Off]);
}

#[test]
fn lost_contact_silences_the_cops() {
    let mut sirens = Sirens::default();
    for i in 0..200 {
        sirens.tick(0.1, &Chase { secs: 6.0 + i as f32 * 0.1, ..chase(0.0) }, &[cop(1)]);
    }
    let lost = Chase { unseen: 6.0, ..chase(40.0) };
    assert_eq!(sirens.tick(0.1, &lost, &[cop(1)])[0].state, State::Off);
}

#[test]
fn a_car_that_leaves_loses_its_machine() {
    let mut sirens = Sirens::default();
    sirens.tick(0.1, &chase(10.0), &[cop(1), cop(2)]);
    assert_eq!(sirens.machines.len(), 2);
    sirens.tick(0.1, &chase(10.1), &[cop(2)]);
    assert_eq!(sirens.machines.len(), 1);
    assert_eq!(sirens.states().len(), 1);
    sirens.clear();
    assert!(sirens.machines.is_empty());
}

#[test]
fn the_input_marks_a_cop_in_a_formation_and_the_pursuit_flag() {
    let car = CarFacts { in_formation: true, ..cop(1) };
    let pressing = Chase { pressing: true, ..chase(30.0) };
    let input = input(&pressing, true, &car);
    assert!(input.in_pursuit && input.pressing && input.in_formation && input.high_intensity);
    assert_eq!(input.phase, Phase::Running);
    let off = Chase { on: false, ..pressing };
    assert!(!super::input(&off, false, &car).in_pursuit);
}

#[test]
fn the_installed_timings_reach_the_machines() {
    let slow = nfsmw_data::pursuit_siren::SirenRow { wail_period: 30.0, ..Default::default() };
    let mut sirens = Sirens::default();
    sirens.set_data(SirenData { rows: vec![slow], ..SirenData::default() });
    let at = |secs: f32, sirens: &mut Sirens| sirens.tick(0.1, &Chase { secs, ..chase(0.0) }, &[cop(1)])[0].state;
    for i in 0..200 {
        at(6.0 + i as f32 * 0.1, &mut sirens);
    }
    // 20 s in, a default row (wail 11 s, then yelp) would be yelping; this one wails on.
    assert_eq!(at(16.0, &mut sirens), State::Wail);
}
