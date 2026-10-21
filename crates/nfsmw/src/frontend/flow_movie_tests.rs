//! The movie stage of the flow: what a movie hands back when it is over, and the screen or command it leads to. Skipped
//! without `NFSMW_GAME_DIR`: a front end starts the track-board loaders, which keep their first result for the process,
//! so a front end over an empty folder would leave the other tests an empty board.

use std::sync::Arc;

use game_install::GameDir;

use super::flow::flow_movie::AfterMovie;
use super::flow::{Frontend, Stage, Start};
use super::ids::screen::{MAIN_MENU_SUB, SAFE_HOUSE_RIVAL_BIO};
use super::logic::{Args, Command, Env};
use super::screens::Screens;
use crate::app::{Host, test_backend::NullBackend};
use crate::settings::{Partial, Settings, test_caps};
use crate::ui::{Catalog, SCREEN_FILES, UiAssets};

/// A scene with nothing in it: the window's scene while the front end runs.
struct Still;

impl crate::viewer::Scene for Still {
    fn title(&self) -> String {
        String::new()
    }
    fn init(&mut self, _: &mut dyn blackbox_gfx::RenderBackend) -> anyhow::Result<()> {
        Ok(())
    }
    fn update(&mut self, _: &mut dyn blackbox_gfx::RenderBackend, _: &crate::input::ActionState, _: f32) {}
    fn frame(&mut self, _: f32) -> (blackbox_gfx::FrameParams, &[blackbox_gfx::Instance]) {
        unreachable!()
    }
}

/// The front end over `dir`, the settings it reads and a host with a renderer that draws nothing.
pub(super) struct Rig {
    pub(super) fe: Frontend,
    host: Host,
    settings: Settings,
    changed: Partial,
}

impl Rig {
    fn new(dir: GameDir, assets: UiAssets) -> Self {
        let settings = Settings::from(Partial::default());
        let catalog = Catalog::load(&dir, &SCREEN_FILES);
        let fe = Frontend::new(Screens::new(catalog, Arc::new(assets)), dir, Start::Drive, None);
        let mut host = Host::new(Box::new(Still), &settings, None);
        host.renderer = Some(Box::new(NullBackend::new(test_caps::native())));
        Self { fe, host, settings, changed: Partial::default() }
    }

    /// Runs `f` with the front end, the host and the environment the plugin gives them.
    pub(super) fn with<R>(&mut self, f: impl FnOnce(&mut Frontend, &mut Host, &mut Env) -> R) -> R {
        let mut env =
            Env { settings: &mut self.settings, changed: &mut self.changed, caps: test_caps::native(), mods: None };
        f(&mut self.fe, &mut self.host, &mut env)
    }
}

/// A front end over the install, or `None` without `NFSMW_GAME_DIR`.
pub(super) fn install_rig() -> Option<Rig> {
    let root = std::env::var_os("NFSMW_GAME_DIR")?;
    let dir = GameDir::open(std::path::PathBuf::from(root)).unwrap();
    let assets = UiAssets::load(&dir).unwrap();
    Some(Rig::new(dir, assets))
}

#[test]
fn a_movie_that_does_not_open_leaves_the_flow_where_it_was() {
    let Some(mut rig) = install_rig() else { return };
    rig.with(|fe, host, _| {
        fe.play_movie("no_such_movie", AfterMovie::Open(SAFE_HOUSE_RIVAL_BIO.to_owned(), Args::default()), host)
    });
    assert_eq!(rig.fe.stage, Stage::Menus, "no movie in the folder: nothing is played");
}

#[test]
fn a_movie_hands_the_flow_its_command_when_it_is_over() {
    let Some(mut rig) = install_rig() else { return };
    rig.fe.stage = Stage::Movie { then: AfterMovie::Command(Box::new(Command::QuitGame)) };
    rig.with(|fe, host, env| fe.end_movie(host, env));
    assert!(rig.fe.quit, "the command the movie was given runs");
    assert_eq!(rig.fe.stage, Stage::Menus);
}

#[test]
fn ending_when_no_movie_plays_changes_nothing() {
    let Some(mut rig) = install_rig() else { return };
    rig.fe.stage = Stage::Paused;
    rig.with(|fe, host, env| fe.end_movie(host, env));
    assert_eq!(rig.fe.stage, Stage::Paused, "only a movie's stage ends as a movie");
    assert!(!rig.fe.quit);
}

#[test]
fn a_story_movie_opens_its_screen_when_it_is_over() {
    let Some(mut rig) = install_rig() else { return };
    let command = Command::PlayMovie {
        movie: "storyfmv_saf25".to_owned(),
        then: AfterMovie::Open(SAFE_HOUSE_RIVAL_BIO.to_owned(), Args::default()),
    };
    rig.with(|fe, host, env| fe.apply(vec![command], host, env));
    assert!(matches!(rig.fe.stage, Stage::Movie { .. }), "the movie plays: {:?}", rig.fe.stage);
    assert_eq!(rig.fe.screens.top(), None, "no screen is up while the movie plays");
    rig.with(|fe, host, env| fe.end_movie(host, env));
    assert_eq!(rig.fe.stage, Stage::Menus);
    assert_eq!(rig.fe.screens.top(), Some(SAFE_HOUSE_RIVAL_BIO), "the story screen opens after the movie");
}

#[test]
fn a_tutorial_returns_to_the_screen_that_asked_for_it() {
    let Some(mut rig) = install_rig() else { return };
    let command = Command::PlayTutorial {
        movie: "drag_tutorial".to_owned(),
        back: MAIN_MENU_SUB.to_owned(),
        args: Args::default(),
    };
    rig.with(|fe, host, env| fe.apply(vec![command], host, env));
    assert!(matches!(rig.fe.stage, Stage::Movie { .. }), "the tutorial plays: {:?}", rig.fe.stage);
    rig.with(|fe, host, env| fe.end_movie(host, env));
    assert_eq!(rig.fe.screens.top(), Some(MAIN_MENU_SUB), "the screen that asked opens again");
}
