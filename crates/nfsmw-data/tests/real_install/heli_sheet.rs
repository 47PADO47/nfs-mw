//! The helicopter height sheet of the open city (`docs/formats/pursuit-data.md` section 1): the chunks of the stream file
//! parse to the triangle count the format doc records, and the sheet covers the drivable map.

use nfsmw_data::world::{DEFAULT_TRACK, WorldIndex, heli_sheet};

use crate::install;

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_sheet_has_the_documented_triangles() {
    let Some(dir) = install() else { return };
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    let sheet = heli_sheet::read(&dir, &index.stream_file).expect("the stream file walks");
    // 435 drivable sections, 51 504 triangles in total (the format doc, verified on the install).
    assert_eq!(sheet.len(), 51_504);
}
