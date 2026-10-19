//! The game flow: boot movies, the title screen, the menus, free roam and the pause menu
//! (docs/specs/frontend-menus.md, section 4).

use std::collections::VecDeque;

use bevy_ecs::prelude::*;
use game_install::GameDir;

use super::browse::Browse;
use super::loading;
use super::logic::{Args, Command, Env, ScreenLogic};
use super::scene::{ConcealFlag, MenuScene, Pausable, PauseFlag};
use super::screens::Screens;
use super::script::UiScript;
use crate::app::Host;
use crate::career::{CareerHandle, ProfileDir};
use crate::input::Control;
use crate::scenes::world::{DEFAULT_CAR, DriveOptions, Options, WorldScene};
use crate::settings::{Partial, Settings};

// Career on the main menu: starting or resuming a career (`start_career`).
#[path = "flow_career.rs"]
mod flow_career;
#[path = "flow_save.rs"]
mod flow_save;

/// Where the front end starts.
#[derive(Clone, Debug, PartialEq)]
pub enum Start {
    /// The whole boot flow: movies, the title screen, the main menu.
    Boot,
    /// The main menu.
    Menu,
    /// One screen, as `view-screen` shows it.
    Screen(String, Args),
    /// One screen over the scene already in the window, which stays (`feng-viewer`). Later screens come from the
    /// [`Browse`] handle the front end was given.
    Browse(String),
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
    /// Free roam is starting: the world streams behind the loading screen until the scene is ready
    /// (`loading.rs`, docs/specs/loading-screen.md).
    Loading,
    Playing,
    Paused,
    /// The world map is open over the frozen game (`world_map/control.rs`).
    Map,
}

use super::confirm::{Alert, AlertLogic};
use super::ids::screen::{DIALOG, INFRACTIONS, LOADING, MAIN_MENU, PAUSE_MENU, SPLASH, SPLASH_WIDE, WORLD_MAP};
use super::infractions::{InfractionsLogic, Ticket};
use crate::ui::Text;

/// Windows wider than this (width over height) get the widescreen title screen; 4:3 is 1.33.
const WIDE_ASPECT: f32 = 1.4;

/// How free roam starts: the stock car on the start of the track, and the world waits for its load so the
/// loading screen covers it (`wait_for_load`).
pub(super) fn free_roam_options() -> Options {
    Options {
        at: None,
        height: 40.0,
        heading: 45.0,
        pitch: -20.0,
        fog_distance: 3000.0,
        wait_for_load: true,
        drive: Some(DriveOptions { car: DEFAULT_CAR.to_owned(), script: None }),
    }
}

#[derive(Resource)]
pub struct Frontend {
    pub(super) screens: Screens,
    pub(super) dir: GameDir,
    start: Option<Start>,
    pub(super) stage: Stage,
    boot: VecDeque<Boot>,
    pub(super) script: Option<UiScript>,
    pub(super) paused: PauseFlag,
    /// Hides the world while the loading screen is up (`loading.rs`).
    pub(super) conceal: ConcealFlag,
    /// Settings changed in the menus and not yet written to the config file.
    pub(super) changed: Partial,
    /// A transition just happened: no screen listens until every pad button is let go.
    pub(super) latch: bool,
    pub(super) console_was_open: bool,
    pub(super) quit: bool,
    pub(super) graphics_requested: bool,
    pub(super) graphics_visible: bool,
    /// Settings are written to the config file (not in screenshot runs).
    save: bool,
    /// The pad mask of the last frame, for the start press that pauses.
    pub(super) last_mask: u32,
    /// A slot waiting for its next key (the controls screen asked; `plugin.rs` takes the key).
    pub(super) remap: Option<(Control, usize)>,
    /// The sidebar's hand-over, when a viewer runs the front end over its own scene (`Start::Browse`).
    pub(super) browse: Option<Browse>,
    /// Whether the scene in the window is the menu backdrop. Its car is read from the install and uploaded once, when
    /// it is built, so the menus that come up next keep it instead of building it again. While free roam has the
    /// window the backdrop is parked in the host (`Host::parked_menu`) and restored from there.
    menu_in_window: bool,
    /// The career being played, empty outside career mode (quick race, free roam from the command line).
    pub(super) career: CareerHandle,
    /// The profile folder careers are saved in. `None` when there is no per-user folder. Screenshot runs use a temp
    /// folder instead (`begin`), so they never write the player's profiles.
    pub(super) profiles: Option<ProfileDir>,
    /// The safe house menu is up over the frozen world: leaving it restarts free roam (`flow_career.rs`).
    pub(super) in_safe_house: bool,
    /// An auto shop was engaged in career mode: the update system asks the garage to open (`poll_career`).
    pub(in crate::frontend) auto_shop_requested: bool,
}

impl Frontend {
    pub fn new(screens: Screens, dir: GameDir, start: Start, script: Option<UiScript>) -> Self {
        // `run_front_end` starts over a built `MenuScene` (`app/render`); a viewer starts over its own scene.
        let menu_in_window = !matches!(start, Start::Browse(_));
        Self {
            screens,
            dir,
            start: Some(start),
            stage: Stage::Menus,
            boot: VecDeque::new(),
            script,
            paused: PauseFlag::default(),
            conceal: ConcealFlag::default(),
            changed: Partial::default(),
            latch: true,
            console_was_open: false,
            quit: false,
            graphics_requested: false,
            graphics_visible: false,
            save: true,
            last_mask: 0,
            remap: None,
            browse: None,
            menu_in_window,
            career: CareerHandle::default(),
            profiles: ProfileDir::default_dir(),
            in_safe_house: false,
            auto_shop_requested: false,
        }
    }

    /// The first thing on screen, once the renderer exists.
    pub(super) fn begin(&mut self, host: &mut Host, env: &mut Env) {
        let Some(start) = self.start.take() else { return };
        self.save = host.screenshot.is_none();
        if host.screenshot.is_some() {
            self.profiles = Some(ProfileDir::new(std::env::temp_dir().join("nfsmw-career-screenshot")));
        }
        match start {
            Start::Boot => {
                self.boot = BOOT.into();
                self.next_boot(host, env);
            }
            Start::Menu => self.show_menu(host, env),
            Start::Screen(name, args) => {
                self.show_backdrop(host);
                let commands = self.screens.open(&name, args, true, env);
                self.apply(commands, host, env);
                self.stage = Stage::Menus;
            }
            Start::Browse(name) => self.show_browsed(&name, host, env),
            Start::Drive => self.start_free_roam(host, env),
        }
    }

    /// Shows `name` in place of the screens there are, over the scene already in the window (which stays).
    pub(super) fn show_browsed(&mut self, name: &str, host: &mut Host, env: &mut Env) {
        self.latch = true;
        self.screens.clear();
        let commands = self.screens.open(name, Args::default(), true, env);
        self.stage = Stage::Menus;
        self.apply(commands, host, env);
    }

    /// Puts `scene` in the window. With `park`, a menu backdrop being replaced is kept for the menus that come back
    /// (`Host::replace_scene_parking_old`); any other scene is released.
    fn replace_scene(&mut self, host: &mut Host, scene: Box<dyn crate::viewer::Scene>, park: bool) -> bool {
        let parks_menu = park && self.menu_in_window;
        let result = match parks_menu {
            true => host.replace_scene_parking_old(scene),
            false => host.replace_scene(scene),
        };
        match result {
            Ok(()) => {
                self.menu_in_window = false;
                true
            }
            Err(e) => {
                log::error!("cannot change the scene: {e:#}");
                false
            }
        }
    }

    /// Puts the menu backdrop in the window. A backdrop already there is kept, and so is one parked while free roam
    /// had the window: building one reads the car from the install and uploads it (`MenuScene::init`, ~150-190 ms),
    /// which the menus that follow would only repeat.
    fn show_backdrop(&mut self, host: &mut Host) {
        if self.menu_in_window {
            return;
        }
        if host.restore_parked_menu() {
            self.menu_in_window = true;
            return;
        }
        if !self.replace_scene(host, Box::new(MenuScene::new(Some(self.dir.clone()))), false) {
            return;
        }
        self.menu_in_window = true;
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
                        self.replace_scene(host, Box::new(scene), false);
                    }
                    Err(e) => {
                        log::warn!("the boot movie {name} is skipped: {e:#}");
                        self.next_boot(host, env);
                    }
                }
            }
            Boot::Splash => {
                self.show_backdrop(host);
                // Like the original (`eIsWidescreen`): the wide package for any window wider than 4:3.
                let wide = host.renderer.as_ref().is_none_or(|r| r.aspect_ratio() > WIDE_ASPECT);
                let commands = self.screens.open(if wide { SPLASH_WIDE } else { SPLASH }, Args::default(), true, env);
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
        self.show_backdrop(host);
        let commands = self.screens.open(MAIN_MENU, Args::default(), true, env);
        self.stage = Stage::Menus;
        self.apply(commands, host, env);
    }

    /// Starts free roam behind the loading screen: the world is built and streams, but is not shown or heard until
    /// it reports ready (`loading_frame`).
    fn start_free_roam(&mut self, host: &mut Host, env: &mut Env) {
        self.latch = true;
        self.screens.clear();
        log::info!("starting free roam");
        let mut world = match WorldScene::open(&self.dir, free_roam_options()) {
            Ok(w) => w,
            Err(e) => {
                log::error!("cannot start free roam: {e:#}");
                return;
            }
        };
        world.set_career(self.career.clone(), self.profiles.clone());
        self.paused.set(false);
        self.conceal.set(true);
        let scene = Pausable::new(Box::new(world), self.paused.clone()).with_conceal(self.conceal.clone());
        // The menu backdrop is parked, not freed: the menus that come back after the race reuse it.
        if !self.replace_scene(host, Box::new(scene), true) {
            return;
        }
        self.stage = Stage::Loading;
        let commands = self.screens.open(LOADING, Args::default(), true, env);
        self.apply(commands, host, env);
    }

    /// One frame of the loading stage: the loading screen animates, and once the scene reports ready the world is
    /// revealed and the stage goes on to driving.
    pub(super) fn loading_frame(&mut self, dt: f32, host: &mut Host, env: &mut Env) {
        let commands = self.screens.update(dt, 0, env);
        self.apply(commands, host, env);
        if !loading::step(&self.conceal, host.scene.ready()) {
            return;
        }
        self.latch = true;
        self.screens.clear();
        self.stage = Stage::Playing;
        log::info!("loaded: free roam starts");
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

    /// Opens the bust's ticket when the world's bust sequence reaches it (free roam, driving). The ticket is up over the
    /// frozen game, as the pause menu is: the stage is `Paused`, so the screens take the pad and the mouse, and the world
    /// does not run until the payment resumes it.
    pub(super) fn poll_bust(&mut self, host: &mut Host, env: &mut Env) {
        if self.stage != Stage::Playing {
            return;
        }
        let Some(lines) = host.scene.take_bust_ticket() else { return };
        log::info!("busted: the ticket is up ({} infractions)", lines.len());
        self.latch = true;
        self.paused.set(true);
        self.stage = Stage::Paused;
        let career_ticket = self.career.with(|career| Ticket::with_career(&lines, career));
        let ticket = career_ticket.unwrap_or(Ticket { lines, ..Ticket::default() });
        let commands = self.screens.open_with(INFRACTIONS, Box::new(InfractionsLogic::new(ticket)), true, env);
        self.apply(commands, host, env);
    }

    /// The payment on the ticket: the bust takes it (and the game goes on driving) or refuses it (the ticket stays).
    fn pay_bust(&mut self, marker: bool, host: &mut Host) {
        if !host.scene.pay_bust(marker) {
            return;
        }
        log::info!("busted: paid with {}", if marker { "a marker" } else { "cash" });
        self.resume();
    }

    /// Opens the world map over the frozen game: its screen is the package `WorldMapMain.fng`, with the map's logic.
    pub(super) fn open_map(&mut self, logic: Box<dyn ScreenLogic>, host: &mut Host, env: &mut Env) {
        self.latch = true;
        self.paused.set(true);
        self.stage = Stage::Map;
        let commands = self.screens.open_with(WORLD_MAP, logic, true, env);
        self.apply(commands, host, env);
    }

    /// Saves the settings the menus changed, leaves the safe house and goes on driving.
    fn resume_driving(&mut self, env: &mut Env) {
        self.save_settings(env.changed);
        self.leave_safe_house();
        self.resume();
    }

    pub(super) fn resume(&mut self) {
        self.latch = true;
        self.screens.clear();
        self.paused.set(false);
        self.stage = Stage::Playing;
    }

    /// Carries out what the screens asked for.
    pub(super) fn apply(&mut self, commands: Vec<Command>, host: &mut Host, env: &mut Env) {
        for command in commands {
            match command {
                Command::StartFreeRoam => {
                    self.career.clear();
                    self.start_free_roam(host, env);
                }
                Command::StartCareer => self.start_career(host, env),
                Command::Resume => self.resume_driving(env),
                Command::SetWaypoint(target) => {
                    host.scene.set_waypoint(Some(target));
                    self.resume_driving(env);
                }
                Command::QuitToMenu => {
                    self.save_settings(env.changed);
                    self.show_menu(host, env);
                }
                Command::QuitGame => {
                    self.save_settings(env.changed);
                    self.quit = true;
                }
                Command::NextBootStep => self.next_boot(host, env),
                Command::CareerSave => self.career_save(host, env),
                Command::CareerLoad(alias) => self.career_load(&alias, host, env),
                Command::CareerNew(alias) => self.career_new(&alias, host, env),
                Command::CareerDelete(alias) => self.career_delete(&alias, host, env),
                Command::CarLot(action) => self.car_lot_action(action, host, env),
                Command::SaveSettings => self.save_settings(env.changed),
                Command::OpenGraphics => self.graphics_requested = true,
                Command::Remap(control, slot) => {
                    log::info!("remap: waiting for a key for {control:?} slot {slot}");
                    self.remap = Some((control, slot));
                }
                Command::SaveAlias => self.save_alias(host, env),
                Command::LoadAlias => self.load_alias(host, env),
                Command::PayBust { marker } => self.pay_bust(marker, host),
                Command::Switch(..) | Command::Push(..) | Command::Pop | Command::Confirm(_) | Command::Alert(_) => {}
            }
        }
    }

    /// Loads the latest alias's settings (`saves::load_latest_alias`) and makes them the current settings. The loaded
    /// values are marked changed, so they go to the config file as any change does. Shows the outcome in an alert.
    fn load_alias(&mut self, host: &mut Host, env: &mut Env) {
        let message = match crate::saves::store().map(|store| crate::saves::load_latest_alias(&store)) {
            None => "There is no per-user folder to load from.".to_owned(),
            Some(Ok(Some((name, settings)))) => {
                *env.settings = settings;
                *env.changed = crate::saves::partial_of(env.settings);
                format!("{name} loaded.")
            }
            Some(Ok(None)) => "No alias is saved yet.".to_owned(),
            Some(Err(e)) => {
                log::warn!("the alias could not be loaded: {e}");
                format!("The alias could not be loaded: {e}")
            }
        };
        let alert = AlertLogic::new(Alert::new(Text::Custom(message)));
        let commands = self.screens.open_with(DIALOG, Box::new(alert), false, env);
        self.apply(commands, host, env);
    }

    /// Saves the current settings as a new named alias (`saves::save_new_alias`) and shows what happened in an alert.
    fn save_alias(&mut self, host: &mut Host, env: &mut Env) {
        let settings = *env.settings;
        let message = match crate::saves::store().map(|store| crate::saves::save_new_alias(&store, &settings)) {
            Some(Ok(name)) => format!("{name} saved."),
            Some(Err(e)) => {
                log::warn!("the alias could not be saved: {e}");
                format!("The alias could not be saved: {e}")
            }
            None => "There is no per-user folder to save to.".to_owned(),
        };
        let alert = AlertLogic::new(Alert::new(Text::Custom(message)));
        let commands = self.screens.open_with(DIALOG, Box::new(alert), false, env);
        self.apply(commands, host, env);
    }

    pub(super) fn save_settings(&self, changed: &mut Partial) {
        if !self.save || *changed == Partial::default() {
            return;
        }
        match Settings::save(changed) {
            Ok(path) => {
                log::info!("settings saved to {}", path.display());
                *changed = Partial::default();
            }
            Err(e) => log::warn!("the settings could not be saved: {e:#}"),
        }
    }
}
