//! `nfsmw.career` and `nfsmw.races` round trips: the unsupported slots answer unsupported, and the calls the fakes override
//! come back through the wrappers, with their records and names intact.

use std::mem::size_of;
use std::rc::Rc;

use blackbox_modding::sdk::Error;

use super::fake::{FakeCareer, FakeRaces};
use super::{callback_on_main, career_of, races_of};
use crate::abi::BbMilestoneDef;
use crate::host::career::{CareerBackend, career_table};
use crate::host::races::{RacesBackend, races_table};
use crate::sdk::{MilestoneDef, OwnedCar, RaceDef, RaceState};

/// A career backend with no overrides at all.
struct NoCareer;

impl CareerBackend for NoCareer {}

/// A races backend with no overrides: the race runtime is not built.
struct NoRaces;

impl RacesBackend for NoRaces {}

#[test]
fn unbuilt_career_and_races_answer_unsupported() {
    let _scope = callback_on_main();
    let career_host = career_table(Rc::new(NoCareer), "example.mod");
    let career = career_of(career_host.header());
    assert_eq!(career.cash(), Err(Error::Unsupported));
    assert_eq!(career.rap_sheet().map(|_| ()), Err(Error::Unsupported));
    assert_eq!(career.blacklist().map(|_| ()), Err(Error::Unsupported));
    assert_eq!(career.save(), Err(Error::Unsupported));
    assert_eq!(career.register_safehouse(1), Err(Error::Unsupported));

    let races_table = races_table(Rc::new(NoRaces), "example.mod");
    let races = races_of(races_table.header());
    assert_eq!(races.list(), Err(Error::Unsupported));
    assert_eq!(races.current(), Err(Error::Unsupported));
    assert_eq!(races.start(1), Err(Error::Unsupported));
}

#[test]
fn cash_and_a_milestone_definition_round_trip() {
    let _scope = callback_on_main();
    let backend = Rc::new(FakeCareer::default());
    let table = career_table(backend.clone(), "example.mod");
    let career = career_of(table.header());

    assert_eq!(career.cash(), Ok(1234));
    assert_eq!(career.add_cash(-50, "fine"), Ok(()));
    assert_eq!(*backend.reasons.borrow(), vec!["-50:fine".to_owned()]);

    let def =
        MilestoneDef { key: "example.mod:first".to_owned(), name: "First".to_owned(), goal: 2.5, reward_bounty: 100 };
    assert_eq!(career.register_milestone(&def), Ok(7));
    let seen: BbMilestoneDef = backend.milestone_defs.borrow()[0];
    assert_eq!(seen.size as usize, size_of::<BbMilestoneDef>());
    assert_eq!(seen.name.text(), Some("First"));
    assert_eq!(seen.key.text(), Some("example.mod:first"));
    assert_eq!((seen.goal, seen.reward_bounty), (2.5, 100));
}

#[test]
fn a_name_longer_than_the_host_takes_is_refused_before_the_call() {
    let _scope = callback_on_main();
    let backend = Rc::new(FakeCareer::default());
    let table = career_table(backend.clone(), "example.mod");
    let career = career_of(table.header());

    let def = MilestoneDef { key: "k".to_owned(), name: "x".repeat(200), goal: 1.0, reward_bounty: 0 };
    assert_eq!(career.register_milestone(&def), Err(Error::Arg));
    assert!(backend.milestone_defs.borrow().is_empty());
}

#[test]
fn a_garage_car_comes_back_as_an_owned_value() {
    let _scope = callback_on_main();
    let backend = Rc::new(FakeCareer::default());
    let table = career_table(backend, "example.mod");
    let career = career_of(table.header());

    assert_eq!(career.garage(), Ok(vec![OwnedCar { car: 42, current: true, name: "Zonda".to_owned() }]));
}

#[test]
fn a_custom_race_and_the_running_race_round_trip() {
    let _scope = callback_on_main();
    let backend = Rc::new(FakeRaces::default());
    let table = races_table(backend.clone(), "example.mod");
    let races = races_of(table.header());

    let def = RaceDef {
        key: "example.mod:sprint".to_owned(),
        name: "Sprint".to_owned(),
        kind: 0,
        laps: 1,
        reward_cash: 500,
        start: [1.0, 2.0, 0.0],
        start_heading: 0.5,
    };
    let checkpoints = [[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [20.0, 0.0, 0.0]];
    assert_eq!(races.register(&def, &checkpoints), Ok(9));
    assert_eq!(*backend.checkpoints_seen.borrow(), vec![3]);
    assert_eq!(*backend.names.borrow(), vec!["Sprint".to_owned()]);

    let running = races.current();
    assert_eq!(running, Ok(Some(RaceState { id: 9, lap: 2, checkpoint: 3, position: 1, elapsed: 12.5 })));
}

#[test]
fn a_call_outside_a_callback_on_the_main_thread_is_refused() {
    blackbox_modding::host::gate::mark_main_thread();
    let table = career_table(Rc::new(FakeCareer::default()), "example.mod");
    let career = career_of(table.header());
    assert_eq!(career.cash(), Err(Error::NotInCallback));
}
