//! The game flow: boot movies, the title screen, the menus, free roam and the pause menu
//! (docs/specs/frontend-menus.md, section 4).

use std::collections::VecDeque;

use bevy_ecs::prelude::*;
use game_install::GameDir;

use super::logic::{Args, Command, Env};
use super::scene::{MenuScene, Pausable, PauseFlag};
use super::screens::Screens;
use super::script::UiScript;
use crate::app::Host;
use crate::scenes::world::{DEFAULT_CAR, DriveOptions, Options, WorldScene};
use crate::settings::{Partial, Settings};

/// Where the front end starts.
#[derive(Clone, Debug, PartialEq)]
pub enum Start {
    /// The whole boot flow: movies, the title screen, the main menu.
    Boot,
    /// The main menu.
    Menu,
    /// One screen, as `view-screen` shows it.
    Screen(String, Args),
    /// Straight into free roam.
    Drive,
}

/// A step of the boot flow (the original `BootFlowManager`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Boot {
    /// A movie from `MOVIES/`; accept or start skips it when `bypass` is set.
    Movie { name: &'static str, bypass: bool },
    /// The title screen.
    Splash,
}

/// The boot order: the EA logo, the public service announcement, the title screen.
pub const BOOT: [Boot; 3] =
    [Boot::Movie { name: "ealogo", bypass: false }, Boot::Movie { name: "psa", bypass: true }, Boot::Splash];

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Stage {
    Boot(Boot),
    Menus,
    Playing,
    Paused,
}

use super::ids::screen::{MAIN_MENU, PAUSE_MENU, SPLASH};

#[derive(Resource)]
pub struct Frontend {
    pub(super) screens: Screens,
    dir: GameDir,
    start: Option<Start>,
    pub(super) stage: Stage,
    boot: VecDeque<Boot>,
    pub(super) script: Option<UiScript>,
    pub(super) paused: PauseFlag,
    /// Settings changed in the menus and not yet written to the config file.
    pub(super) changed: Partial,
    /// A transition just happened: no screen listens until every pad button is let go.
    pub(super) latch: bool,
    pub(super) console_was_open: bool,
    pub(super) quit: bool,
    /// Settings are written to the config file (not in screenshot runs).
    save: bool,
    /// The pad mask of the last frame, for the start press that pauses.
    pub(super) last_mask: u32,
}

impl Frontend {
    pub fn new(screens: Screens, dir: GameDir, start: Start, script: Option<UiScript>) -> Self {
        Self {
            screens,
            dir,
            start: Some(start),
            stage: Stage::Menus,
            boot: VecDeque::new(),
            script,
            paused: PauseFlag::default(),
            changed: Partial::default(),
            latch: true,
            console_was_open: false,
            quit: false,
            save: true,
            last_mask: 0,
        }
    }

    /// The first thing on screen, once the renderer exists.
    pub(super) fn begin(&mut self, host: &mut Host, env: &mut Env) {
        let Some(start) = self.start.take() else { return };
        self.save = host.screenshot.is_none();
        match start {
            Start::Boot => {
                self.boot = BOOT.into();
                self.next_boot(host, env);
            }
            Start::Menu => self.show_menu(host, env),
            Start::Screen(name, args) => {
                self.replace_scene(host, Box::new(MenuScene));
                let commands = self.screens.open(&name, args, true, env);
                self.apply(commands, host, env);
                self.stage = Stage::Menus;
            }
            Start::Drive => self.start_free_roam(host),
        }
    }

    fn replace_scene(&mut self, host: &mut Host, scene: Box<dyn crate::viewer::Scene>) -> bool {
        match host.replace_scene(scene) {
            Ok(()) => true,
            Err(e) => {
                log::error!("cannot change the scene: {e:#}");
                false
            }
        }
    }

    /// Goes on in the boot flow; after the last step the main menu.
    pub(super) fn next_boot(&mut self, host: &mut Host, env: &mut Env) {
        self.latch = true;
        self.screens.clear();
        let Some(step) = self.boot.pop_front() else {
            self.show_menu(host, env);
            return;
        };
        self.stage = Stage::Boot(step);
        log::info!("boot: {step:?}");
        match step {
            Boot::Movie { name, .. } => {
                let scene = crate::movie::MovieScene::open(&self.dir, name, 0.0);
                match scene {
                    Ok(scene) => {
                        self.replace_scene(host, Box::new(scene));
                    }
                    Err(e) => {
                        log::warn!("the boot movie {name} is skipped: {e:#}");
                        self.next_boot(host, env);
                    }
                }
            }
            Boot::Splash => {
                self.replace_scene(host, Box::new(MenuScene));
                let commands = self.screens.open(SPLASH, Args::default(), true, env);
                self.apply(commands, host, env);
            }
        }
    }

    /// The main menu over the empty backdrop.
    pub(super) fn show_menu(&mut self, host: &mut Host, env: &mut Env) {
        log::info!("main menu");
        self.latch = true;
        self.paused.set(false);
        self.screens.clear();
        self.replace_scene(host, Box::new(MenuScene));
        let commands = self.screens.open(MAIN_MENU, Args::default(), true, env);
        self.stage = Stage::Menus;
        self.apply(commands, host, env);
    }

    fn start_free_roam(&mut self, host: &mut Host) {
        self.latch = true;
        self.screens.clear();
        let options = Options {
            at: None,
            height: 40.0,
            heading: 45.0,
            pitch: -20.0,
            fog_distance: 3000.0,
            wait_for_load: false,
            drive: Some(DriveOptions { car: DEFAULT_CAR.to_owned(), script: None }),
        };
        log::info!("starting free roam");
        let world = match WorldScene::open(&self.dir, options) {
            Ok(w) => w,
            Err(e) => {
                log::error!("cannot start free roam: {e:#}");
                return;
            }
        };
        self.paused.set(false);
        let scene = Pausable::new(Box::new(world), self.paused.clone());
        if self.replace_scene(host, Box::new(scene)) {
            self.stage = Stage::Playing;
        }
    }

    /// Opens the pause menu over the frozen game.
    pub(super) fn pause(&mut self, host: &mut Host, env: &mut Env) {
        log::info!("paused");
        self.latch = true;
        self.paused.set(true);
        self.stage = Stage::Paused;
        let commands = self.screens.open(PAUSE_MENU, Args { pause: true, ..Args::default() }, true, env);
        self.apply(commands, host, env);
    }

    fn resume(&mut self) {
        self.latch = true;
        self.screens.clear();
        self.paused.set(false);
        self.stage = Stage::Playing;
    }

    /// Carries out what the screens asked for.
    pub(super) fn apply(&mut self, commands: Vec<Command>, host: &mut Host, env: &mut Env) {
        for command in commands {
            match command {
                Command::StartFreeRoam => self.start_free_roam(host),
                Command::Resume => self.resume(),
                Command::QuitToMenu => self.show_menu(host, env),
                Command::QuitGame => self.quit = true,
                Command::NextBootStep => self.next_boot(host, env),
                Command::SaveSettings => self.save_settings(),
                Command::Switch(..) | Command::Push(..) | Command::Pop => {}
            }
        }
    }

    fn save_settings(&mut self) {
        if !self.save || self.changed == Partial::default() {
            return;
        }
        match Settings::save(&self.changed) {
            Ok(path) => {
                log::info!("settings saved to {}", path.display());
                self.changed = Partial::default();
            }
            Err(e) => log::warn!("the settings could not be saved: {e:#}"),
        }
    }
}
