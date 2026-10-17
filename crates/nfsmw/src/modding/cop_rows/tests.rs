//! The cop wave overrides: what a mod sets is read back, lands on the next chase's rows only for the heat table, and converts
//! to the ABI's form and back.

use nfsmw_data::pursuit::{CopRecord, HeatRow};
use nfsmw_modding::abi::CopTable;

use super::{get, row_of, set, spec_of, with_heat_overrides, without_models};

fn row(name: &str, count: u32) -> HeatRow {
    HeatRow {
        cops: vec![CopRecord { name: name.to_owned(), count, chance: 100 }],
        patrol_cars: 2,
        full_engagement_cops: 4,
        time_between_first_four_spawns: 3.0,
        time_between_cop_spawns: 9.0,
    }
}

#[test]
fn a_set_wave_is_read_back_and_a_new_one_replaces_it() {
    assert!(get(CopTable::Roam, 3).is_none());
    set(CopTable::Roam, 3, row("copmidsize", 2));
    set(CopTable::Roam, 3, row("copmidsize", 5));
    assert_eq!(get(CopTable::Roam, 3).map(|r| r.cops[0].count), Some(5));
    assert!(get(CopTable::Race, 3).is_none(), "the race table is its own");
}

#[test]
fn a_new_chase_takes_the_heat_overrides_and_nothing_else() {
    set(CopTable::Roam, 2, row("coproadpolice", 3));
    set(CopTable::Race, 1, row("copracer", 1));
    let install = vec![row("copcheap", 1), row("copmidsize", 1), row("copsuv", 1)];
    let rows = with_heat_overrides(install);
    assert_eq!(rows[0].cops[0].name, "copcheap", "level 1 has no override");
    assert_eq!(rows[1].cops[0].name, "coproadpolice", "level 2 is replaced");
    assert_eq!(rows[2].cops[0].name, "copsuv", "level 3 has no override");
    assert_eq!(rows.len(), 3, "the rows keep their count");
}

#[test]
fn a_wave_converts_to_the_abi_form_and_back() {
    let original = row("copmidsize", 4);
    let spec = spec_of(&original);
    assert_eq!(spec.row.cop_count, 1);
    assert_eq!((spec.row.patrol_cars, spec.row.full_engagement_cops), (2, 4));
    assert_eq!((spec.row.first_four_interval, spec.row.spawn_interval), (3.0, 9.0));
    assert_eq!(row_of(&spec), Some(original));
}

#[test]
fn a_mod_wave_names_its_car_so_the_chase_loads_it() {
    set(CopTable::Roam, 2, row("copmod", 2));
    let rows = with_heat_overrides(vec![row("copcheap", 1), row("copmidsize", 1)]);
    let names = nfsmw_data::pursuit::wave_names(&rows);
    assert_eq!(names, vec!["copcheap", "copmod"], "the car the mod named is one to load");
}

#[test]
fn a_car_that_did_not_load_leaves_its_entries_out_and_keeps_the_helicopter() {
    let mut wave = row("copcheap", 1);
    wave.cops.push(CopRecord { name: "copbad".to_owned(), count: 3, chance: 50 });
    wave.cops.push(CopRecord { name: "copheli".to_owned(), count: 1, chance: 100 });
    let rows = without_models(vec![wave.clone()], &["COPBAD".to_owned()]);
    let names: Vec<&str> = rows[0].cops.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, vec!["copcheap", "copheli"]);
    assert_eq!(rows[0].patrol_cars, wave.patrol_cars, "the rest of the row is kept");
}

#[test]
fn nothing_missing_leaves_the_rows_as_they_are() {
    let rows = vec![row("copcheap", 1), row("copmidsize", 2)];
    assert_eq!(without_models(rows.clone(), &[]), rows);
}
