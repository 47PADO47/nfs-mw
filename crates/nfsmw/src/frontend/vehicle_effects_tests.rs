use super::ids::{LABEL_OFF, LABEL_ON};
use super::logic::Category;
use super::options::{Control, Data, Setting, Title, rows};
use crate::settings::{Partial, Settings, SmokeQuality};

#[test]
fn optional_vehicle_effect_rows_toggle_independently_of_tires_and_hud() {
    let rows = rows(Category::Video);
    for (index, setting, title) in
        [(7, Setting::CollisionSparks, "Collision Sparks"), (8, Setting::SpeedTrails, "Speed Trails")]
    {
        assert_eq!(rows[index].setting, setting);
        assert_eq!(rows[index].title, Title::Text(title));
        let initial = Settings::from(Partial {
            hud: Some(false),
            tire_smoke: Some(false),
            smoke_quality: Some(SmokeQuality::High),
            ..Partial::default()
        });
        for forward in [true, false] {
            let (mut settings, mut changes) = (initial, Partial::default());
            assert_eq!(setting.control(&settings), Control::Toggle);
            assert_eq!(setting.data(&settings), Data::Label(LABEL_OFF));
            for enabled in [true, false] {
                assert!(setting.step(&mut settings, &mut changes, forward));
                let (mut expected, mut written) = (initial, Partial::default());
                match setting {
                    Setting::CollisionSparks => {
                        expected.collision_sparks = enabled;
                        written.collision_sparks = Some(enabled);
                    }
                    Setting::SpeedTrails => {
                        expected.speed_trails = enabled;
                        written.speed_trails = Some(enabled);
                    }
                    _ => unreachable!(),
                }
                assert_eq!(settings, expected);
                assert_eq!(changes, written);
                assert_eq!(setting.data(&settings), Data::Label(if enabled { LABEL_ON } else { LABEL_OFF }));
            }
        }
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
        let mut env = Env { settings: &mut settings, changed: &mut changes };
        screens.open(name, Args { pause, category: Category::Video, ..Args::default() }, true, &mut env);
        run(&mut screens, &mut env, 0, 60);
        for _ in 0..7 {
            press(&mut screens, &mut env, pad::DOWN);
        }
        for (slot, title, setting) in
            [(8, "Collision Sparks", Setting::CollisionSparks), (9, "Speed Trails", Setting::SpeedTrails)]
        {
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
            for (button, enabled) in [(0, false), (pad::RIGHT, true), (pad::LEFT, false), (pad::RIGHT, true)] {
                if button != 0 {
                    press(&mut screens, &mut env, button);
                }
                let tree = screens.trees().pop().unwrap();
                let value = strings.get(if enabled { LABEL_ON } else { LABEL_OFF }).unwrap();
                for (object, text) in
                    [(format!("OPTION_NAME_{slot}"), title), (format!("OPTION_DATA_{slot}"), value.as_str())]
                {
                    let node = tree.nodes.iter().find(|node| node.name_hash == fe_hash_upper(&object) && node.visible);
                    assert_eq!(node.and_then(|node| node.text.as_deref()), Some(text), "{name}, {object}");
                }
                assert_eq!(setting.data(env.settings), Data::Label(if enabled { LABEL_ON } else { LABEL_OFF }));
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
            Partial { collision_sparks: Some(true), speed_trails: Some(true), ..Partial::default() }
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
