//! `nfsmw.pursuit` round trips: the SDK wrapper calls the host table, the trampolines check the arguments and call the fake.

use std::rc::Rc;

use blackbox_modding::abi::make_handle;
use blackbox_modding::sdk::{Error, VehicleId};

use super::fake::FakePursuit;
use super::pursuit_of;
use crate::host::pursuit_table;
use crate::sdk::{CopRecord, CopTable, EndMode, HeatRow};

fn sample_row() -> HeatRow {
    HeatRow {
        cops: vec![
            CopRecord { car: "copmidsize".to_owned(), count: 4, chance: 60 },
            CopRecord { car: "copheli".to_owned(), count: 1, chance: 10 },
        ],
        patrol_cars: 2,
        full_engagement_cops: 6,
        first_four_interval: 10.0,
        spawn_interval: 5.0,
    }
}

#[test]
fn heat_is_clamped_by_the_backend_and_nan_is_refused_by_the_host() {
    let backend = Rc::new(FakePursuit::default());
    let table = pursuit_table(backend.clone(), "example.mod");
    let pursuit = pursuit_of(table.header());

    assert_eq!(pursuit.set_heat(12.0), Ok(()));
    assert_eq!(pursuit.heat(), Ok(10.0));
    assert_eq!(pursuit.set_heat(f32::NAN), Err(Error::Arg));
    assert_eq!(backend.heat.get(), 10.0);
}

#[test]
fn a_cop_wave_round_trips_and_a_bad_one_is_refused() {
    let backend = Rc::new(FakePursuit::default());
    let table = pursuit_table(backend, "example.mod");
    let pursuit = pursuit_of(table.header());
    let row = sample_row();

    assert_eq!(pursuit.set_cop_table(CopTable::Race, 4, &row), Ok(()));
    assert_eq!(pursuit.cop_table(CopTable::Race, 4), Ok(row.clone()));
    assert_eq!(pursuit.cop_table(CopTable::Roam, 4).map(|wave| wave.cops.len()), Ok(0));

    let mut over_hundred = row.clone();
    over_hundred.cops[0].chance = 101;
    assert_eq!(pursuit.set_cop_table(CopTable::Roam, 4, &over_hundred), Err(Error::Arg));
    assert_eq!(pursuit.set_cop_table(CopTable::Roam, 11, &row), Err(Error::Arg));
    assert_eq!(pursuit.cop_table(CopTable::Roam, 0), Err(Error::Arg));
}

#[test]
fn a_chase_starts_reports_and_ends_through_the_table() {
    let backend = Rc::new(FakePursuit::default());
    let table = pursuit_table(backend.clone(), "example.mod");
    let pursuit = pursuit_of(table.header());
    let vehicle = VehicleId::from_raw(make_handle(5, 1));

    assert_eq!(pursuit.is_pursuing(), Ok(false));
    assert_eq!(pursuit.start(0, None), Err(Error::Arg));
    assert_eq!(pursuit.start(11, None), Err(Error::Arg));
    assert_eq!(pursuit.start(3, Some(vehicle)), Ok(()));
    assert_eq!(backend.started.borrow().as_slice(), &[(3, Some(make_handle(5, 1)))]);

    assert_eq!(pursuit.is_pursuing(), Ok(true));
    assert_eq!(pursuit.busted(), Ok(0.25));
    assert_eq!(pursuit.cops(), Ok(3));

    assert_eq!(pursuit.end(EndMode::Evade), Ok(()));
    assert_eq!(backend.ended.borrow().as_slice(), &[EndMode::Evade]);
    assert_eq!(pursuit.is_pursuing(), Ok(false));
}

#[test]
fn a_panicking_backend_comes_back_as_an_error_and_the_host_goes_on() {
    let backend = Rc::new(FakePursuit { panic_on_heat: true, ..FakePursuit::default() });
    let table = pursuit_table(backend, "example.mod");
    let pursuit = pursuit_of(table.header());

    assert_eq!(pursuit.heat(), Err(Error::Panic));
    assert_eq!(pursuit.is_pursuing(), Ok(false));
}
