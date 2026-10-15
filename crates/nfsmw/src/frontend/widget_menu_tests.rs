use super::*;
use blackbox_feng::Runtime;
use game_install::GameDir;
use std::collections::HashMap;

use crate::settings::{Partial, Settings};
use crate::ui::{Catalog, SCREEN_FILES, UiAssets};

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
