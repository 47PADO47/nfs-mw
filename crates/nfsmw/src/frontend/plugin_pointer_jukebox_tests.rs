//! Pointer routing against the installed EA Trax footer.

use super::*;
use crate::frontend::{
    flow::{Frontend, Start},
    ids::screen,
    jukebox,
    logic::{Args, Env},
    screens::Screens,
};
use crate::settings::{Partial, Settings, test_caps};
use crate::ui::{Catalog, SCREEN_FILES, UiAssets};
use game_install::GameDir;
use std::sync::Arc;

#[test]
#[ignore = "requires installed jukebox package; set NFSMW_GAME_DIR"]
fn installed_hidden_jukebox_footer_does_not_activate_via_pointer_router() {
    let dir = GameDir::open(std::path::PathBuf::from(std::env::var_os("NFSMW_GAME_DIR").unwrap())).unwrap();
    jukebox::prime(&dir);
    let assets = Arc::new(UiAssets::load(&dir).unwrap());
    let catalog = Catalog::load(&dir, &SCREEN_FILES);
    let mut screens = Screens::new(catalog, assets);
    let mut settings = Settings::from(Partial::default());
    let mut changed = Partial::default();
    let mut env = Env { settings: &mut settings, changed: &mut changed, caps: test_caps::native(), mods: None };
    screens.open(screen::EA_TRAX_JUKEBOX, Args::default(), true, &mut env);
    let mut fe = Frontend::new(screens, dir, Start::Menu, None);
    for _ in 0..90 {
        fe.screens.update(1.0 / 60.0, 0, &mut env);
    }
    for _ in 0..3 {
        fe.screens.update(1.0 / 60.0, (pad::BUTTON0 << 1) | (pad::BUTTON0 << 5), &mut env);
    }
    for _ in 0..3 {
        fe.screens.update(1.0 / 60.0, 0, &mut env);
    }
    let bindings = Bindings::default();
    let raw = fe.screens.trees().pop().unwrap();
    let old = raw.nodes.iter().find(|n| n.name_hash == 0x1F7F_2DE0).unwrap();
    let centre = old.world.transform_point3(glam::Vec3::ZERO).truncate();
    let point = centre + glam::Vec2::new(0.0, 13.5);
    assert!(
        (point.x - centre.x).abs() <= 40.0 && (point.y - centre.y).abs() <= 14.0,
        "inside the former manual footer hitbox"
    );
    assert!(fe.screens.hint_at(point, &bindings, InputDevice::Xbox).is_none(), "outside replacement hints");
    let title = |fe: &Frontend| {
        fe.screens
            .trees()
            .pop()
            .unwrap()
            .nodes
            .iter()
            .find(|n| n.name_hash == 0x2890_C8AF)
            .unwrap()
            .text
            .clone()
            .unwrap()
    };
    let before = title(&fe);
    assert!(before.starts_with("> "), "reorder is active");
    let _ = press_at(&mut fe, point, &bindings, InputDevice::Xbox, &mut env);
    for _ in 0..3 {
        fe.screens.update(1.0 / 60.0, 0, &mut env);
    }
    assert_eq!(title(&fe), before, "hidden native Re-order does not act through pointer routing");
    let presented = fe.screens.presented_trees(&bindings, InputDevice::Xbox).pop().unwrap();
    let label =
        presented.nodes.iter().find(|n| n.visible && n.text.as_deref() == Some("Grab")).expect("visible Grab hint");
    let point = label.world.transform_point3(glam::Vec3::ZERO).truncate();
    let mask = match press_at(&mut fe, point, &bindings, InputDevice::Xbox, &mut env) {
        Press::Button(mask) => mask,
        _ => panic!("visible Grab hint must route to its bound button"),
    };
    for _ in 0..3 {
        fe.screens.update(1.0 / 60.0, mask, &mut env);
    }
    for _ in 0..3 {
        fe.screens.update(1.0 / 60.0, 0, &mut env);
    }
    assert!(!title(&fe).starts_with("> "), "visible Grab still works");
}

#[test]
#[ignore = "requires installed jukebox package; set NFSMW_GAME_DIR"]
fn installed_visible_jukebox_accept_routes_once_and_returns() {
    const CHILD: &str = "NFSMW_JUKEBOX_POINTER_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let unique = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let profile = std::env::temp_dir().join(format!("nfsmw-jukebox-pointer-{}-{unique}", std::process::id()));
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["frontend::plugin::pointer_input::jukebox_tests::installed_visible_jukebox_accept_routes_once_and_returns", "--exact", "--ignored"])
            .env(CHILD, "1").env("BLACKBOX_PROFILE_DIR", &profile).status().unwrap();
        assert!(status.success(), "isolated pointer route");
        assert!(profile.join("nfsmw/config/eatrax_jukebox.toml").is_file(), "child ran and saved");
        return;
    }
    let path = crate::settings::eatrax::path().unwrap();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, "random = false\nsongs = [[0, 0], [1, 0], [2, 0], [3, 0], [4, 0]]\n").unwrap();
    let dir = GameDir::open(std::path::PathBuf::from(std::env::var_os("NFSMW_GAME_DIR").unwrap())).unwrap();
    jukebox::prime(&dir);
    let assets = Arc::new(UiAssets::load(&dir).unwrap());
    let catalog = Catalog::load(&dir, &SCREEN_FILES);
    let mut screens = Screens::new(catalog, assets);
    let mut settings = Settings::from(Partial::default());
    let mut changed = Partial::default();
    let mut env = Env { settings: &mut settings, changed: &mut changed, caps: test_caps::native(), mods: None };
    screens.open(screen::EA_TRAX_JUKEBOX, Args::default(), true, &mut env);
    let mut fe = Frontend::new(screens, dir, Start::Menu, None);
    for _ in 0..90 {
        fe.screens.update(1.0 / 60.0, 0, &mut env);
    }
    for _ in 0..4 {
        for _ in 0..3 {
            fe.screens.update(1.0 / 60.0, pad::DOWN, &mut env);
        }
        for _ in 0..3 {
            fe.screens.update(1.0 / 60.0, 0, &mut env);
        }
    }
    let before = fe.screens.trees().pop().unwrap();
    let row =
        |tree: &blackbox_feng::UiTree, hash| tree.nodes.iter().find(|n| n.name_hash == hash).unwrap().text.clone();
    let title = row(&before, 0x2890_C8B2);
    let mode = row(&before, 0xF3EB_DC51);
    let arrow = {
        let tree = fe.screens.trees().pop().unwrap();
        let node = tree.nodes.iter().find(|n| n.guid == 0xF83DE && n.visible).expect("right mode arrow");
        node.world.transform_point3(glam::Vec3::ZERO).truncate()
    };
    let bindings = Bindings::default();
    let mask = match press_at(&mut fe, arrow, &bindings, InputDevice::Xbox, &mut env) {
        Press::Accept => pad::ACCEPT,
        Press::Button(mask) => mask,
        Press::Screen(commands) => {
            assert!(commands.is_empty());
            0
        }
        Press::Nothing => 0,
        _ => panic!("mode arrow must not route to Back or Quit"),
    };
    for _ in 0..3 {
        fe.screens.update(1.0 / 60.0, mask, &mut env);
    }
    for _ in 0..3 {
        fe.screens.update(1.0 / 60.0, 0, &mut env);
    }
    assert_eq!(fe.screens.top(), Some(screen::EA_TRAX_JUKEBOX), "mode arrow does not accept the screen");
    let after = fe.screens.trees().pop().unwrap();
    assert_eq!(row(&after, 0x2890_C8B2), title, "mode click preserves song title");
    assert_ne!(row(&after, 0xF3EB_DC51), mode, "selected row mode column changes");
    let bindings = Bindings::default();
    let tree = fe.screens.presented_trees(&bindings, InputDevice::Xbox).pop().unwrap();
    let label = tree.nodes.iter().find(|n| n.visible && n.text.as_deref() == Some("Accept")).unwrap();
    let point = label.world.transform_point3(glam::Vec3::ZERO).truncate();
    assert!(matches!(press_at(&mut fe, point, &bindings, InputDevice::Xbox, &mut env), Press::Accept));
    for _ in 0..100 {
        fe.screens.update(1.0 / 60.0, pad::ACCEPT, &mut env);
    }
    for _ in 0..90 {
        fe.screens.update(1.0 / 60.0, 0, &mut env);
    }
    assert_eq!(fe.screens.top(), Some(screen::MAIN_MENU_SUB));
    let tree = fe.screens.trees().pop().unwrap();
    assert!(
        tree.nodes
            .iter()
            .any(|n| n.visible && n.world_colour[3] > 0 && n.name_hash == crate::frontend::ids::ICON_TITLE),
        "returned category title is visible"
    );
    let file: toml::Value = toml::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    assert_eq!(file["songs"][0][1].as_integer(), Some(0), "other song is unchanged");
    assert_eq!(file["songs"][4][1].as_integer(), Some(1), "Accept preserves the selected mode");
}
