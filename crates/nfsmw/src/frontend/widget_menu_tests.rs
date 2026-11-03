//! Tests of the option screens' rows (the logic is in `widget_menu.rs`).
use super::*;
use blackbox_feng::Runtime;
use game_install::GameDir;
use std::collections::HashMap;

use crate::settings::{Partial, Settings};
use crate::ui::{Catalog, SCREEN_FILES, UiAssets};

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn installed_scrollbar_pointer_follows_inherited_visibility_and_transforms() {
    let dir = GameDir::open(std::path::PathBuf::from(std::env::var_os("NFSMW_GAME_DIR").unwrap())).unwrap();
    let catalog = Catalog::load(&dir, &SCREEN_FILES);
    let assets = UiAssets::load(&dir).unwrap();
    let mut rt = Runtime::new();
    let package = rt.load(catalog.find(screen::OPTIONS).unwrap().clone());
    let mut settings = Settings::from(Partial::default());
    settings.ui_style = crate::settings::UiStyle::Original;
    let before = settings;
    let (mut changed, mut commands, mut memory) = (Partial::default(), Vec::new(), HashMap::new());
    let mut cx = Cx {
        rt: &mut rt,
        package,
        assets: &assets,
        settings: &mut settings,
        changed: &mut changed,
        caps: crate::settings::test_caps::native(),
        commands: &mut commands,
        memory: &mut memory,
        key_texts: &[],
        world_map: None,
        mods: None,
        name: screen::OPTIONS,
    };
    let mut menu = WidgetMenu::new(Args { category: super::super::logic::Category::Video, ..Default::default() });
    menu.start(&mut cx);
    for _ in 0..120 {
        cx.rt.update(1.0 / 60.0);
        menu.tick(&mut cx, 1.0 / 60.0);
    }
    let tree = cx.rt.tree(package);
    let root = tree.nodes.iter().find(|n| n.name_hash == 0xA808_E057).unwrap();
    let up = tree.nodes.iter().find(|n| n.parent == Some(root.index) && n.name_hash == 0x4449_69FD).unwrap();
    let image = tree
        .nodes
        .iter()
        .find(|n| n.parent == Some(up.index) && tree.draw_order.contains(&n.index) && n.world_colour[3] > 0)
        .unwrap();
    let point = image.world.transform_point3(Vec3::ZERO).truncate();
    let root_object = ObjectRef { package, index: root.index };
    let image_object = ObjectRef { package, index: image.index };
    assert_eq!(scrollbar_arrow_at(&cx, point), Some(false));
    menu.vertical = true;
    assert!(!menu.click(&mut cx, point), "alternate UI cannot activate native scrollbar arrows");
    assert_eq!(menu.selected, 0);
    menu.vertical = false;
    assert!(menu.click(&mut cx, point));
    assert_eq!(menu.selected, menu.rows.len() - 1, "up wraps using selection navigation");
    assert_eq!(*cx.settings, before);

    cx.rt.set_hidden(root_object, true);
    assert_eq!(scrollbar_arrow_at(&cx, point), None, "hidden parent has no hit");
    cx.rt.set_hidden(root_object, false);
    cx.rt.set_alpha(root_object, 0);
    assert_eq!(scrollbar_arrow_at(&cx, point), None, "transparent parent has no hit");
    cx.rt.set_alpha(root_object, root.world_colour[3]);
    let position = cx.rt.position(root_object).unwrap();
    cx.rt.set_position(root_object, Vec3::new(position.x, position.y, -1000.0));
    assert_eq!(scrollbar_arrow_at(&cx, point), None, "behind-plane images have no hit");
    cx.rt.set_position(root_object, position + Vec3::new(80.0, 0.0, 0.0));
    let moved = cx.rt.object_quad(image_object).unwrap().transform_point3(Vec3::ZERO).truncate();
    assert_eq!(scrollbar_arrow_at(&cx, point), None, "old image location has no hit");
    assert_eq!(scrollbar_arrow_at(&cx, moved), Some(false), "hit follows transformed image");
    menu.leaving = true;
    let selected = menu.selected;
    assert!(!menu.click(&mut cx, moved));
    assert_eq!(menu.selected, selected, "leaving cannot navigate");
    assert_eq!(*cx.settings, before);
    assert_eq!(*cx.changed, Partial::default());
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn vehicle_effects_retail_title_animation_matches_the_unmodified_asset() {
    let dir = GameDir::open(std::path::PathBuf::from(std::env::var_os("NFSMW_GAME_DIR").unwrap())).unwrap();
    let catalog = Catalog::load(&dir, &SCREEN_FILES);
    let assets = UiAssets::load(&dir).unwrap();
    for (name, pause) in [(screen::OPTIONS, false), (screen::PAUSE_OPTIONS, true)] {
        let package = catalog.find(name).unwrap().clone();
        let (mut baseline, mut bound) = (Runtime::new(), Runtime::new());
        let baseline_id = baseline.load(package.clone());
        let bound_id = bound.load(package);
        // Finish the enter scripts before explicitly starting the retail title's highlight.
        // The pause package can clear its initial focus during those scripts.
        for _ in 0..120 {
            baseline.update(1.0 / 60.0);
            bound.update(1.0 / 60.0);
        }
        let hash = fe_hash_upper("OPTION_NAME_1");
        let original = baseline.find(baseline_id, hash).unwrap();
        let tested = bound.find(bound_id, hash).unwrap();
        baseline.run_script(original, ids::SCRIPT_HIGHLIGHT);
        bound.run_script(tested, ids::SCRIPT_HIGHLIGHT);
        let mut menu =
            WidgetMenu::new(Args { pause, category: super::super::logic::Category::Video, ..Args::default() });
        let (mut settings, mut changed) = (Settings::from(Partial::default()), Partial::default());
        let (mut commands, mut memory) = (Vec::new(), HashMap::new());
        let mut cx = Cx {
            rt: &mut bound,
            package: bound_id,
            assets: &assets,
            settings: &mut settings,
            changed: &mut changed,
            caps: crate::settings::test_caps::native(),
            commands: &mut commands,
            memory: &mut memory,
            key_texts: &[],
            world_map: None,
            mods: None,
            name,
        };
        menu.find_rows(&mut cx);
        let (mut min_alpha, mut max_alpha) = (255, 0);
        for frame in 0..160 {
            baseline.update(1.0 / 60.0);
            cx.rt.update(1.0 / 60.0);
            menu.keep_selected_title_readable(&mut cx);
            let expected = baseline.object(original).unwrap().data.colour_rgba();
            let actual = cx.rt.object(tested).unwrap().data.colour_rgba();
            min_alpha = min_alpha.min(expected[3]);
            max_alpha = max_alpha.max(expected[3]);
            assert_eq!(actual, expected, "{name}, frame {frame}: retail RGBA must match the untouched asset");
        }
        assert!(
            min_alpha < max_alpha,
            "{name}: explicitly highlighted baseline must exercise animation (min {min_alpha}, max {max_alpha})"
        );
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn installed_advanced_graphics_action_opens_the_overlay_without_changing_settings() {
    let dir = GameDir::open(std::path::PathBuf::from(std::env::var_os("NFSMW_GAME_DIR").unwrap())).unwrap();
    let catalog = Catalog::load(&dir, &SCREEN_FILES);
    let assets = UiAssets::load(&dir).unwrap();
    for (name, pause) in [(screen::OPTIONS, false), (screen::PAUSE_OPTIONS, true)] {
        let mut rt = Runtime::new();
        let package = rt.load(catalog.find(name).unwrap().clone());
        let mut settings = Settings::from(Partial::default());
        let before = settings;
        let (mut changed, mut commands, mut memory) = (Partial::default(), Vec::new(), HashMap::new());
        let mut cx = Cx {
            rt: &mut rt,
            package,
            assets: &assets,
            settings: &mut settings,
            changed: &mut changed,
            caps: crate::settings::test_caps::native(),
            commands: &mut commands,
            memory: &mut memory,
            key_texts: &[],
            world_map: None,
            mods: None,
            name,
        };
        let mut menu =
            WidgetMenu::new(Args { pause, category: super::super::logic::Category::Video, ..Default::default() });
        menu.start(&mut cx);
        menu.selected = menu.rows.iter().position(|row| row.setting == options::Setting::AdvancedGraphics).unwrap();
        for message in [PAD_RIGHT, PAD_LEFT, BUTTON_PRESSED, PAD_ACCEPT] {
            cx.commands.clear();
            menu.message(&mut cx, message);
            assert_eq!(*cx.commands, vec![Command::OpenGraphics]);
            assert!(!menu.leaving, "opening graphics must retain the parent menu");
            assert_eq!(*cx.settings, before);
        }
    }
}

#[test]
#[ignore = "needs installed options packages; set NFSMW_GAME_DIR"]
fn installed_volume_slider_repeats_its_texture_at_zero_half_and_full() {
    use blackbox_feng::NodeKind;
    let dir = GameDir::open(std::path::PathBuf::from(std::env::var_os("NFSMW_GAME_DIR").unwrap())).unwrap();
    let catalog = Catalog::load(&dir, &SCREEN_FILES);
    let assets = UiAssets::load(&dir).unwrap();
    for (name, pause) in [(screen::OPTIONS, false), (screen::PAUSE_OPTIONS, true)] {
        for value in [0, 50, 100] {
            let mut rt = Runtime::new();
            let package = rt.load(catalog.find(name).unwrap().clone());
            let mut settings = Settings::from(Partial::default());
            settings.master_volume = crate::settings::Percent(value);
            settings.sfx_volume = crate::settings::Percent(value);
            let (mut changed, mut commands, mut memory) = (Partial::default(), Vec::new(), HashMap::new());
            let mut cx = Cx {
                rt: &mut rt,
                package,
                assets: &assets,
                settings: &mut settings,
                changed: &mut changed,
                caps: crate::settings::test_caps::native(),
                commands: &mut commands,
                memory: &mut memory,
                key_texts: &[],
                world_map: None,
                mods: None,
                name,
            };
            let mut menu =
                WidgetMenu::new(Args { pause, category: super::super::logic::Category::Audio, ..Default::default() });
            menu.start(&mut cx);
            for _ in 0..90 {
                cx.rt.update(1.0 / 60.0);
                menu.tick(&mut cx, 1.0 / 60.0);
            }
            for selected in [0, 1] {
                if selected == 1 {
                    menu.message(&mut cx, PAD_DOWN);
                    cx.rt.update(1.0 / 60.0);
                    menu.tick(&mut cx, 1.0 / 60.0);
                }
                assert_eq!(menu.selected, selected);
                let tree = cx.rt.tree(package);
                for slot in 1..=2 {
                    let base = tree
                        .nodes
                        .iter()
                        .find(|n| n.name_hash == fe_hash_upper(&format!("BASE_SLIDER_{slot}")))
                        .unwrap();
                    let fill = tree
                        .nodes
                        .iter()
                        .find(|n| n.name_hash == fe_hash_upper(&format!("FILLBAR_SLIDER_{slot}")))
                        .unwrap();
                    let NodeKind::Image { texture, uv, .. } = fill.kind else { panic!("fill must be an image") };
                    let texture_width = assets.texture_size(texture).expect("installed fill texture").0 as f32;
                    let expected = base.size.x.abs() * value as f32 / 100.0;
                    assert!((fill.size.x - expected).abs() < 0.001, "{name} row {slot}, {value}% width");
                    assert!(
                        (fill.local_position.x
                            - fill.size.x * 0.5
                            - (base.local_position.x - base.size.x.abs() * 0.5 + 2.0))
                            .abs()
                            < 0.001,
                        "fixed left edge"
                    );
                    assert!(
                        (uv[2] - uv[0] - expected / texture_width).abs() < 0.001,
                        "{name} row {slot}, {value}% must preserve texture pitch: uv={uv:?}, width={expected}, texture={texture_width}"
                    );
                    assert_eq!([uv[0], uv[1], uv[3]], [0.0, 0.0, 1.0]);
                }
            }
        }
    }
}
