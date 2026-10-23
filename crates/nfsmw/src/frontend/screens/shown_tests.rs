//! The decorated trees across frames, against the packages of the install (skipped without `NFSMW_GAME_DIR`).

use std::sync::Arc;

use bevy_input::keyboard::KeyCode;
use blackbox_feng::runtime::pad;
use game_install::GameDir;

use super::Screens;
use crate::frontend::ids::screen;
use crate::frontend::logic::{Args, Env};
use crate::input::{Action, Bindings, InputDevice};
use crate::settings::{Partial, Settings, test_caps};
use crate::ui::{Catalog, SCREEN_FILES, UiAssets};

const FRAME: f32 = 1.0 / 60.0;

struct Rig {
    screens: Screens,
    settings: Settings,
    changed: Partial,
}

impl Rig {
    fn open(name: &str) -> Option<Self> {
        let root = std::env::var_os("NFSMW_GAME_DIR")?;
        let dir = GameDir::open(std::path::PathBuf::from(root)).unwrap();
        let assets = Arc::new(UiAssets::load(&dir).unwrap());
        let catalog = Catalog::load(&dir, &SCREEN_FILES);
        let mut rig = Self {
            screens: Screens::new(catalog, assets),
            settings: Settings::from(Partial::default()),
            changed: Partial::default(),
        };
        let mut env =
            Env { settings: &mut rig.settings, changed: &mut rig.changed, caps: test_caps::native(), mods: None };
        let _ = rig.screens.open(name, Args::default(), true, &mut env);
        Some(rig)
    }

    fn frames(&mut self, mask: u32, count: usize) {
        for _ in 0..count {
            let mut env =
                Env { settings: &mut self.settings, changed: &mut self.changed, caps: test_caps::native(), mods: None };
            let _ = self.screens.update(FRAME, mask, &mut env);
        }
    }
}

#[test]
fn an_unchanged_screen_hands_on_the_same_tree() {
    let Some(mut rig) = Rig::open(screen::MAIN_MENU) else { return };
    rig.frames(0, 120);
    let (bindings, device) = (Bindings::default(), InputDevice::Keyboard);
    let first = rig.screens.presented(&bindings, device);
    let builds = rig.screens.tree_builds();
    assert!(builds > 0, "the first frame builds");
    let second = rig.screens.presented(&bindings, device);
    assert_eq!(rig.screens.tree_builds(), builds, "a second frame of the same package builds nothing");
    assert!(Arc::ptr_eq(&first[0].tree, &second[0].tree), "the tree is handed on");
    assert_eq!(first[0].serial, second[0].serial);
}

#[test]
fn a_changed_package_builds_the_tree_again() {
    let Some(mut rig) = Rig::open(screen::MAIN_MENU) else { return };
    rig.frames(0, 120);
    let (bindings, device) = (Bindings::default(), InputDevice::Keyboard);
    let before = rig.screens.presented(&bindings, device);
    let builds = rig.screens.tree_builds();
    rig.frames(pad::RIGHT, 3);
    rig.frames(0, 3);
    let after = rig.screens.presented(&bindings, device);
    assert!(rig.screens.tree_builds() > builds, "moving the focus changed the package");
    assert_ne!(before[0].serial, after[0].serial);
}

#[test]
fn rebound_keys_and_another_device_build_the_tree_again() {
    let Some(mut rig) = Rig::open(screen::MAIN_MENU) else { return };
    rig.frames(0, 120);
    let (bindings, device) = (Bindings::default(), InputDevice::Keyboard);
    rig.screens.presented(&bindings, device);
    let builds = rig.screens.tree_builds();
    let mut rebound = bindings.clone();
    rebound.rebind_key(Action::MenuQuit, KeyCode::KeyZ);
    rig.screens.presented(&rebound, device);
    assert_eq!(rig.screens.tree_builds(), builds + 1, "the hints name the new key");
    rig.screens.presented(&rebound, InputDevice::Xbox);
    assert_eq!(rig.screens.tree_builds(), builds + 2, "the hints show the pad");
}
