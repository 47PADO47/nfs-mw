//! The screens against the packages of the install (skipped without `NFSMW_GAME_DIR`): pad presses in, what the
//! screens ask for and show out.

use std::sync::Arc;

use blackbox_feng::runtime::pad;
use game_install::GameDir;

use super::ids::screen;
use super::logic::{Args, Category, Command, Env};
use super::screens::Screens;
use crate::settings::{Partial, Percent, Settings, test_caps};
use crate::ui::{Catalog, SCREEN_FILES, UiAssets};

const FRAME: f32 = 1.0 / 60.0;

#[path = "prompt_tests.rs"]
mod prompt_tests;

#[path = "click_tests.rs"]
mod click_tests;

#[path = "icon_transition_tests.rs"]
mod icon_transition_tests;

#[path = "player_options_tests.rs"]
mod player_options_tests;

#[test]
fn authored_right_arrow_mirrors_and_cursor_brackets_pulse() {
    use glam::Vec3;
    let Some(mut h) = Harness::open(screen::MAIN_MENU, Args::default()) else { return };
    h.wait(1.0);
    let tree = h.screens.trees().pop().unwrap();
    let right = tree.nodes.iter().find(|n| n.name_hash == 0xFCD5_B255).unwrap();
    assert!(right.world.transform_vector3(Vec3::X).x < 0.0);
    let width = |tree: &blackbox_feng::UiTree| {
        let cursor = tree.nodes.iter().find(|n| n.name_hash == 0x557F_EADD).unwrap();
        cursor.world.transform_vector3(Vec3::X).length()
    };
    let first = width(&tree);
    h.wait(0.35);
    assert!((width(&h.screens.trees().pop().unwrap()) - first).abs() > 0.01);
}

#[test]
fn every_option_category_has_current_prompts_and_no_legacy_duplicates() {
    use crate::input::{Bindings, InputDevice};
    use crate::ui::input_icons::{ATLAS, Glyph};
    use blackbox_feng::NodeKind;
    for (name, pause) in [(screen::OPTIONS, false), (screen::PAUSE_OPTIONS, true)] {
        for category in [
            Category::Audio,
            Category::Video,
            Category::Gameplay,
            Category::Player,
            Category::Controls,
            Category::Discord,
        ] {
            let Some(mut h) = Harness::open(name, Args { pause, category, ..Args::default() }) else { return };
            h.wait(1.0);
            let mut bindings = Bindings::default();
            bindings.bind(crate::input::Action::MenuBack, "button:West", false).unwrap();
            let tree = h.screens.presented_trees(&bindings, InputDevice::Xbox).pop().unwrap();
            let hints: Vec<_> = tree.draw_order.iter().map(|i| &tree.nodes[*i]).collect();
            assert!(hints.iter().any(|n| n.text.as_deref() == Some("Done")));
            assert!(
                hints
                    .iter()
                    .any(|n| matches!(n.kind, NodeKind::Image { texture: ATLAS, uv, .. } if uv == Glyph::X.uv()))
            );
            assert!(
                !hints
                    .iter()
                    .any(|n| n.text.as_deref().is_some_and(|s| matches!(s.trim(), "Accept" | "Back" | "Defaults")))
            );
            let tree = h.screens.presented_trees(&bindings, InputDevice::Keyboard).pop().unwrap();
            assert!(
                !tree.draw_order.iter().any(|i| matches!(tree.nodes[*i].kind, NodeKind::Image { texture: ATLAS, .. }))
            );
        }
    }
}

#[test]
fn start_in_frontend_categories_cannot_resume_an_absent_driving_scene() {
    let Some(mut h) = Harness::open(screen::MAIN_MENU_SUB, Args { options: true, ..Args::default() }) else { return };
    h.wait(1.0);
    h.press(pad::START);
    h.wait(1.5);
    assert!(!h.said(&Command::Resume));
    assert_eq!(h.screens.top(), Some(screen::MAIN_MENU_SUB));
}

#[test]
fn title_prompts_follow_the_authored_position_and_fade_above_the_copyright() {
    use crate::input::{Bindings, InputDevice};
    use glam::Vec3;
    for name in [screen::SPLASH, screen::SPLASH_WIDE] {
        let Some(mut h) = Harness::open(name, Args::default()) else { return };
        h.wait(6.0);
        let native = h.screens.trees().pop().unwrap();
        let anchor = native.nodes.iter().find(|n| n.name_hash == 0xC4DF_3FF2).unwrap();
        let copyright = native.nodes.iter().find(|n| n.name_hash == 0x5B9D_88B9).unwrap();
        let tree = h.screens.presented_trees(&Bindings::default(), InputDevice::Xbox).pop().unwrap();
        let hint = tree.nodes[native.nodes.len()..]
            .iter()
            .find(|n| matches!(n.kind, blackbox_feng::NodeKind::Image { .. }))
            .unwrap();
        let y = hint.world.transform_point3(Vec3::ZERO).y;
        assert!((y - anchor.world.transform_point3(Vec3::ZERO).y).abs() < 0.01);
        assert_eq!(hint.world_colour[3], anchor.world_colour[3]);
        assert!((y - copyright.world.transform_point3(Vec3::ZERO).y).abs() > 20.0);
    }
}

#[test]
fn controls_icon_opens_the_key_list_and_control_options_the_sensitivity_rows() {
    for (menu, rows, keys, pause) in [
        (screen::MAIN_MENU_SUB, screen::OPTIONS, screen::CONTROLS, false),
        (screen::PAUSE_MENU, screen::PAUSE_OPTIONS, screen::PAUSE_CONTROLS, true),
    ] {
        let Some(mut h) = Harness::open(menu, Args { options: true, pause, ..Args::default() }) else { return };
        h.wait(1.0);
        let title_is = |h: &Harness, text: &str| {
            h.screens.trees().iter().any(|tree| {
                tree.nodes.iter().any(|n| n.name_hash == super::ids::ICON_TITLE && n.text.as_deref() == Some(text))
            })
        };
        // Audio, Video, Gameplay, Player, then Controls in both category strips.
        let to_controls = 4;
        for _ in 0..to_controls {
            h.press(pad::RIGHT);
        }
        h.wait(0.5);
        assert!(title_is(&h, "Controls"), "{menu}: the Controls category");
        // Hold A through the switch. The next screen must not accept the same gesture again.
        h.run(pad::ACCEPT, 120);
        h.run(0, 3);
        h.wait(1.5);
        assert_eq!(h.screens.top(), Some(keys), "{menu}: Controls opens the key list");
        h.press(pad::BACK);
        h.wait(1.5);
        assert_eq!(h.screens.top(), Some(menu), "{menu}: back returns to the categories");
        assert!(title_is(&h, "Controls"), "{menu}: return must keep Controls selected");
        // Control Options, the next category, opens the sensitivity and camera rows.
        h.press(pad::RIGHT);
        h.wait(0.5);
        assert!(title_is(&h, "Control Options"), "{menu}: the sixth category");
        h.run(pad::ACCEPT, 120);
        h.run(0, 3);
        h.wait(1.5);
        assert_eq!(h.screens.top(), Some(rows), "{menu}: Control Options opens the rows");
        h.wait(1.0);
        h.press(pad::RIGHT);
        assert!(h.changed.deadzone_mode.is_some(), "{menu}: {:?}", h.changed);
        h.press(pad::BACK);
        h.wait(1.5);
        assert!(h.said(&Command::SaveSettings));
        assert_eq!(h.screens.top(), Some(menu));
        assert!(title_is(&h, "Control Options"), "{menu}: return must keep Control Options selected");
        h.press(pad::LEFT);
        h.wait(0.5);
        assert!(title_is(&h, "Controls"), "{menu}: moving back restores the Controls label");
    }
}

struct Harness {
    screens: Screens,
    settings: Settings,
    changed: Partial,
    commands: Vec<Command>,
}

impl Harness {
    fn open(name: &str, args: Args) -> Option<Self> {
        let root = std::env::var_os("NFSMW_GAME_DIR")?;
        let dir = GameDir::open(std::path::PathBuf::from(root)).unwrap();
        let assets = Arc::new(UiAssets::load(&dir).unwrap());
        let catalog = Catalog::load(&dir, &SCREEN_FILES);
        let mut h = Self {
            screens: Screens::new(catalog, assets),
            settings: Settings::from(Partial::default()),
            changed: Partial::default(),
            commands: Vec::new(),
        };
        let mut env = Env { settings: &mut h.settings, changed: &mut h.changed, caps: test_caps::native(), mods: None };
        h.commands = h.screens.open(name, args, true, &mut env);
        Some(h)
    }

    fn run(&mut self, mask: u32, frames: usize) {
        for _ in 0..frames {
            let mut env =
                Env { settings: &mut self.settings, changed: &mut self.changed, caps: test_caps::native(), mods: None };
            self.commands.extend(self.screens.update(FRAME, mask, &mut env));
        }
    }

    fn wait(&mut self, seconds: f32) {
        self.run(0, (seconds / FRAME) as usize);
    }

    /// A press and a release.
    fn press(&mut self, button: u32) {
        self.run(button, 3);
        self.run(0, 3);
    }

    fn said(&self, command: &Command) -> bool {
        self.commands.contains(command)
    }
}

/// Presses down from the row of `from` to the row of `to`, counted from the rows the renderer offers.
fn down_from_to(h: &mut Harness, category: Category, from: super::options::Setting, to: super::options::Setting) {
    let caps = crate::settings::test_caps::native();
    let rows = super::options::rows(category, &caps);
    let index = |setting| rows.iter().position(|row| row.setting == setting).expect("the category has the row");
    for _ in index(from)..index(to) {
        h.press(pad::DOWN);
    }
}

#[test]
fn the_main_menu_starts_a_career_from_career() {
    let Some(mut h) = Harness::open(screen::MAIN_MENU, Args::default()) else { return };
    h.wait(1.0);
    h.press(pad::ACCEPT);
    h.wait(1.5);
    assert!(h.said(&Command::StartCareer), "career: {:?}", h.commands);
}

#[test]
fn quick_race_opens_the_track_select_screen() {
    let Some(mut h) = Harness::open(screen::MAIN_MENU, Args::default()) else { return };
    h.wait(1.0);
    // Career, Challenge Series, Quick Race: the challenge series is browsable but not there yet.
    h.press(pad::RIGHT);
    h.press(pad::RIGHT);
    h.press(pad::ACCEPT);
    h.wait(1.5);
    // The screens carry out the switch themselves, so it is the screen on top that shows it happened.
    assert_eq!(h.screens.top(), Some(screen::TRACK_SELECT), "quick race: {:?}", h.commands);
    assert!(!h.said(&Command::StartFreeRoam), "quick race must not start a race: {:?}", h.commands);
}

#[test]
fn track_options_accept_starts_nothing() {
    let Some(mut h) = Harness::open(screen::TRACK_OPTIONS, Args::default()) else { return };
    h.wait(1.0);
    h.press(pad::ACCEPT);
    h.wait(1.5);
    assert!(!h.said(&Command::StartFreeRoam), "{:?}", h.commands);
    assert!(!h.commands.iter().any(|c| matches!(c, Command::Switch(..) | Command::Push(..))), "{:?}", h.commands);
}

#[test]
fn the_quit_button_of_the_main_menu_quits() {
    let Some(mut h) = Harness::open(screen::MAIN_MENU, Args::default()) else { return };
    h.wait(1.0);
    h.screens.post_to_top(super::ids::QUIT_BUTTON, super::ids::MOUSE_LEFT_RELEASED);
    h.wait(0.2);
    assert!(h.said(&Command::QuitGame), "{:?}", h.commands);
}

#[test]
fn a_click_focuses_the_visible_button_under_the_pointer() {
    use glam::Vec3;
    let Some(mut h) = Harness::open(screen::MAIN_MENU, Args::default()) else { return };
    h.wait(1.0);
    let tree = h.screens.trees().pop().unwrap();
    let visible = |b: &blackbox_feng::ObjectRef| tree.nodes.iter().any(|n| n.index == b.index && n.visible);
    let button = h.screens.top_buttons().into_iter().find(visible).expect("the main menu shows a button");
    let node = tree.nodes.iter().find(|n| n.index == button.index).unwrap();
    let centre = node.world.transform_point3(Vec3::ZERO).truncate();
    assert!(h.screens.focus_button_at(centre), "the centre of a visible button is on it");
    assert_eq!(h.screens.focused_button(), Some(button));
    assert!(!h.screens.focus_button_at(glam::Vec2::new(-600.0, 0.0)), "far off every button");
}

#[test]
fn a_category_opens_its_rows_and_a_row_changes_a_setting_at_once() {
    let Some(mut h) = Harness::open(screen::MAIN_MENU_SUB, Args { options: true, ..Args::default() }) else { return };
    h.wait(1.0);
    h.press(pad::ACCEPT);
    h.wait(1.5);
    assert_eq!(h.screens.top(), Some(screen::OPTIONS));
    h.wait(1.0);
    assert_eq!(h.settings.master_volume, Percent(80));
    h.press(pad::LEFT);
    assert_eq!(h.settings.master_volume, Percent(70), "the first audio row is the master volume");
    assert_eq!(h.changed.master_volume, Some(Percent(70)));
    h.press(pad::DOWN);
    h.press(pad::RIGHT);
    assert_eq!(h.settings.sfx_volume, Percent(100));
    h.press(pad::ACCEPT);
    h.wait(1.5);
    assert!(h.said(&Command::SaveSettings), "{:?}", h.commands);
    assert_eq!(h.screens.top(), Some(screen::MAIN_MENU_SUB), "back to the categories");
}

#[test]
fn the_video_rows_change_vsync_and_the_hud_row_the_hud() {
    let args = Args { category: Category::Video, ..Args::default() };
    let Some(mut h) = Harness::open(screen::OPTIONS, args) else { return };
    use super::options::Setting;
    h.wait(1.0);
    down_from_to(&mut h, Category::Video, Setting::WindowMode, Setting::Vsync);
    h.press(pad::RIGHT);
    assert!(!h.settings.vsync);
    h.press(pad::DOWN);
    h.press(pad::RIGHT);
    assert_eq!(h.settings.max_fps.to_string(), "30");
    // Down from Frame Limit past the end of the list: the selection wraps to the first row (window mode).
    let rows = super::options::rows(Category::Video, &crate::settings::test_caps::native());
    let max_fps = rows.iter().position(|row| row.setting == Setting::MaxFps).expect("frame limit row");
    for _ in max_fps..rows.len() {
        h.press(pad::DOWN);
    }
    h.press(pad::RIGHT);
    assert!(h.changed.window_mode.is_some(), "the selection wrapped to the first row: window mode changes");
    let args = Args { category: Category::Player, ..Args::default() };
    let mut h = Harness::open(screen::OPTIONS, args).unwrap();
    h.wait(1.0);
    // Player starts with Transmission, then Camera and Gauges.
    assert_eq!(h.settings.transmission, crate::settings::Transmission::Automatic);
    h.press(pad::RIGHT);
    assert_eq!(h.settings.transmission, crate::settings::Transmission::Manual);
    assert_eq!(h.changed.transmission, Some(crate::settings::Transmission::Manual));
    h.press(pad::DOWN);
    h.press(pad::DOWN);
    assert!(h.settings.hud);
    h.press(pad::LEFT);
    assert!(!h.settings.hud);
    assert_eq!(h.changed.hud, Some(false));
}

#[test]
fn the_pause_menu_resumes_opens_the_options_and_quits() {
    let pause = Args { pause: true, ..Args::default() };
    let Some(mut h) = Harness::open(screen::PAUSE_MENU, pause) else { return };
    h.wait(1.0);
    h.press(pad::ACCEPT);
    h.wait(1.5);
    assert!(h.said(&Command::Resume), "{:?}", h.commands);

    let mut h = Harness::open(screen::PAUSE_MENU, pause).unwrap();
    h.wait(1.0);
    h.press(pad::START);
    h.wait(1.5);
    assert!(h.said(&Command::Resume), "start resumes: {:?}", h.commands);

    let mut h = Harness::open(screen::PAUSE_MENU, pause).unwrap();
    h.wait(1.0);
    h.press(pad::RIGHT);
    h.press(pad::ACCEPT);
    h.wait(1.5);
    assert_eq!(h.screens.top(), Some(screen::PAUSE_MENU), "the options categories are in the same package");
    h.press(pad::ACCEPT);
    h.wait(1.5);
    assert_eq!(h.screens.top(), Some(screen::PAUSE_OPTIONS));
    h.press(pad::BACK);
    h.wait(1.5);
    assert_eq!(h.screens.top(), Some(screen::PAUSE_MENU));

    // Quit asks first: No (the default) keeps the pause menu.
    let mut h = Harness::open(screen::PAUSE_MENU, pause).unwrap();
    h.wait(1.0);
    h.press(pad::RIGHT);
    h.press(pad::RIGHT);
    h.press(pad::ACCEPT);
    h.wait(1.5);
    assert_eq!(h.screens.top(), Some(screen::DIALOG), "quit asks first: {:?}", h.commands);
    h.press(pad::ACCEPT);
    h.wait(1.5);
    assert_eq!(h.screens.top(), Some(screen::PAUSE_MENU), "no keeps the game paused");
    assert!(!h.said(&Command::QuitToMenu), "{:?}", h.commands);
    // The pause menu still reacts after the question: Start resumes.
    h.press(pad::START);
    h.wait(1.5);
    assert!(h.said(&Command::Resume), "the pause menu froze after No: {:?}", h.commands);

    // Yes quits to the main menu.
    let mut h = Harness::open(screen::PAUSE_MENU, pause).unwrap();
    h.wait(1.0);
    h.press(pad::RIGHT);
    h.press(pad::RIGHT);
    h.press(pad::ACCEPT);
    h.wait(1.5);
    h.press(pad::LEFT);
    h.press(pad::ACCEPT);
    h.wait(1.5);
    assert!(h.said(&Command::QuitToMenu), "{:?}", h.commands);
}

#[test]
fn the_title_screen_waits_five_seconds_for_a_press() {
    let Some(mut h) = Harness::open(screen::SPLASH, Args::default()) else { return };
    h.wait(1.0);
    h.press(pad::ACCEPT);
    h.wait(1.0);
    assert!(!h.said(&Command::NextBootStep), "too early");
    h.wait(4.5);
    h.press(pad::START);
    h.wait(0.2);
    assert!(h.said(&Command::NextBootStep), "{:?}", h.commands);
}

#[test]
fn main_and_pause_video_options_change_and_save_display_and_tire_settings() {
    use crate::settings::{SmokeQuality, WindowMode};
    for (name, pause) in [(screen::OPTIONS, false), (screen::PAUSE_OPTIONS, true)] {
        let args = Args { pause, category: Category::Video, ..Args::default() };
        let Some(mut h) = Harness::open(name, args) else { return };
        use super::options::Setting::{SkidMarks, SmokeQuality as Smoke, TireSmoke, WindowMode as Window};
        h.wait(1.0);
        h.press(pad::RIGHT);
        assert_eq!(h.settings.window_mode, WindowMode::Borderless);
        // Smoke quality is dimmed while tire smoke is off, so it is set first.
        down_from_to(&mut h, Category::Video, Window, Smoke);
        h.press(pad::RIGHT);
        assert_eq!(h.settings.smoke_quality, SmokeQuality::High);
        h.press(pad::UP);
        h.press(pad::RIGHT);
        down_from_to(&mut h, Category::Video, TireSmoke, SkidMarks);
        h.press(pad::RIGHT);
        assert!(!h.settings.tire_smoke && !h.settings.skid_marks);
        h.press(pad::BACK);
        h.wait(1.5);
        assert!(h.said(&Command::SaveSettings), "{name}: {:?}", h.commands);
        assert_eq!(h.changed.window_mode, Some(WindowMode::Borderless));
        assert_eq!(h.changed.tire_smoke, Some(false));
        assert_eq!(h.changed.skid_marks, Some(false));
        assert_eq!(h.changed.smoke_quality, Some(SmokeQuality::High));
    }
}

#[test]
fn controls_options_show_every_response_row_with_visible_values() {
    use super::options::{Data, Title, rows};
    for (name, pause, visible) in [(screen::OPTIONS, false, 9), (screen::PAUSE_OPTIONS, true, 10)] {
        let args = Args { pause, category: Category::Controls, ..Args::default() };
        let Some(mut h) = Harness::open(name, args) else { return };
        h.wait(1.0);
        let rows = rows(Category::Controls, &crate::settings::test_caps::native());
        for (index, row) in rows.iter().enumerate() {
            let slot = index.min(visible - 1) + 1;
            let tree = h.screens.trees().pop().unwrap();
            if let Title::Text(title) = row.title {
                let hash = blackbox_feng::fe_hash_upper(&format!("OPTION_NAME_{slot}"));
                let node =
                    tree.nodes.iter().find(|node| node.name_hash == hash && node.visible).expect("visible option name");
                assert_eq!(node.text.as_deref(), Some(title), "{name}, row {index}");
            }
            if let Data::Text(value) = row.setting.data(&h.settings) {
                let hash = blackbox_feng::fe_hash_upper(&format!("OPTION_DATA_{slot}"));
                let node = tree
                    .nodes
                    .iter()
                    .find(|node| node.name_hash == hash && node.visible)
                    .expect("visible option value");
                assert_eq!(node.text.as_deref(), Some(value.as_str()), "{name}, row {index}");
            }
            h.press(pad::DOWN);
        }
        // Wrapped to the first row; down to Invert Camera Y (its place in the list, not the last row) and toggle it.
        let invert = super::options::Setting::Input(super::input_options::InputSetting::InvertCameraY);
        down_from_to(&mut h, Category::Controls, rows[0].setting, invert);
        h.press(pad::RIGHT);
        assert!(h.settings.controls.invert_camera_y);
        h.press(pad::BACK);
        h.wait(1.5);
        assert!(h.said(&Command::SaveSettings));
        assert_eq!(h.changed.invert_camera_y, Some(true));
    }
}

#[test]
fn main_and_pause_minimap_options_apply_and_request_a_save() {
    for (name, pause) in [(screen::OPTIONS, false), (screen::PAUSE_OPTIONS, true)] {
        let args = Args { pause, category: Category::Gameplay, ..Args::default() };
        let Some(mut h) = Harness::open(name, args) else { return };
        h.wait(1.0);
        let row = super::options::rows(Category::Gameplay, &crate::settings::test_caps::native())
            .iter()
            .position(|r| r.setting == super::options::Setting::Minimap)
            .unwrap();
        for _ in 0..row {
            h.press(pad::DOWN);
        }
        h.press(pad::RIGHT);
        assert_eq!(h.settings.minimap, crate::settings::MinimapMode::Rotating);
        assert_eq!(h.changed.minimap, Some(crate::settings::MinimapMode::Rotating));
        h.press(pad::BACK);
        h.wait(1.5);
        assert!(h.said(&Command::SaveSettings));
    }
}
