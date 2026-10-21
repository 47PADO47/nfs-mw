use nfsmw_data::nis::CameraLibrary;

use crate::install;

/// Prints whether a set exists for a scene name. The names are the cutscene and FMV scenes the story mode refers to.
fn describe(set: Option<&[blackbox_chunk::ice::Track]>) -> String {
    match set {
        Some(tracks) => format!("a set with {} tracks", tracks.len()),
        None => "no set".into(),
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_camera_packs_hold_sets_by_scene_name() {
    let Some(dir) = install() else { return };
    let cameras = CameraLibrary::load(&dir).expect("the camera packs of InGameB.bun");
    assert!(cameras.nis_count() > 0, "the NIS pack has no sets");
    assert!(cameras.fmv_count() > 0, "the FMV pack has no sets");
    eprintln!("NIS pack: {} sets", cameras.nis_count());
    eprintln!("FMV pack: {} sets", cameras.fmv_count());
    for name in ["IntroNis05", "IntroNisDD", "ArrestF02", "EndingNis04"] {
        eprintln!("NIS {name}: {}", describe(cameras.nis(name)));
    }
    for name in ["FMVroc02", "storyfmv_roc02", "roc02"] {
        eprintln!("FMV {name}: {}", describe(cameras.fmv(name)));
    }
}
