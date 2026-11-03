//! Installed jukebox navigation and cursor alignment against the native package.

use super::*;
use crate::settings::{Partial, Settings, test_caps};
use crate::ui::{Catalog, SCREEN_FILES};
use blackbox_feng::{
    Runtime,
    runtime::{Outgoing, pad},
};
use std::collections::HashMap;

fn frames(logic: &mut JukeboxLogic, cx: &mut Cx, mask: u32, count: usize) {
    for _ in 0..count {
        cx.rt.set_pad_mask(mask);
        cx.rt.update(1.0 / 60.0);
        for event in cx.rt.take_outgoing() {
            if let Outgoing::Game { message, package, .. } = event
                && package == cx.package
            {
                logic.message(cx, message);
            }
        }
        logic.tick(cx, 1.0 / 60.0);
    }
}

fn press(logic: &mut JukeboxLogic, cx: &mut Cx, mask: u32) {
    frames(logic, cx, mask, 3);
    frames(logic, cx, 0, 3);
}

#[track_caller]
fn check(logic: &JukeboxLogic, cx: &Cx, selected: usize) {
    assert_eq!(logic.state.selected, selected, "logical selection");
    let first = logic.state.page_start();
    let slot = selected - first;
    let tree = cx.rt.tree(cx.package);
    let text = |hash| tree.nodes.iter().find(|n| n.name_hash == hash).unwrap().text.clone().unwrap();
    for row in 0..ROWS {
        let index = first + row;
        if let Some(entry) = logic.state.jukebox.entries().get(index) {
            assert_eq!(text(NUMBER[row]), number(index), "row number");
            assert_eq!(text(TITLE[row]), logic.songs[entry.song].title, "clean title");
            assert_eq!(text(MODE[row]), mode_text(entry.mode), "mode column");
        } else {
            assert_eq!(text(TITLE[row]), "", "unused final-page slot is blank");
        }
    }
    let root = tree.nodes.iter().find(|n| n.name_hash == 0xA808_E057).unwrap().index;
    let node = |hash| tree.nodes.iter().find(|n| n.parent == Some(root) && n.name_hash == hash).unwrap();
    let track = node(0x688A_1551);
    let handle = node(0x3FBB_7A4E);
    let fraction = first as f32 / (logic.songs.len() - ROWS) as f32;
    let range = track.size.y.abs() - handle.size.y.abs();
    assert!(
        (handle.position.y - (track.position.y - range * 0.5 + range * fraction)).abs() < 0.01,
        "scrollbar tracks the visible window"
    );
    let cursor = cx.object(0x0674_5352).expect("cursor group");
    let state = cx.rt.object(cursor).unwrap();
    let current = state.current.map(|i| state.scripts[i].id);
    assert_eq!(current, Some(0x001B_20C2 + slot as u32), "cursor slot follows selected song");
}

#[test]
#[ignore = "requires installed menu and music packages; set NFSMW_GAME_DIR"]
fn installed_jukebox_selection_tracks_native_cursor_across_pages() {
    let dir = GameDir::open(std::path::PathBuf::from(std::env::var_os("NFSMW_GAME_DIR").unwrap())).unwrap();
    let songs: &'static [Song] = Box::leak(read_songs(&dir).expect("installed song list").into_boxed_slice());
    assert!(songs.len() > 8, "test requires multiple full pages");
    let assets = UiAssets::load(&dir).unwrap();
    let catalog = Catalog::load(&dir, &SCREEN_FILES);
    let mut rt = Runtime::with_tick_rate(1200.0);
    let package = rt.load(catalog.find(screen::EA_TRAX_JUKEBOX).unwrap().clone());
    let mut settings = Settings::from(Partial::default());
    let (mut changed, mut commands, mut memory) = (Partial::default(), Vec::new(), HashMap::new());
    let mut cx = Cx {
        rt: &mut rt,
        package,
        assets: &assets,
        settings: &mut settings,
        changed: &mut changed,
        caps: test_caps::native(),
        commands: &mut commands,
        memory: &mut memory,
        key_texts: &[],
        world_map: None,
        mods: None,
        name: screen::EA_TRAX_JUKEBOX,
    };
    let defaults = Jukebox::defaults_for(songs);
    let mut logic = JukeboxLogic { songs, state: State::new(defaults), vertical: false, top: 0, leaving: false };
    logic.start(&mut cx);
    frames(&mut logic, &mut cx, 0, 90);
    check(&logic, &cx, 0);
    frames(&mut logic, &mut cx, pad::DOWN, 60);
    frames(&mut logic, &mut cx, 0, 3);
    check(&logic, &cx, 9);
    for _ in 0..9 {
        press(&mut logic, &mut cx, pad::UP);
    }
    check(&logic, &cx, 0);
    for index in 1..songs.len() {
        press(&mut logic, &mut cx, pad::DOWN);
        check(&logic, &cx, index);
    }
    press(&mut logic, &mut cx, pad::DOWN);
    check(&logic, &cx, songs.len() - 1);
    logic.scroll(&mut cx, 3);
    frames(&mut logic, &mut cx, 0, 4);
    check(&logic, &cx, songs.len() - 4);
    let selected = logic.state.selected;
    let chosen_song = logic.state.jukebox.entries()[selected].song;
    let before = logic.state.clone();
    cx.commands.clear();
    press(&mut logic, &mut cx, pad::BUTTON0 << 4);
    assert!(
        cx.commands.contains(&Command::JukeboxPreview(Some(chosen_song))),
        "X requests the visible track's preview"
    );
    assert_eq!(logic.state, before, "preview leaves selection, modes and song order unchanged");
}

#[test]
#[ignore = "requires installed jukebox package; set NFSMW_GAME_DIR"]
fn installed_jukebox_replaces_native_footer_for_controller_and_keyboard() {
    use crate::frontend::logic::Footer;
    use crate::frontend::prompts;
    use crate::input::{Bindings, InputDevice};
    let dir = GameDir::open(std::path::PathBuf::from(std::env::var_os("NFSMW_GAME_DIR").unwrap())).unwrap();
    let assets = UiAssets::load(&dir).unwrap();
    let catalog = Catalog::load(&dir, &SCREEN_FILES);
    let mut rt = Runtime::with_tick_rate(1200.0);
    let package = rt.load(catalog.find(screen::EA_TRAX_JUKEBOX).unwrap().clone());
    for _ in 0..90 {
        rt.update(1.0 / 60.0);
    }
    for device in [InputDevice::Xbox, InputDevice::Keyboard] {
        let mut tree = rt.tree(package);
        let first = tree.nodes.len();
        let hints = prompts::decorate(
            &mut tree,
            screen::EA_TRAX_JUKEBOX,
            Footer::Jukebox { reorder: false, has_songs: true },
            &Bindings::default(),
            device,
            &assets,
        );
        assert!(!hints.is_empty(), "{device:?}: actionable replacement footer");
        assert!(tree.nodes[first..].iter().any(|n| n.visible && n.text.is_some()), "visible hint labels");
        for hash in [0x01bc_fc29, 0x73dc_b662, 0x7f02_b4cd, 0xff88_751e, 0x3fd9_6ce9] {
            let node = tree.nodes.iter().find(|n| n.name_hash == hash).expect("authored legacy hint group");
            assert!(!node.visible, "legacy group {hash:08x} is hidden");
            assert!(
                tree.nodes.iter().filter(|n| n.parent == Some(node.index)).all(|n| !n.visible),
                "legacy hint descendants"
            );
        }

        if device == InputDevice::Xbox {
            assert!(
                tree.nodes[first..].iter().any(|n| n.visible
                    && matches!(n.kind,
                blackbox_feng::NodeKind::Image { texture, .. } if texture == crate::ui::input_icons::ATLAS)),
                "visible controller icons"
            );
        } else {
            assert!(
                tree.nodes[first..].iter().any(|n| n.visible && n.text.as_deref() == Some("Esc")),
                "keyboard Back binding"
            );
        }
    }
}

#[test]
#[ignore = "requires installed jukebox and music packages; set NFSMW_GAME_DIR"]
fn installed_jukebox_hold_matches_at_high_frame_rates_through_screens() {
    use crate::frontend::{logic::Env, screens::Screens};
    use std::sync::Arc;
    let dir = GameDir::open(std::path::PathBuf::from(std::env::var_os("NFSMW_GAME_DIR").unwrap())).unwrap();
    let songs: &'static [Song] = Box::leak(read_songs(&dir).unwrap().into_boxed_slice());
    assert!(songs.len() > 9);
    let assets = Arc::new(UiAssets::load(&dir).unwrap());
    for hz in [60, 2163, 2400] {
        let catalog = Catalog::load(&dir, &SCREEN_FILES);
        let mut screens = Screens::new(catalog, assets.clone());
        let mut settings = Settings::from(Partial::default());
        let mut changed = Partial::default();
        let mut env = Env { settings: &mut settings, changed: &mut changed, caps: test_caps::native(), mods: None };
        let logic = JukeboxLogic {
            songs,
            state: State::new(Jukebox::defaults_for(songs)),
            vertical: false,
            top: 0,
            leaving: false,
        };
        screens.open_with(screen::EA_TRAX_JUKEBOX, Box::new(logic), true, &mut env);
        for _ in 0..90 {
            screens.update(1.0 / 60.0, 0, &mut env);
        }
        for _ in 0..hz {
            screens.update(1.0 / hz as f32, pad::DOWN, &mut env);
        }
        for _ in 0..4 {
            screens.update(1.0 / 60.0, 0, &mut env);
        }
        let tree = screens.trees().pop().unwrap();
        let first = tree.nodes.iter().find(|n| n.name_hash == NUMBER[0]).unwrap();
        assert_eq!(first.text.as_deref(), Some("07"), "{hz} Hz: 9 moves scroll to track 10");
        let cursor = tree.nodes.iter().find(|n| n.name_hash == 0x0674_5352).unwrap();
        let title = tree.nodes.iter().find(|n| n.name_hash == TITLE[3]).unwrap();
        assert!((cursor.position.y - title.position.y - 25.0).abs() < 0.01, "{hz} Hz: cursor selects track 10");
    }
}

#[test]
fn jukebox_hints_name_only_the_actions_the_remake_supports() {
    use crate::frontend::{
        logic::Footer,
        prompts::{HintLabel, hint_set},
    };
    use crate::input::{Action, Bindings, InputDevice};
    for device in [InputDevice::Xbox, InputDevice::Keyboard] {
        let shown = hint_set(
            screen::EA_TRAX_JUKEBOX,
            Footer::Jukebox { reorder: false, has_songs: true },
            &Bindings::default(),
            device,
        );
        for (label, action) in [
            (HintLabel::Accept, Action::MenuAccept),
            (HintLabel::Back, Action::MenuBack),
            (HintLabel::Grab, Action::JukeboxGrab),
            (HintLabel::Preview, Action::MenuButton2),
        ] {
            assert!(shown.iter().any(|(_, l, a)| *l == label && *a == Some(action)));
        }
        assert!(shown.iter().any(|(_, label, _)| *label == HintLabel::ChangeMode));
        assert!(shown.iter().any(|(_, label, _)| *label == HintLabel::Order));
        let reorder = hint_set(
            screen::EA_TRAX_JUKEBOX,
            Footer::Jukebox { reorder: true, has_songs: true },
            &Bindings::default(),
            device,
        );
        assert!(reorder.iter().any(|(_, label, _)| *label == HintLabel::Move));
        assert!(!reorder.iter().any(|(_, label, _)| *label == HintLabel::ChangeMode));
    }
}
