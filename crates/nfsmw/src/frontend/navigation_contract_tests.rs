//! Repeated real-screen routes, including accepted exits and overflow lists.
use super::*;
use crate::input::{Bindings, InputDevice};

#[test]
#[ignore = "requires installed menu packages and music metadata"]
fn pass_b_jukebox_save_and_back_return_to_visible_audio_category_twice() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    crate::frontend::jukebox::prime(&dir);
    for exit in [pad::ACCEPT, pad::BACK] {
        let mut h = Harness::open(screen::EA_TRAX_JUKEBOX, Args::default()).unwrap();
        for visit in 0..2 {
            h.wait(1.0);
            h.commands.clear();
            h.press(pad::RIGHT);
            assert!(h.commands.iter().any(|c| matches!(c, Command::JukeboxChanged(_))), "mode edit must be saved");
            h.run(exit, 120);
            h.run(0, 90);
            assert_eq!(h.screens.top(), Some(screen::MAIN_MENU_SUB));
            assert!(h.commands.contains(&Command::JukeboxPreview(None)));
            for device in [InputDevice::Xbox, InputDevice::Keyboard] {
                let shown = h.screens.presented_trees(&Bindings::default(), device);
                let tree = shown.last().unwrap();
                assert!(
                    tree.nodes.iter().any(|n| n.visible && n.world_colour[3] > 0 && n.text.as_deref() == Some("Audio")),
                    "return category visible"
                );
                assert!(
                    tree.nodes.iter().any(|n| n.visible && n.world_colour[3] > 0 && n.text.as_deref() == Some("Back")),
                    "return footer visible"
                );
            }
            if visit == 0 {
                let mut env =
                    Env { settings: &mut h.settings, changed: &mut h.changed, caps: test_caps::native(), mods: None };
                h.screens.open(screen::EA_TRAX_JUKEBOX, Args::default(), true, &mut env);
            }
        }
    }
}

#[test]
#[ignore = "requires installed option widgets"]
fn pass_b_overflow_scrollbar_tracks_real_selection_then_resets_after_wrap_and_reopen() {
    for category in [Category::Video, Category::Hud] {
        for _ in 0..2 {
            let mut h = Harness::open(screen::OPTIONS, Args { category, ..Default::default() }).unwrap();
            h.wait(1.0);
            let rows = crate::frontend::options::rows(category, &test_caps::native()).len();
            assert!(rows > 9);
            let position = |h: &Harness| {
                let tree = h.screens.trees().pop().unwrap();
                let root = tree.nodes.iter().find(|n| n.name_hash == 0xa808_e057).unwrap();
                assert!(root.visible && root.world_colour[3] > 0);
                tree.nodes
                    .iter()
                    .find(|n| n.parent == Some(root.index) && n.name_hash == 0x3fbb_7a4e)
                    .unwrap()
                    .position
                    .y
            };
            let first = position(&h);
            let mut previous = first;
            for _ in 1..rows {
                h.press(pad::DOWN);
                let next = position(&h);
                assert!(next + 0.001 >= previous, "scrollbar must follow the visible window");
                previous = next;
            }
            assert!(previous > first, "overflow list must move its scrollbar");
            h.press(pad::DOWN);
            assert!((position(&h) - first).abs() < 0.001, "wrap restores first window");
        }
    }
}

#[test]
#[ignore = "requires installed settings artwork and Vulkan GPU"]
fn pass_b_gpu_settings_footer_survives_child_return_and_device_switch() {
    for (name, pause) in [(screen::OPTIONS, false), (screen::PAUSE_OPTIONS, true)] {
        let mut h = Harness::open(name, Args { pause, category: Category::Audio, ..Default::default() }).unwrap();
        for visit in 0..2 {
            h.wait(1.0);
            for (label, device) in [("pad", InputDevice::Xbox), ("keyboard", InputDevice::Keyboard)] {
                let tree = h.screens.presented_trees(&Bindings::default(), device).pop().unwrap();
                crate::frontend::tests::rendered_playtest::render(
                    &tree,
                    h.screens.assets(),
                    (1920, 1080),
                    &format!("settings-{pause}-{visit}-{label}"),
                );
            }
            h.press(pad::BACK);
            h.wait(1.0);
            h.press(pad::ACCEPT);
            h.wait(1.0);
            assert_eq!(h.screens.top(), Some(name));
        }
    }
}
