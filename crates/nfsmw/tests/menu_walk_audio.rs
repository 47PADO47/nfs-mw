//! AUDIO-030: a hidden `play` session walks the boot flow and the main menu with the silent audio output, and every
//! front-end cue the walk must start is checked in the audio trace. Needs the game, a display (the window stays hidden
//! and off-screen) and the binary: `NFSMW_GAME_DIR=<install> cargo test -p nfsmw --test menu_walk_audio -- --ignored`.
//! `NFSMW_WALK_EXE` runs another build of the binary (a release build walks faster).
use std::path::{Path, PathBuf};
use std::process::Command;

/// The walk (`docs/testing.md`, "Menu sound walk"): the boot flow as an unscripted start runs it, then the original's
/// reference key sequence on the main menu, Quick Race, an Options slider and the career menu.
const WALK: &str = "until WS_MW_LS_Splash.fng; wait 10; accept; until MC_List.fng; wait 2; accept; until Dialog.fng; \
    wait 1; accept; until MainMenu.fng; wait 2; \
    right; wait 1; right; wait 1; right; wait 1; left; wait 1; left; wait 1; left; wait 1; \
    accept; until MainMenu_Sub.fng; wait 2; down; wait 1; down; wait 1; up; wait 1; back; until MainMenu.fng; wait 2; \
    right; wait 1; accept; wait 3; back; until MainMenu.fng; wait 2; \
    left; wait 1; left; wait 1; left; wait 1; left; wait 1; left; wait 1; left; wait 1; left; wait 1; left; wait 1; \
    right; wait 1; right; wait 1; accept; until MainMenu_Sub.fng; wait 2; right; wait 1; back; until MainMenu.fng; \
    wait 2; left; wait 1; left; wait 1; left; wait 1; left; wait 1; left; wait 1; left; wait 1; left; wait 1; \
    left; wait 1; right; wait 1; right; wait 1; right; wait 1; right; wait 1; right; wait 1; right; wait 1; \
    right; wait 1; accept; until MainMenu_Sub.fng; wait 2; accept; wait 3; right; wait 1; right; wait 1; left; \
    wait 1; back; wait 3; back; until MainMenu.fng; wait 2; \
    left; wait 1; left; wait 1; left; wait 1; left; wait 1; left; wait 1; left; wait 1; left; wait 1; left; wait 1; \
    accept; until MainMenu_Sub.fng; wait 2; right; wait 1; back; until MainMenu.fng; wait 4";

/// The cues the walk must start, in order. Other cues may come between them. These are the cues the build before
/// AUDIO-030 (e24e4e79b) started on this walk, with the screen-transition cues the original requests (123 to 126,
/// observed live). Left out: the 0s for Up and Down in the career sub menu (the original requests none, AUDIO-033), Left
/// at the strip's first icon and Right on a slider at its limit (nothing moves: the original's scroll and slider flags
/// stay clear, read in code), and the second Back of the Options page (AUDIO-034).
const EXPECTED: &[i32] = &[
    4, // splash
    4, 10, // alias list: create
    4, 11, 123, // dialog, main menu enters
    2, 2, 2, 2, 2, 2, // right x3, left x3
    4, 124, 125, // career sub menu
    5, 126, 123, // back
    2, 4, 124, 5, 123, // right, Challenge Series and back
    2,   // left to Career (then the strip's end)
    2, 2, 4, 124, 125, 2, 5, 126, 123, // Quick Race
    2, 2, // left from Quick Race to Career
    2, 2, 2, 2, 2, 2, 2, 4, 124, 125, // right to Options
    4, 126, // the first category
    2, 5, 125, // a slider down, back
    5, 126, 123, // back to the main menu
    2, 2, 2, 2, 2, 2, 2, // left from Options to Career
    4, 124, 125, 2, 5, 126, 123, // career menu
];

fn started_menu_voices(trace: &Path) -> Vec<i32> {
    let text = std::fs::read_to_string(trace).expect("audio trace");
    text.lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|record| record["event"] == "menu_trigger" && record["success"] == true && record["group"] == "Sfx")
        .filter_map(|record| record["trigger"].as_i64().map(|id| id as i32))
        .collect()
}

/// The position in `expected` up to which `played` contains it in order.
fn matched_prefix(played: &[i32], expected: &[i32]) -> usize {
    let mut next = 0;
    for &id in played {
        if next < expected.len() && expected[next] == id {
            next += 1;
        }
    }
    next
}

#[test]
fn ordered_match_skips_extra_cues() {
    assert_eq!(matched_prefix(&[4, 7, 10, 2], &[4, 10, 2]), 3);
    assert_eq!(matched_prefix(&[4, 2], &[4, 10, 2]), 1);
}

#[test]
#[ignore = "needs the game (NFSMW_GAME_DIR), a display and about two minutes; runs a hidden, silent session"]
fn the_menu_walk_starts_every_front_end_cue() {
    let game = std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR");
    let exe =
        std::env::var_os("NFSMW_WALK_EXE").map_or_else(|| PathBuf::from(env!("CARGO_BIN_EXE_nfsmw")), PathBuf::from);
    let out = std::env::temp_dir().join(format!("nfsmw-menu-walk-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(out.join("profile")).unwrap();
    let audio = out.join("audio.jsonl");
    let status = Command::new(&exe)
        .current_dir(exe.parent().unwrap())
        .env("BLACKBOX_PROFILE_DIR", out.join("profile"))
        .env("NFSMW_AUDIO_OUTPUT", "silent")
        .env("NFSMW_AUDIO_TRACE", &audio)
        .env("NFSMW_UI_TRACE", out.join("ui.jsonl"))
        .args(["--game-dir".as_ref(), game.as_os_str()])
        .args([
            "play",
            "--renderer",
            "blackbox",
            "--silent-audio",
            "--offscreen",
            "--bench-seconds",
            "110",
            "--no-mods",
        ])
        .args(["--boot-flow", "--ui-script", WALK])
        .status()
        .expect("the binary runs");
    assert!(status.success(), "the walk ended with {status}");
    let played = started_menu_voices(&audio);
    let matched = matched_prefix(&played, EXPECTED);
    assert_eq!(
        matched,
        EXPECTED.len(),
        "missing cue {:?} at position {matched} of the walk; started: {played:?}; traces in {}",
        EXPECTED.get(matched),
        out.display()
    );
    let _ = std::fs::remove_dir_all(&out);
}
