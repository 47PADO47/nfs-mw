use super::logic::Category;
use super::options::{Control, Data, Setting, Title, rows};
use crate::settings::{HudLayout, Partial, Settings};

#[test]
fn hud_layout_row_follows_transmission_and_keeps_the_hud_enabled() {
    let rows = rows(Category::Gameplay, &crate::settings::test_caps::native());
    let index = rows.iter().position(|row| row.setting == Setting::Transmission).unwrap() + 1;
    assert_eq!(rows[index].setting, Setting::HudLayout);
    assert_eq!(rows[index].title, Title::Text("HUD Layout"));
    let (mut settings, mut changes) = (Settings::from(Partial::default()), Partial::default());
    assert_eq!(Setting::HudLayout.control(&settings), Control::Toggle);
    assert_eq!(Setting::HudLayout.data(&settings), Data::Text("PC".into()));
    for forward in [true, false] {
        let expected = match forward {
            true => [(HudLayout::Classic, "Centered"), (HudLayout::Xbox360, "Xbox 360"), (HudLayout::Pc, "PC")],
            false => [(HudLayout::Xbox360, "Xbox 360"), (HudLayout::Classic, "Centered"), (HudLayout::Pc, "PC")],
        };
        for (layout, title) in expected {
            assert!(Setting::HudLayout.step(&mut settings, &mut changes, forward));
            assert_eq!(settings.hud_layout, layout);
            assert_eq!(Setting::HudLayout.data(&settings), Data::Text(title.into()));
            assert_eq!(changes, Partial { hud_layout: Some(layout), ..Partial::default() });
            assert!(settings.hud);
        }
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn hud_layout_main_and_pause_rows_show_cycle_and_request_saving() {
    use std::sync::Arc;

    use blackbox_feng::{fe_hash_upper, runtime::pad};
    use game_install::GameDir;

    use super::ids::screen;
    use super::logic::{Args, Command, Env};
    use super::screens::Screens;
    use crate::ui::{Catalog, SCREEN_FILES, UiAssets};

    fn run(screens: &mut Screens, env: &mut Env<'_>, mask: u32, frames: usize) -> Vec<Command> {
        let mut commands = Vec::new();
        for _ in 0..frames {
            commands.extend(screens.update(1.0 / 60.0, mask, env));
        }
        commands
    }

    let dir = GameDir::open(std::path::PathBuf::from(std::env::var_os("NFSMW_GAME_DIR").unwrap())).unwrap();
    let assets = Arc::new(UiAssets::load(&dir).unwrap());
    for (name, pause) in [(screen::OPTIONS, false), (screen::PAUSE_OPTIONS, true)] {
        let catalog = Catalog::load(&dir, &SCREEN_FILES);
        let mut screens = Screens::new(catalog, assets.clone());
        let (mut settings, mut changes) = (Settings::from(Partial::default()), Partial::default());
        let mut env =
            Env { settings: &mut settings, changed: &mut changes, caps: crate::settings::test_caps::native() };
        screens.open(name, Args { pause, category: Category::Gameplay, ..Args::default() }, true, &mut env);
        run(&mut screens, &mut env, 0, 60);
        for _ in 0..2 {
            run(&mut screens, &mut env, pad::DOWN, 3);
            run(&mut screens, &mut env, 0, 3);
        }
        run(&mut screens, &mut env, pad::RIGHT, 3);
        run(&mut screens, &mut env, 0, 3);
        assert_eq!(env.settings.hud_layout, HudLayout::Classic);
        let tree = screens.trees().pop().unwrap();
        for (object, text) in [("OPTION_NAME_3", "HUD Layout"), ("OPTION_DATA_3", "Centered")] {
            let node = tree.nodes.iter().find(|node| node.name_hash == fe_hash_upper(object) && node.visible).unwrap();
            assert_eq!(node.text.as_deref(), Some(text), "{name}");
        }
        for (button, expected) in [(pad::RIGHT, HudLayout::Xbox360), (pad::LEFT, HudLayout::Classic)] {
            run(&mut screens, &mut env, button, 3);
            run(&mut screens, &mut env, 0, 3);
            assert_eq!(env.settings.hud_layout, expected, "{name}");
        }
        assert_eq!(env.settings.hud_layout, HudLayout::Classic);
        assert_eq!(env.changed.hud_layout, Some(HudLayout::Classic));
        let mut commands = Vec::new();
        for frame in 0..93 {
            let mask = match frame < 3 {
                true => pad::BACK,
                false => 0,
            };
            commands.extend(screens.update(1.0 / 60.0, mask, &mut env));
        }
        assert!(commands.contains(&Command::SaveSettings), "{name}");
    }
}
