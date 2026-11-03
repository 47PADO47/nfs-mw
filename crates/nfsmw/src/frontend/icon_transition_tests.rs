//! Installed icon rows must shrink during exit, including the embedded Controls icon. The menus' lists come from
//! `icon_items.rs`, so the counts and presses below are read from there rather than written out.

use blackbox_feng::{NodeKind, fe_hash_upper};
use glam::Vec2;

use super::*;
use crate::frontend::icon_items::{Icon, Id, draw_scale, icons};
use crate::frontend::icon_menu::Kind;
use crate::frontend::ids;

#[derive(Clone, Copy, Debug)]
struct Pose {
    size: Vec2,
    center: Vec2,
    alpha: u8,
}

fn row(h: &Harness, count: usize) -> Vec<Pose> {
    let tree = h.screens.trees().pop().unwrap();
    (3..3 + count)
        .map(|slot| {
            let hash = fe_hash_upper(&format!("OPTION_{slot}"));
            let n = tree.nodes.iter().find(|n| n.name_hash == hash).unwrap();
            Pose { size: n.size.abs().truncate(), center: n.local_position.truncate(), alpha: n.world_colour[3] }
        })
        .collect()
}

/// The number of icons in a menu, and how many right presses reach `id` from its first icon.
fn reach(kind: Kind, id: Id) -> (usize, usize) {
    let list = icons(kind);
    (list.len(), list.iter().position(|i| i.id == id).expect("the icon is in the menu"))
}

fn assert_exit(h: &mut Harness, count: usize, icon: Icon, button: u32) {
    h.wait(0.5);
    let full = row(h, count);
    // Discord deliberately occupies65% of a slot. Verify the selected artwork's
    // authored full size instead of assuming every selection grows past60px.
    let tree = h.screens.trees().pop().unwrap();
    let master = tree.nodes.iter().find(|n| n.name_hash == ids::OPTION_MASTER).unwrap();
    let selected = tree
        .nodes
        .iter()
        .find(|n| {
            n.visible
                && matches!(n.kind, NodeKind::Image { texture, .. } if texture == icon.texture)
                && (n.local_position.truncate() - master.local_position.truncate()).length() < 0.001
        })
        .expect("the selected icon is drawn at the cursor");
    let expected = master.size.abs().truncate() * draw_scale(&icon);
    assert!(
        (selected.size.abs().truncate() - expected).length() < 0.001,
        "selected artwork must finish entering: {:?} {:?}, expected{expected:?}",
        icon.id,
        selected.size
    );
    let mut previous = full.clone();
    let mut shrunk = false;
    // Observe the native leave script before EXIT_COMPLETE can unload the screen.
    for frame in 0..20 {
        let mask = match frame {
            0 => button,
            _ => 0,
        };
        h.run(mask, 1);
        let tree = h.screens.trees().pop().unwrap();
        let master = tree.nodes.iter().find(|n| n.name_hash == ids::OPTION_MASTER).unwrap();
        assert!(!master.visible, "the authored placeholder must stay hidden during exit frame {frame}");
        let current = row(h, count);
        for ((p, before), idle) in current.iter().zip(&previous).zip(&full) {
            assert!(p.size.x <= before.size.x + 0.001, "icon grew during exit: {before:?} -> {p:?}");
            assert!(p.alpha <= before.alpha, "icon became opaque during exit");
            assert!((p.center - idle.center).length() < 0.001, "exit must scale about the icon centre");
            if idle.size.x > 1.0 {
                shrunk |= p.size.x > 0.01 && p.size.x < idle.size.x - 0.01;
            }
        }
        if frame >= 12 {
            assert!(current.iter().all(|p| p.size.x < 0.001 && p.alpha == 0), "outgoing row stayed visible");
        }
        previous = current;
    }
    assert!(shrunk, "the row must animate through intermediate sizes");
}

#[test]
#[ignore = "requires installed menu packages; set NFSMW_GAME_DIR"]
fn all_icon_menus_shrink_on_native_accept_back_and_start() {
    let main = Kind::Main;
    let cats = Kind::Categories { pause: false, mods: false };
    let pause_cats = Kind::Categories { pause: true, mods: false };
    let sub = Args { options: true, ..Args::default() };
    let pause = Args { pause: true, ..Args::default() };
    let pause_sub = Args { pause: true, options: true, ..Args::default() };
    for (name, args, kind, id, button) in [
        (screen::MAIN_MENU, Args::default(), main, Id::QuickRace, pad::ACCEPT),
        (screen::MAIN_MENU_SUB, sub, cats, Id::Category(Category::Gameplay), pad::ACCEPT),
        (screen::MAIN_MENU_SUB, sub, cats, Id::Category(Category::Player), pad::ACCEPT),
        (screen::MAIN_MENU_SUB, sub, cats, Id::Category(Category::Audio), pad::BACK),
        (screen::MAIN_MENU_SUB, sub, cats, Id::Category(Category::Discord), pad::ACCEPT),
        (screen::PAUSE_MENU, pause, Kind::Pause, Id::Options, pad::ACCEPT),
        (screen::PAUSE_MENU, pause, Kind::Pause, Id::Resume, pad::BACK),
        (screen::PAUSE_MENU, pause, Kind::Pause, Id::Resume, pad::START),
        (screen::PAUSE_MENU, pause_sub, pause_cats, Id::Controls, pad::ACCEPT),
        (screen::PAUSE_MENU, pause_sub, pause_cats, Id::Category(Category::Player), pad::ACCEPT),
        (screen::PAUSE_MENU, pause_sub, pause_cats, Id::Category(Category::Audio), pad::BACK),
        (screen::PAUSE_MENU, pause_sub, pause_cats, Id::Category(Category::Audio), pad::START),
    ] {
        let (count, steps) = reach(kind, id);
        let mut h = Harness::open(name, args).unwrap();
        h.wait(1.0);
        for _ in 0..steps {
            h.press(pad::RIGHT);
        }
        let icon = icons(kind).into_iter().find(|icon| icon.id == id).unwrap();
        assert_exit(&mut h, count, icon, button);
    }
}

/// The index in `row` of the controller icon. The window over the icons (`icon_window.rs`) can move the strip, so the
/// icon is found by its picture rather than by its place in the list.
fn controller_at(h: &Harness, count: usize) -> usize {
    let tree = h.screens.trees().pop().unwrap();
    let slot = (3..3 + count)
        .find(|slot| {
            let hash = fe_hash_upper(&format!("OPTION_{slot}"));
            tree.nodes.iter().any(|n| {
                n.name_hash == hash && matches!(n.kind, NodeKind::Image { texture: crate::ui::input_icons::ATLAS, .. })
            })
        })
        .expect("the controller icon is on screen");
    slot - 3
}

#[test]
#[ignore = "requires installed menu packages; set NFSMW_GAME_DIR"]
fn the_controller_icon_grows_on_return_and_shares_the_row_exit_factor() {
    let (count, control) = reach(Kind::Categories { pause: true, mods: false }, Id::ControlOptions);
    let mut h = Harness::open(screen::PAUSE_MENU, Args { pause: true, options: true, ..Args::default() }).unwrap();
    assert!(row(&h, count).iter().all(|p| p.size.x == 0.0 && p.alpha == 0));
    h.wait(1.0);
    for _ in 0..control {
        h.press(pad::RIGHT);
    }
    h.wait(0.5);
    let full = row(&h, count);
    h.run(pad::ACCEPT, 1);
    h.run(0, 3);
    let current = row(&h, count);
    let at = controller_at(&h, count);
    let factor = current[at].size.x / full[at].size.x;
    assert!(factor > 0.0 && factor < 1.0);
    for (p, idle) in current.iter().zip(&full).filter(|(_, idle)| idle.size.x > 1.0) {
        assert!((p.size.x / idle.size.x - factor).abs() < 0.001);
    }
    h.wait(1.0);
    assert_eq!(h.screens.top(), Some(screen::PAUSE_OPTIONS));
    h.wait(1.0);
    h.run(pad::BACK, 1);
    for _ in 0..90 {
        h.run(0, 1);
        if h.screens.top() == Some(screen::PAUSE_MENU) {
            break;
        }
    }
    assert_eq!(h.screens.top(), Some(screen::PAUSE_MENU));
    assert!(row(&h, count).iter().all(|p| p.size.x == 0.0 && p.alpha == 0), "return must start at zero");
    h.run(0, 3);
    let at = controller_at(&h, count);
    let small = row(&h, count)[at];
    assert!(small.size.x > 0.0 && small.size.x < 60.0);
    h.wait(1.0);
    assert!(
        row(&h, count)[controller_at(&h, count)].size.x > 60.0,
        "remembered Control Options selection must finish growing"
    );
}

/// The main menu has eight icons over eleven package objects, so its row is windowed: the selected icon is drawn
/// once, on the cursor, from whichever object the window put it on.
#[test]
#[ignore = "requires installed menu packages; set NFSMW_GAME_DIR"]
fn the_main_menu_windows_its_icons_over_its_objects() {
    let (_, options) = reach(Kind::Main, Id::Options);
    let mut h = Harness::open(screen::MAIN_MENU, Args::default()).unwrap();
    h.wait(1.0);
    for _ in 0..options {
        h.press(pad::RIGHT);
    }
    h.wait(0.5);
    let tree = h.screens.trees().pop().unwrap();
    let node = |hash: u32| tree.nodes.iter().find(|n| n.name_hash == hash).unwrap();
    let master_x = node(ids::OPTION_MASTER).local_position.x;
    let objects: Vec<_> = (1..=11).map(|i| node(fe_hash_upper(&format!("OPTION_{i}")))).collect();
    let drawn: Vec<_> = objects
        .iter()
        .filter(|n| matches!(n.kind, NodeKind::Image { texture, .. } if texture == 0x3058_FE37))
        .collect();
    assert_eq!(drawn.len(), 1, "the Options icon is on exactly one object");
    assert!((drawn[0].local_position.x - master_x).abs() < 1.0, "the selected icon sits on the cursor");
}

/// Online is listed in the main menu but has no screen: it is dimmed, and it is drawn on the cursor when selected.
#[test]
#[ignore = "requires installed menu packages; set NFSMW_GAME_DIR"]
fn the_main_menu_lists_online_dimmed() {
    let (_, online) = reach(Kind::Main, Id::Online);
    let mut h = Harness::open(screen::MAIN_MENU, Args::default()).unwrap();
    h.wait(1.0);
    for _ in 0..online {
        h.press(pad::RIGHT);
    }
    h.wait(0.5);
    let tree = h.screens.trees().pop().unwrap();
    let drawn = tree
        .nodes
        .iter()
        .find(|n| matches!(n.kind, NodeKind::Image { texture: 0x2056_3D30, .. }))
        .expect("the Online icon is on a package object");
    assert!(drawn.world_colour[3] > 0 && drawn.world_colour[3] <= 150, "dimmed, but drawn");
    let master = tree.nodes.iter().find(|n| n.name_hash == ids::OPTION_MASTER).unwrap();
    assert!((drawn.local_position.x - master.local_position.x).abs() < 1.0);
}

/// The main menu's Options icon, as the number of right presses from its first icon.
fn main_options_steps() -> usize {
    icons(Kind::Main).iter().position(|i| i.id == Id::Options).expect("the main menu has Options")
}

#[test]
#[ignore = "requires installed menu packages; set NFSMW_GAME_DIR"]
fn every_icon_of_the_main_menu_can_be_selected_but_only_the_working_ones_do_anything() {
    let Some(mut h) = Harness::open(screen::MAIN_MENU, Args::default()) else { return };
    h.wait(1.0);
    h.press(pad::RIGHT);
    h.press(pad::ACCEPT);
    h.wait(1.5);
    assert!(h.commands.is_empty(), "the challenge series do nothing yet: {:?}", h.commands);
    assert_eq!(h.screens.top(), Some(screen::MAIN_MENU));
    // Online and LAN come before Options, and are greyed out too: the row is walked through them.
    for _ in 1..main_options_steps() {
        h.press(pad::RIGHT);
    }
    h.press(pad::ACCEPT);
    h.wait(1.5);
    assert_eq!(h.screens.top(), Some(screen::MAIN_MENU_SUB), "the steps right reach Options");
}

#[test]
#[ignore = "requires installed menu packages; set NFSMW_GAME_DIR"]
fn options_and_back_move_between_the_main_menu_and_the_categories() {
    let Some(mut h) = Harness::open(screen::MAIN_MENU, Args::default()) else { return };
    h.wait(1.0);
    for _ in 0..main_options_steps() {
        h.press(pad::RIGHT);
    }
    h.press(pad::ACCEPT);
    h.wait(1.5);
    assert_eq!(h.screens.top(), Some(screen::MAIN_MENU_SUB));
    h.wait(1.0);
    h.press(pad::BACK);
    h.wait(1.5);
    assert_eq!(h.screens.top(), Some(screen::MAIN_MENU), "{:?}", h.commands);
}

/// The Credits icon of the option categories opens the credits screen, and its Back returns to the categories.
#[test]
fn credits_icon_opens_the_credits_and_back_returns_to_the_categories() {
    let Some(mut h) = Harness::open(screen::MAIN_MENU_SUB, Args { options: true, ..Args::default() }) else { return };
    h.wait(1.0);
    let (_, credits) = reach(Kind::Categories { pause: false, mods: false }, Id::Credits);
    for _ in 0..credits {
        h.press(pad::RIGHT);
    }
    h.wait(0.5);
    h.press(pad::ACCEPT);
    h.wait(1.5);
    assert_eq!(h.screens.top(), Some(screen::CREDITS), "credits: {:?}", h.commands);
    h.press(pad::BACK);
    h.wait(1.5);
    assert_eq!(h.screens.top(), Some(screen::MAIN_MENU_SUB), "back from credits: {:?}", h.commands);
}

/// The Back button of the credits is a group with no mouse answer of its own. Its picture is what the player sees,
/// so a click on the visible Back label goes back to the categories.
#[test]
fn a_click_on_the_credits_back_label_returns_to_the_categories() {
    let Some(mut h) = Harness::open(screen::MAIN_MENU_SUB, Args { options: true, ..Args::default() }) else { return };
    h.wait(1.0);
    let (_, credits) = reach(Kind::Categories { pause: false, mods: false }, Id::Credits);
    for _ in 0..credits {
        h.press(pad::RIGHT);
    }
    h.wait(0.5);
    h.press(pad::ACCEPT);
    h.wait(1.5);
    assert_eq!(h.screens.top(), Some(screen::CREDITS), "credits: {:?}", h.commands);
    let tree = h.screens.trees().pop().unwrap();
    let label = tree.nodes.iter().find(|n| n.guid == 0x000F_8109 && n.visible).expect("the visible Back label");
    let at = label.world.transform_point3(glam::Vec3::ZERO).truncate() + glam::Vec2::new(-20.0, 0.0);
    let mut env = Env { settings: &mut h.settings, changed: &mut h.changed, caps: test_caps::native(), mods: None };
    let commands = h.screens.click(at, &mut env).expect("the credits take the click");
    h.commands.extend(commands);
    h.wait(1.5);
    assert_eq!(h.screens.top(), Some(screen::MAIN_MENU_SUB), "back from credits by mouse: {:?}", h.commands);
}

/// The replacement Re-order hint routes to the current binding, while the native page arrow still takes clicks.
#[test]
fn the_jukebox_footer_labels_take_mouse_clicks() {
    let (_, ea_trax) = reach(Kind::Categories { pause: false, mods: false }, Id::EaTrax);
    let dir =
        game_install::GameDir::open(std::path::PathBuf::from(std::env::var_os("NFSMW_GAME_DIR").unwrap_or_default()))
            .ok();
    if let Some(dir) = &dir {
        crate::frontend::jukebox::prime(dir);
    }
    let Some(mut h) = Harness::open(screen::MAIN_MENU_SUB, Args { options: true, ..Args::default() }) else { return };
    h.wait(1.0);
    for _ in 0..ea_trax {
        h.press(pad::RIGHT);
    }
    h.wait(0.5);
    h.press(pad::ACCEPT);
    h.wait(1.5);
    assert_eq!(h.screens.top(), Some(screen::EA_TRAX_JUKEBOX), "EA Trax: {:?}", h.commands);
    let reorder = |h: &Harness| {
        let bindings = crate::input::Bindings::default();
        let device = crate::input::InputDevice::Xbox;
        let tree = h.screens.presented_trees(&bindings, device).pop().unwrap();
        let label = tree
            .nodes
            .iter()
            .find(|n| n.visible && n.text.as_deref() == Some("Re-order"))
            .expect("visible Re-order hint");
        let point = label.world.transform_point3(glam::Vec3::ZERO).truncate();
        assert_eq!(h.screens.hint_at(point, &bindings, device), Some(crate::input::Action::MenuButton1));
    };
    let titles = |h: &Harness| -> Vec<String> {
        let tree = h.screens.trees().pop().unwrap();
        tree.nodes
            .iter()
            .filter(|n| [0x2890_C8AF, 0x2890_C8B0, 0x2890_C8B1, 0x2890_C8B2].contains(&n.name_hash))
            .filter_map(|n| n.text.clone())
            .collect()
    };
    let before = titles(&h);
    reorder(&h);
    h.press((pad::BUTTON0 << 1) | (pad::BUTTON0 << 5));
    h.wait(0.5);
    assert_ne!(titles(&h), before, "Re-order marks the highlighted song");
}

#[test]
#[ignore = "requires installed menu packages; set NFSMW_GAME_DIR"]
fn installed_playtest_eatrax_save_returns_to_visible_usable_categories() {
    const CHILD: &str = "NFSMW_JUKEBOX_RETURN_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let unique = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let profile = std::env::temp_dir().join(format!("nfsmw-jukebox-return-{}-{unique}", std::process::id()));
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["frontend::tests::icon_transition_tests::installed_playtest_eatrax_save_returns_to_visible_usable_categories", "--exact", "--ignored"])
            .env(CHILD, "1").env("BLACKBOX_PROFILE_DIR", &profile)
            .status().expect("start isolated jukebox test");
        assert!(status.success(), "isolated jukebox route failed");
        assert!(
            profile.join("nfsmw/config/eatrax_jukebox.toml").is_file(),
            "child ran and saved inside its isolated profile"
        );
        return;
    }
    let dir =
        game_install::GameDir::open(std::path::PathBuf::from(std::env::var_os("NFSMW_GAME_DIR").unwrap())).unwrap();
    crate::frontend::jukebox::prime(&dir);
    let (_, ea_trax) = reach(Kind::Categories { pause: false, mods: false }, Id::EaTrax);
    let mut h = Harness::open(screen::MAIN_MENU_SUB, Args { options: true, ..Args::default() }).unwrap();
    h.wait(1.0);
    for _ in 0..ea_trax {
        h.press(pad::RIGHT);
    }
    h.wait(0.5);
    h.press(pad::ACCEPT);
    h.wait(1.5);
    assert_eq!(h.screens.top(), Some(screen::EA_TRAX_JUKEBOX));
    let order = |h: &Harness| {
        let tree = h.screens.trees().pop().unwrap();
        let n = tree.nodes.iter().find(|n| n.name_hash == 0xCA74_A2FA && n.visible).expect("visible play-order label");
        n.text.clone().expect("play-order text")
    };
    let before = order(&h);
    h.press(pad::RTRIGGER);
    h.wait(0.5);
    let saved = order(&h);
    assert_ne!(before, saved, "play-order change must be visible before saving");
    let check_parent = |h: &Harness| {
        assert_eq!(h.screens.top(), Some(screen::MAIN_MENU_SUB));
        let tree = h.screens.trees().pop().unwrap();
        assert!(
            tree.nodes.iter().any(|n| n.visible
                && n.world_colour[3] > 0
                && tree.draw_order.contains(&n.index)
                && matches!(n.kind, NodeKind::Image { texture: 0xC52C_CBF6, .. })),
            "returned EA Trax icon is drawn"
        );
        assert!(
            tree.nodes.iter().any(|n| n.name_hash == ids::ICON_TITLE
                && n.visible
                && n.world_colour[3] > 0
                && n.text.as_ref().is_some_and(|s| !s.is_empty())),
            "returned title is visible"
        );
        for device in [crate::input::InputDevice::Xbox, crate::input::InputDevice::Keyboard] {
            let presented = h.screens.presented_trees(&crate::input::Bindings::default(), device);
            let shown = presented.last().unwrap();
            for label in ["Back", "Accept"] {
                assert!(
                    shown.nodes.iter().any(|n| n.visible && n.world_colour[3] > 0 && n.text.as_deref() == Some(label)),
                    "visible {device:?} {label} footer"
                );
            }
        }
    };
    for _ in 0..2 {
        h.press(pad::ACCEPT);
        h.wait(1.5);
        check_parent(&h);
        h.press(pad::ACCEPT);
        h.wait(1.5);
        assert_eq!(h.screens.top(), Some(screen::EA_TRAX_JUKEBOX), "return selection accepts EA Trax again");
        assert_eq!(order(&h), saved, "play order survives save and reopen");
        h.press(pad::BACK);
        h.wait(1.5);
        check_parent(&h);
        h.press(pad::ACCEPT);
        h.wait(1.5);
        assert_eq!(h.screens.top(), Some(screen::EA_TRAX_JUKEBOX), "Back return is usable");
        assert_eq!(order(&h), saved, "play order survives Back and reopen");
    }
    let bindings = crate::input::Bindings::default();
    let device = crate::input::InputDevice::Xbox;
    let tree = h.screens.presented_trees(&bindings, device).pop().unwrap();
    let label =
        tree.nodes.iter().find(|n| n.visible && n.text.as_deref() == Some("Accept")).expect("visible Accept hint");
    let point = label.world.transform_point3(glam::Vec3::ZERO).truncate();
    assert_eq!(h.screens.hint_at(point, &bindings, device), Some(crate::input::Action::MenuAccept));
    let first_mode = || {
        let file = std::fs::read_to_string(crate::settings::eatrax::path().unwrap()).unwrap();
        let data: toml::Value = toml::from_str(&file).unwrap();
        data["songs"].as_array().unwrap()[0].as_array().unwrap()[1].as_integer().unwrap()
    };
    let before_mode = first_mode();
    h.run(pad::ACCEPT, 1);
    assert!(h.click(point), "additional footer clicks are consumed while leaving");
    h.run(pad::ACCEPT | pad::RTRIGGER, 100);
    h.run(0, 3);
    assert_eq!(first_mode(), before_mode, "held Accept does not change mode");
    h.wait(1.5);
    check_parent(&h);
    h.press(pad::ACCEPT);
    h.wait(1.5);
    assert_eq!(h.screens.top(), Some(screen::EA_TRAX_JUKEBOX));
    assert_eq!(order(&h), saved, "input during exit cannot change the saved order");
}
