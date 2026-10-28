//! Actual installed vignette alpha/draw state through open, resize and retained-screen restoration.

use super::*;
use crate::frontend::garage_ui::{GarageUi, Page};
use crate::frontend::logic::{Env, Plain};
use crate::input::{Bindings, InputDevice};
use crate::settings::{Partial, Settings, test_caps};
use crate::ui::{Catalog, SCREEN_FILES, UiAssets};
use blackbox_feng::NodeKind;
use game_install::GameDir;
use std::sync::Arc;

fn settle(screens: &mut Screens, env: &mut Env) {
    for _ in 0..120 {
        screens.update(1.0 / 60.0, 0, env);
    }
}

fn assert_variant(screens: &Screens, wide: bool) -> Arc<blackbox_feng::UiTree> {
    let shown = screens.presented(&Bindings::default(), InputDevice::Keyboard);
    let tree = shown.last().unwrap().tree.clone();
    for (name, active) in [(0xee8b_19c2, !wide), (0x0740_fef3, wide)] {
        let node = tree.nodes.iter().find(|node| node.name_hash == name).expect("authored vignette image");
        assert_eq!(node.visible && node.world_colour[3] > 0, active, "vignette {name:08x}, wide={wide}");
        let index = tree.nodes.iter().position(|node| node.name_hash == name).unwrap();
        assert_eq!(tree.draw_order.contains(&index), active, "only the selected native vignette is drawn");
        if active {
            let NodeKind::Image { texture, .. } = node.kind else { panic!("native vignette is an image") };
            assert!(screens.assets().texture(texture).is_some(), "selected native vignette has real artwork");
        }
    }
    tree
}

#[test]
#[ignore = "requires installed Showcase variants and art"]
fn batch12_showcase_aspect_selects_authored_vignette_on_open_resize_and_restore() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let assets = Arc::new(UiAssets::load(&dir).unwrap());
    let catalog = Catalog::load(&dir, &SCREEN_FILES);
    let mut screens = Screens::new(catalog, assets);
    let mut settings = Settings::from(Partial::default());
    let mut changed = Partial::default();
    let mut env = Env { settings: &mut settings, changed: &mut changed, caps: test_caps::native(), mods: None };
    let ui = GarageUi::new();
    ui.sync(&crate::garage::Model::default());

    assert!(screens.aspect > WIDE_ASPECT, "the initial screen aspect is wide before viewport setup");
    screens.open_with("Showcase.fng", ui.logic(Page::Showcase), false, &mut env);
    screens.set_aspect(4.0 / 3.0);
    settle(&mut screens, &mut env);
    assert_variant(&screens, false);

    for aspect in [4.0 / 3.0, 16.0 / 9.0] {
        screens.clear();
        screens.set_aspect(aspect);
        screens.open_with("Showcase.fng", ui.logic(Page::Showcase), false, &mut env);
        settle(&mut screens, &mut env);
        assert_variant(&screens, aspect > WIDE_ASPECT);
    }
    for wide in [false, true, false, true] {
        let before = screens.presented(&Bindings::default(), InputDevice::Keyboard).last().unwrap().tree.clone();
        screens.set_aspect(if wide { 16.0 / 9.0 } else { 4.0 / 3.0 });
        settle(&mut screens, &mut env);
        let after = assert_variant(&screens, wide);
        assert!(!Arc::ptr_eq(&before, &after), "the aspect scripts invalidate the presented tree");
    }

    for last_wide in [false, true] {
        screens.set_aspect(if last_wide { 4.0 / 3.0 } else { 16.0 / 9.0 });
        screens.set_aspect(if last_wide { 16.0 / 9.0 } else { 4.0 / 3.0 });
        settle(&mut screens, &mut env);
        assert_variant(&screens, last_wide);
        screens.clear();
        screens.set_aspect(if last_wide { 4.0 / 3.0 } else { 16.0 / 9.0 });
        screens.open_with("Showcase.fng", ui.logic(Page::Showcase), false, &mut env);
        screens.set_aspect(if last_wide { 16.0 / 9.0 } else { 4.0 / 3.0 });
        settle(&mut screens, &mut env);
        assert_variant(&screens, last_wide);
    }

    let id = screens.stack.last().unwrap().id;
    let parent = screens.runtime.find(id, 0x1e61_78b9).unwrap();
    let clock = screens.runtime.object(parent).unwrap().time;
    screens.set_aspect(21.0 / 9.0);
    screens.update(1.0 / 60.0, 0, &mut env);
    assert_eq!(screens.runtime.object(parent).unwrap().time, clock, "same aspect category does not replay tracks");

    screens.open_with("Dialog.fng", Box::new(Plain), false, &mut env);
    screens.pop(&mut env);
    settle(&mut screens, &mut env);
    assert_variant(&screens, true);
    screens.open_with("Dialog.fng", Box::new(Plain), false, &mut env);
    screens.set_aspect(4.0 / 3.0);
    screens.pop(&mut env);
    settle(&mut screens, &mut env);
    assert_variant(&screens, false);
    let normal = screens.runtime.find(id, 0x1e61_7877).unwrap();
    assert_eq!(screens.runtime.script_of(normal), Some(INIT_SCRIPT));
    let wide = screens.runtime.find(id, WIDE_GROUP).unwrap();
    assert_eq!(screens.runtime.script_of(wide), Some(HIDE), "resume reapplies the authored normal group pair");
}
