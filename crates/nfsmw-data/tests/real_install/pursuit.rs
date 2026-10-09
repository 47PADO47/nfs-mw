//! The cop waves read from the install match the tables in `docs/specs/ai-pursuit-heat.md` §2.7.

use blackbox_attrib::Database;
use nfsmw_data::pursuit::heat_row;

use crate::install;

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_cop_waves_match_the_documented_table() {
    let Some(dir) = install() else { return };
    let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).expect("attributes.bin");
    let wave = |heat| {
        let row = heat_row(&db, heat).expect("a row");
        row.cops.iter().map(|c| (c.name.clone(), c.count, c.chance)).collect::<Vec<_>>()
    };
    assert_eq!(wave(1), vec![("copmidsize".to_owned(), 4, 100)]);
    assert_eq!(wave(3), vec![("copgto".to_owned(), 7, 100)]);
    assert_eq!(wave(4), vec![("copgtoghost".to_owned(), 8, 100), ("copheli".to_owned(), 1, 50)]);
    assert_eq!(wave(6)[0], ("copsportghost".to_owned(), 8, 80));
    assert_eq!(heat_row(&db, 1).unwrap().full_engagement_cops, 5);
    assert_eq!(heat_row(&db, 5).unwrap().full_engagement_cops, 25);
}
