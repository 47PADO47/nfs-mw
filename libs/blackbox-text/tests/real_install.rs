//! Against the user's own install; ignored without `NFSMW_GAME_DIR`.

use blackbox_text::{StringTable, label_hash};

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn english_has_the_speed_units() {
    let Some(dir) = std::env::var_os("NFSMW_GAME_DIR") else { return };
    let bytes = std::fs::read(std::path::Path::new(&dir).join("LANGUAGES/English.bin")).unwrap();
    let t = StringTable::from_file(&bytes).unwrap();
    assert!(t.len() > 4000, "{} strings", t.len());
    for label in [0x8569a25fu32, 0x8569ab44, 0x00000017, 0x0000f078, 0x0000c5c4] {
        println!("{label:08x}: {:?}", t.get(label));
    }
    assert_eq!(t.get(0x8569a25f).as_deref().map(str::to_uppercase).as_deref(), Some("KM/H"));
    assert!(t.get(label_hash("SUBTITLE_BL_1")).is_some());
}
