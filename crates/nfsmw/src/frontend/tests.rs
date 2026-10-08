//! The screens against the packages of the install (skipped without `NFSMW_GAME_DIR`): pad presses in, what the
//! screens ask for and show out.

use std::sync::Arc;

use blackbox_feng::runtime::pad;
use game_install::GameDir;

use super::ids::screen;
use super::logic::{Args, Category, Command, Env};
use super::screens::Screens;
use crate::settings::{Partial, Percent, Settings};
use crate::ui::{Catalog, SCREEN_FILES, UiAssets};

const FRAME: f32 = 1.0 / 60.0;

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
        let mut env = Env { settings: &mut h.settings, changed: &mut h.changed };
        h.commands = h.screens.open(name, args, true, &mut env);
        Some(h)
    }

    fn run(&mut self, mask: u32, frames: usize) {
        for _ in 0..frames {
            let mut env = Env { settings: &mut self.settings, changed: &mut self.changed };
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

#[test]
fn the_main_menu_starts_free_roam_from_career_and_quick_race() {
    let Some(mut h) = Harness::open(screen::MAIN_MENU, Args::default()) else { return };
    h.wait(1.0);
    h.press(pad::ACCEPT);
    h.wait(1.5);
    assert!(h.said(&Command::StartFreeRoam), "career: {:?}", h.commands);

    let mut h = Harness::open(screen::MAIN_MENU, Args::default()).unwrap();
    h.wait(1.0);
    // The challenge series are not there: right goes from Career to Quick Race.
    h.press(pad::RIGHT);
    h.press(pad::ACCEPT);
    h.wait(1.5);
    assert!(h.said(&Command::StartFreeRoam), "quick race: {:?}", h.commands);
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
fn options_and_back_move_between_the_main_menu_and_the_categories() {
    let Some(mut h) = Harness::open(screen::MAIN_MENU, Args::default()) else { return };
    h.wait(1.0);
    for _ in 0..4 {
        h.press(pad::RIGHT);
    }
    h.press(pad::ACCEPT);
    h.wait(1.5);
    assert_eq!(h.screens.top(), Some(screen::MAIN_MENU_SUB));
    h.wait(1.0);
    h.press(pad::BACK);
    h.wait(1.5);
    assert_eq!(h.screens.top(), Some(screen::MAIN_MENU), "{:?}", h.commands);
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
    h.wait(1.0);
    h.press(pad::RIGHT);
    assert!(!h.settings.vsync);
    h.press(pad::DOWN);
    h.press(pad::RIGHT);
    assert_eq!(h.settings.max_fps.to_string(), "30");
    for _ in 1..super::options::rows(Category::Video).len() {
        h.press(pad::DOWN);
    }
    h.press(pad::RIGHT);
    assert!(h.settings.vsync, "the selection wrapped to the first row: vsync is back on");
    let args = Args { category: Category::Gameplay, ..Args::default() };
    let mut h = Harness::open(screen::OPTIONS, args).unwrap();
    h.wait(1.0);
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

    let mut h = Harness::open(screen::PAUSE_MENU, pause).unwrap();
    h.wait(1.0);
    h.press(pad::RIGHT);
    h.press(pad::RIGHT);
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
        h.wait(1.0);
        for _ in 0..3 {
            h.press(pad::DOWN);
        }
        h.press(pad::RIGHT);
        assert_eq!(h.settings.window_mode, WindowMode::Borderless);
        h.press(pad::DOWN);
        h.press(pad::RIGHT);
        h.press(pad::DOWN);
        h.press(pad::RIGHT);
        assert!(!h.settings.tire_smoke && !h.settings.skid_marks);
        h.press(pad::DOWN);
        h.press(pad::RIGHT);
        assert_eq!(h.settings.smoke_quality, SmokeQuality::High);
        h.press(pad::BACK);
        h.wait(1.5);
        assert!(h.said(&Command::SaveSettings), "{name}: {:?}", h.commands);
        assert_eq!(h.changed.window_mode, Some(WindowMode::Borderless));
        assert_eq!(h.changed.tire_smoke, Some(false));
        assert_eq!(h.changed.skid_marks, Some(false));
        assert_eq!(h.changed.smoke_quality, Some(SmokeQuality::High));
    }
}
