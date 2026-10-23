//! The screen against the installed package (skipped without `NFSMW_GAME_DIR`), over a small list of events.

use std::sync::Arc;

use blackbox_feng::runtime::pad;
use game_install::GameDir;

use super::kinds::{Kind, MILESTONES};
use super::view::Crop;
use super::*;
use crate::frontend::screens::Screens;
use crate::settings::{Partial, Settings, test_caps};
use crate::ui::{Catalog, SCREEN_FILES, UiAssets};

const FRAME: f32 = 1.0 / 60.0;

fn entry(key: u32, event: &str, kind: Kind, open: bool, unlocks: &[u32]) -> Entry {
    Entry {
        key,
        name: format!("race_bin_challenge/{event}"),
        event: event.into(),
        kind,
        tag: key as i32,
        goal: 60.0,
        open_at_start: open,
        unlocks: unlocks.to_vec(),
        crop: Crop::WHOLE,
        map_texture: 0,
    }
}

/// Five events: a tollbooth trial and a pursuit that start open, and three locked ones.
fn board() -> Arc<Board> {
    let pursuit = Kind::Pursuit(MILESTONES[0]);
    Arc::new(Board {
        entries: vec![
            entry(1, "19.8.32", Kind::Race, true, &[3]),
            entry(2, "19.8.1", pursuit, true, &[]),
            entry(3, "19.8.33", Kind::Race, false, &[]),
            entry(4, "19.8.2", pursuit, false, &[]),
            entry(5, "19.8.34", Kind::Race, false, &[]),
        ],
    })
}

struct Harness {
    screens: Screens,
    settings: Settings,
    changed: Partial,
    commands: Vec<Command>,
}

impl Harness {
    fn open() -> Option<Self> {
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
        let logic = ChallengeSeries::with(board(), Progress::default());
        let mut env = Env { settings: &mut h.settings, changed: &mut h.changed, caps: test_caps::native(), mods: None };
        h.commands = h.screens.open_with(screen::CHALLENGE_SERIES, Box::new(logic), true, &mut env);
        h.run(0, 90);
        Some(h)
    }

    fn run(&mut self, mask: u32, frames: usize) {
        for _ in 0..frames {
            let mut env =
                Env { settings: &mut self.settings, changed: &mut self.changed, caps: test_caps::native(), mods: None };
            self.commands.extend(self.screens.update(FRAME, mask, &mut env));
        }
    }

    fn press(&mut self, button: u32) {
        self.run(button, 3);
        self.run(0, 30);
    }

    /// The text of the first string object with this name hash on the top screen.
    fn text(&self, name: u32) -> Option<String> {
        let tree = self.screens.trees().pop()?;
        tree.nodes.iter().find(|n| n.name_hash == name)?.text.clone()
    }
}

use crate::frontend::logic::Env;

#[test]
fn the_counter_follows_the_selection_and_right_wraps_inside_the_row() {
    let Some(mut h) = Harness::open() else { return };
    assert_eq!(h.text(0x5B80_5EE5).as_deref(), Some("1"));
    h.press(pad::RIGHT);
    assert_eq!(h.text(0x5B80_5EE5).as_deref(), Some("2"));
    h.press(pad::DOWN);
    assert_eq!(h.text(0x5B80_5EE5).as_deref(), Some("5"), "one row down in a grid four wide");
}

#[test]
fn accept_on_a_locked_event_asks_nothing() {
    let Some(mut h) = Harness::open() else { return };
    h.press(pad::RIGHT);
    h.press(pad::RIGHT);
    h.press(pad::ACCEPT);
    assert_eq!(h.screens.top(), Some(screen::CHALLENGE_SERIES), "{:?}", h.commands);
}

#[test]
fn accept_on_an_open_event_asks_then_offers_the_transmission_and_starts() {
    let Some(mut h) = Harness::open() else { return };
    h.press(pad::ACCEPT);
    assert_eq!(h.screens.top(), Some(screen::DIALOG), "the launch question");
    h.press(pad::LEFT);
    h.press(pad::ACCEPT);
    assert_eq!(h.screens.top(), Some(screen::DIALOG), "the transmission question follows Yes");
    h.press(pad::ACCEPT);
    let started = h
        .commands
        .iter()
        .any(|c| matches!(c, Command::StartChallenge { race, .. } if race == "race_bin_challenge/19.8.32"));
    assert!(started, "{:?}", h.commands);
}

#[test]
fn no_on_the_launch_question_goes_back_to_the_grid() {
    let Some(mut h) = Harness::open() else { return };
    h.press(pad::ACCEPT);
    h.press(pad::ACCEPT);
    assert_eq!(h.screens.top(), Some(screen::CHALLENGE_SERIES));
    assert!(!h.commands.iter().any(|c| matches!(c, Command::StartChallenge { .. })));
}

#[test]
fn the_tutorial_button_plays_the_tollbooth_movie_only_on_a_tollbooth_event() {
    let Some(mut h) = Harness::open() else { return };
    h.press(pad::BUTTON0 << 4);
    assert!(plays(&h.commands, "tollbooth_tutorial"), "{:?}", h.commands);
    h.commands.clear();
    h.press(pad::RIGHT);
    h.press(pad::BUTTON0 << 4);
    assert!(plays(&h.commands, "pursuit_tutorial"), "{:?}", h.commands);
}

impl Harness {
    /// The centre of the named object on the top screen.
    fn centre(&self, name: &str) -> glam::Vec2 {
        let tree = self.screens.trees().pop().unwrap();
        let hash = blackbox_feng::fe_hash_upper(name);
        let node = tree.nodes.iter().find(|n| n.name_hash == hash).unwrap_or_else(|| panic!("no {name}"));
        node.world.transform_point3(glam::Vec3::ZERO).truncate()
    }

    fn click(&mut self, point: glam::Vec2) -> bool {
        let mut env =
            Env { settings: &mut self.settings, changed: &mut self.changed, caps: test_caps::native(), mods: None };
        let Some(commands) = self.screens.click(point, &mut env) else { return false };
        self.commands.extend(commands);
        true
    }
}

#[test]
fn a_click_on_an_open_event_asks_to_launch_it_and_on_a_locked_one_does_nothing() {
    let Some(mut h) = Harness::open() else { return };
    let locked = h.centre("EVENT_ICON_3");
    crate::audio::menu::take();
    assert!(h.click(locked), "the screen takes a click on an icon");
    h.run(0, 60);
    assert_eq!(h.screens.top(), Some(screen::CHALLENGE_SERIES), "a locked event asks nothing: {:?}", h.commands);
    assert!(crate::audio::menu::take().is_empty(), "locked event click stays silent after updates");
    let open = h.centre("EVENT_ICON_2");
    assert!(h.click(open));
    h.run(0, 60);
    assert_eq!(h.screens.top(), Some(screen::DIALOG), "an open event asks to launch: {:?}", h.commands);
    let sounds = crate::audio::menu::take();
    assert_eq!(sounds.iter().filter(|&&id| id == 4).count(), 1, "{sounds:?}");
}

/// Whether the commands play this tutorial and come back to the series.
fn plays(commands: &[Command], movie: &str) -> bool {
    commands.iter().any(
        |c| matches!(c, Command::PlayTutorial { movie: m, back, .. } if m == movie && back == screen::CHALLENGE_SERIES),
    )
}
