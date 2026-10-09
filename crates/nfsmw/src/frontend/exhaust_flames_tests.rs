use super::ids::{LABEL_OFF, LABEL_ON};
use super::logic::Category;
use super::options::{Control, Data, Setting, Title, rows};
use crate::settings::{Partial, Settings};

#[test]
fn the_video_screen_has_one_exhaust_flames_row_after_the_vehicle_effects() {
    let video = rows(Category::Video);
    let at = video.iter().position(|r| r.setting == Setting::ExhaustFlames).expect("a row");
    assert_eq!(video[at].title, Title::Text("Exhaust Flames"));
    assert_eq!(video[at - 1].setting, Setting::SpeedTrails);
    // The pause menu shows the same rows as the main menu.
    assert_eq!(video.iter().filter(|r| r.setting == Setting::ExhaustFlames).count(), 1);
    for category in [Category::Audio, Category::Gameplay] {
        assert!(rows(category).iter().all(|r| r.setting != Setting::ExhaustFlames));
    }
}

#[test]
fn the_row_toggles_both_ways_and_records_only_its_own_change() {
    let initial = Settings::from(Partial::default());
    assert_eq!(Setting::ExhaustFlames.control(&initial), Control::Toggle);
    assert_eq!(Setting::ExhaustFlames.data(&initial), Data::Label(LABEL_ON), "the default is on");
    for forward in [true, false] {
        let (mut settings, mut changed) = (initial, Partial::default());
        for enabled in [false, true] {
            assert!(Setting::ExhaustFlames.step(&mut settings, &mut changed, forward));
            assert_eq!(settings, Settings { exhaust_flames: enabled, ..initial });
            assert_eq!(changed, Partial { exhaust_flames: Some(enabled), ..Partial::default() });
            let label = if enabled { LABEL_ON } else { LABEL_OFF };
            assert_eq!(Setting::ExhaustFlames.data(&settings), Data::Label(label));
        }
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn main_and_pause_video_screens_show_the_exhaust_flames_row_and_save_it() {
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
    fn press(screens: &mut Screens, env: &mut Env<'_>, button: u32) -> Vec<Command> {
        let mut commands = run(screens, env, button, 3);
        commands.extend(run(screens, env, 0, 3));
        commands
    }

    let dir = GameDir::open(std::path::PathBuf::from(std::env::var_os("NFSMW_GAME_DIR").unwrap())).unwrap();
    let assets = Arc::new(UiAssets::load(&dir).unwrap());
    let strings = assets.strings.as_ref().expect("the option values need the game's language table");
    for (name, pause) in [(screen::OPTIONS, false), (screen::PAUSE_OPTIONS, true)] {
        let mut screens = Screens::new(Catalog::load(&dir, &SCREEN_FILES), assets.clone());
        let (mut settings, mut changes) = (Settings::from(Partial::default()), Partial::default());
        let mut env = Env { settings: &mut settings, changed: &mut changes };
        screens.open(name, Args { pause, category: Category::Video, ..Args::default() }, true, &mut env);
        run(&mut screens, &mut env, 0, 60);
        let at = rows(Category::Video).iter().position(|r| r.setting == Setting::ExhaustFlames).unwrap();
        for _ in 0..at {
            press(&mut screens, &mut env, pad::DOWN);
        }
        // Switch it off, then look at what the screen says in the visible rows.
        press(&mut screens, &mut env, pad::RIGHT);
        assert!(!env.settings.exhaust_flames, "{name}");
        let tree = screens.trees().pop().unwrap();
        let texts: Vec<&str> = tree.nodes.iter().filter(|n| n.visible).filter_map(|n| n.text.as_deref()).collect();
        assert!(texts.contains(&"Exhaust Flames"), "{name}: the row is on the screen");
        let off = strings.get(super::ids::LABEL_OFF).unwrap().to_string();
        assert!(texts.contains(&off.as_str()), "{name}: its value reads Off");
        let hash = fe_hash_upper("OPTION_NAME_1");
        assert!(tree.nodes.iter().any(|n| n.name_hash == hash), "{name}: the list still has its rows");
        let mut commands = Vec::new();
        for frame in 0..93 {
            let button = if frame < 3 { pad::BACK } else { 0 };
            commands.extend(run(&mut screens, &mut env, button, 1));
        }
        assert!(commands.contains(&Command::SaveSettings), "{name}: leaving saves");
        assert_eq!(env.changed.exhaust_flames, Some(false), "{name}");
    }
}
