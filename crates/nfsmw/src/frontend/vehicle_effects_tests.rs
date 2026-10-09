use super::ids::LABEL_OFF;
use super::logic::Category;
use super::options::{Control, Data, Setting, Title, rows};
use crate::settings::{Partial, Settings, SmokeQuality};

#[test]
fn optional_vehicle_effect_rows_cycle_stock_and_experimental_independently() {
    use crate::settings::SparkStyle::{OriginalPc, RestoredExperimental};
    let rows = rows(Category::Video, &crate::settings::test_caps::native());
    assert_eq!(rows[7].title, Title::Text("Collision Sparks"));
    assert_eq!(rows[8].title, Title::Text("Speed Trails (Experimental)"));
    let initial = Settings::from(Partial {
        hud: Some(false),
        tire_smoke: Some(false),
        smoke_quality: Some(SmokeQuality::High),
        ..Partial::default()
    });
    for (forward, modes) in [
        (true, [OriginalPc, RestoredExperimental, OriginalPc]),
        (false, [RestoredExperimental, OriginalPc, OriginalPc]),
    ] {
        let (mut settings, mut changed) = (initial, Partial::default());
        for (i, mode) in modes.into_iter().enumerate() {
            Setting::CollisionSparks.step(&mut settings, &mut changed, forward);
            let enabled = i != 2;
            assert_eq!(settings, Settings { collision_sparks: enabled, spark_style: mode, ..initial });
            assert_eq!(
                changed,
                Partial { collision_sparks: Some(enabled), spark_style: Some(mode), ..Partial::default() }
            );
            let expected = match enabled {
                true => Data::Text(mode.label().into()),
                false => Data::Label(LABEL_OFF),
            };
            assert_eq!(Setting::CollisionSparks.data(&settings), expected);
        }
    }
    let (mut settings, mut changed) = (initial, Partial::default());
    assert_eq!(Setting::SpeedTrails.control(&settings), Control::Toggle);
    for enabled in [true, false] {
        Setting::SpeedTrails.step(&mut settings, &mut changed, true);
        assert_eq!(settings, Settings { speed_trails: enabled, ..initial });
        assert_eq!(changed, Partial { speed_trails: Some(enabled), ..Partial::default() });
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn vehicle_effects_main_and_pause_video_rows_show_toggle_and_save() {
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

    fn press(screens: &mut Screens, env: &mut Env<'_>, button: u32) {
        run(screens, env, button, 3);
        run(screens, env, 0, 3);
    }

    fn title_colour(screens: &Screens, slot: usize) -> ([u8; 4], [u8; 4]) {
        let tree = screens.trees().pop().unwrap();
        let hash = fe_hash_upper(&format!("OPTION_NAME_{slot}"));
        let node = tree.nodes.iter().find(|node| node.name_hash == hash).unwrap();
        (node.colour, node.world_colour)
    }

    let dir = GameDir::open(std::path::PathBuf::from(std::env::var_os("NFSMW_GAME_DIR").unwrap())).unwrap();
    let assets = Arc::new(UiAssets::load(&dir).unwrap());
    let strings = assets.strings.as_ref().expect("the option values need the game's language table");
    for (name, pause) in [(screen::OPTIONS, false), (screen::PAUSE_OPTIONS, true)] {
        let mut screens = Screens::new(Catalog::load(&dir, &SCREEN_FILES), assets.clone());
        let initial = Settings::from(Partial {
            hud: Some(false),
            tire_smoke: Some(false),
            smoke_quality: Some(SmokeQuality::High),
            ..Partial::default()
        });
        let (mut settings, mut changes) = (initial, Partial::default());
        let mut env =
            Env { settings: &mut settings, changed: &mut changes, caps: crate::settings::test_caps::native() };
        screens.open(name, Args { pause, category: Category::Video, ..Args::default() }, true, &mut env);
        run(&mut screens, &mut env, 0, 60);
        for _ in 0..7 {
            press(&mut screens, &mut env, pad::DOWN);
        }
        for (slot, title, setting) in [
            (8, "Collision Sparks", Setting::CollisionSparks),
            (9, "Speed Trails (Experimental)", Setting::SpeedTrails),
        ] {
            let native_rgb = title_colour(&screens, slot).0[..3].to_vec();
            // More than two complete 1200-tick highlight loops, including the transparent endpoints.
            for _ in 0..160 {
                run(&mut screens, &mut env, 0, 1);
                let (local, world) = title_colour(&screens, slot);
                assert_eq!(local[3], 255, "{name}, slot {slot}: selected custom title opacity");
                assert_eq!(world[3], 255, "{name}, slot {slot}: readable through its ancestors");
                assert_eq!(local[..3], native_rgb, "{name}, slot {slot}: authored RGB stays intact");
                assert!(world[..3].iter().copied().max().unwrap() >= 160, "{name}, slot {slot}: visible color");
            }
            let toggles = match setting {
                Setting::CollisionSparks => vec![
                    (0, false),
                    (pad::RIGHT, true),
                    (pad::RIGHT, true),
                    (pad::RIGHT, false),
                    (pad::LEFT, true),
                    (pad::LEFT, true),
                    (pad::LEFT, false),
                    (pad::RIGHT, true),
                ],
                _ => vec![(0, false), (pad::RIGHT, true), (pad::LEFT, false), (pad::RIGHT, true)],
            };
            for (button, enabled) in toggles {
                if button != 0 {
                    press(&mut screens, &mut env, button);
                }
                let tree = screens.trees().pop().unwrap();
                let data = setting.data(env.settings);
                let value = match &data {
                    Data::Label(hash) => strings.get(*hash).unwrap().to_string(),
                    Data::Text(text) => text.clone(),
                };
                for (object, text) in
                    [(format!("OPTION_NAME_{slot}"), title), (format!("OPTION_DATA_{slot}"), value.as_str())]
                {
                    let node = tree.nodes.iter().find(|node| node.name_hash == fe_hash_upper(&object) && node.visible);
                    assert_eq!(node.and_then(|node| node.text.as_deref()), Some(text), "{name}, {object}");
                }
                let actual = match setting {
                    Setting::CollisionSparks => env.settings.collision_sparks,
                    _ => env.settings.speed_trails,
                };
                assert_eq!(actual, enabled);
            }
            if slot == 8 {
                press(&mut screens, &mut env, pad::DOWN);
                run(&mut screens, &mut env, 0, 12);
                assert_eq!(title_colour(&screens, 8).0[3], 255, "{name}: deselected title settles normally");
            }
        }
        let expected = Settings { collision_sparks: true, speed_trails: true, ..initial };
        assert_eq!(*env.settings, expected, "{name}: other visual settings stay unchanged");
        assert_eq!(
            *env.changed,
            Partial {
                collision_sparks: Some(true),
                spark_style: Some(crate::settings::SparkStyle::OriginalPc),
                speed_trails: Some(true),
                ..Partial::default()
            }
        );
        run(&mut screens, &mut env, 0, 30);
        let mut commands = Vec::new();
        let mut native_exit_alpha = false;
        for frame in 0..93 {
            let button = match frame < 3 {
                true => pad::BACK,
                false => 0,
            };
            commands.extend(run(&mut screens, &mut env, button, 1));
            if screens.top() == Some(name) {
                native_exit_alpha |= title_colour(&screens, 9).0[3] < 255;
            }
        }
        assert!(native_exit_alpha, "{name}: leaving restores the authored title alpha animation");
        assert!(commands.contains(&Command::SaveSettings), "{name}");
    }
}
