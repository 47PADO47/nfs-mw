//! The song list of the install: 26 songs in the order of the game's list.

use blackbox_attrib::Database;
use nfsmw_data::music::{Playability, songs};

use crate::install;

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_song_list_has_26_songs_in_the_games_order() {
    let Some(dir) = install() else { return };
    let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).expect("attributes.bin");
    let list = songs(&db);
    assert_eq!(list.len(), 26);
    assert_eq!((list[0].artist.as_str(), list[0].title.as_str()), ("Styles Of Beyond", "Nine Thou (Superstars Remix)"));
    assert_eq!(list[0].playability, Playability::Both);
    assert_eq!(list[0].event & 0x00FF_FFFF, 0x00EA_F18B);
    assert_eq!(list[6].title, "Fired Up");
    assert_eq!(list[6].playability, Playability::FrontEnd);
    assert_eq!(list[25].title, "Most Wanted Mash Up");
    assert_eq!(list[25].playability, Playability::InGame);
    let count = |want: Playability| list.iter().filter(|s| s.playability == want).count();
    assert_eq!(count(Playability::FrontEnd), 8);
    assert_eq!(count(Playability::InGame), 16);
    assert_eq!(count(Playability::Both), 2);
    assert_eq!(count(Playability::Off), 0);
    assert!(list.iter().all(|s| !s.artist.is_empty() && !s.title.is_empty() && s.event != 0));
}
