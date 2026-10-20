//! The game flow: boot movies, the title screen, the menus, free roam and the pause menu
//! (docs/specs/frontend-menus.md, section 4).

use std::collections::VecDeque;

use bevy_ecs::prelude::*;
use game_install::GameDir;

use flow_loading::{Load, LoadingScreen};

use super::browse::Browse;
use super::career_screens::save::{ListMode, list_aliases, set_list_mode};
use super::loading;
use super::logic::{Args, Command, Env, PauseCtx, QuitTo, ScreenLogic};
use super::look::Look;
use super::scene::{ConcealFlag, MenuScene, Pausable, PauseFlag};
use super::screens::Screens;
use super::script::UiScript;
use crate::app::Host;
use crate::career::{CareerHandle, ProfileDir};
use crate::input::Control;
use crate::scenes::world::{DEFAULT_CAR, DriveOptions, Options};
use crate::settings::{Partial, Settings};

// Career on the main menu: starting or resuming a career (`start_career`).
#[path = "flow_career.rs"]
mod flow_career;
#[path = "flow_loading.rs"]
mod flow_loading;
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
    /// Where the boot flow ends: the list of saved aliases, or the main menu when there are none.
    Aliases,
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

use super::ids::screen::{INFRACTIONS, LOADING, LOADING_TIPS, MAIN_MENU, PAUSE_MENU, SPLASH, SPLASH_WIDE, WORLD_MAP};
use super::infractions::{InfractionsLogic, Ticket};

/// Windows wider than this (width over height) get the widescreen title screen; 4:3 is 1.33.
const WIDE_ASPECT: f32 = 1.4;

/// How free roam starts: the stock car on the start of the track, and the world waits for its load so the
/// loading screen covers it (`wait_for_load`).
pub(super) fn free_roam_options(car: &str) -> Options {
    Options {
        at: None,
        height: 40.0,
        heading: 45.0,
        pitch: -20.0,
        fog_distance: 3000.0,
        wait_for_load: true,
        drive: Some(DriveOptions { car: car.to_owned(), script: None }),
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
    /// Where the start of free roam is while the loading screen is up (`flow_loading.rs`).
    load: Option<Load>,
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
    /// The safe house menu stands before the world exists (Resume Career): Resume Free Roam starts the world and Back goes
    /// to the career menu (`flow_career.rs`).
    pub(super) prelude: bool,
    /// The menu the game in progress was started from: where Quit goes and what the item says (`pause`).
    pub(super) origin: QuitTo,
    /// What the menu backdrop shows, in the window or parked while the world has it (`sync_backdrop`).
    look: Look,
    /// An auto shop was engaged in career mode: the update system asks the garage to open (`poll_career`).
    pub(in crate::frontend) auto_shop_requested: bool,
    /// The race a quick race asked for: the world starts it as soon as it stands (`start_free_roam`).
    quick_race: Option<String>,
    /// The car Quick Play picked for the world to drive (`start_free_roam`).
    quick_car: Option<String>,
    /// How the active theme dressed the map the last frame it was open (`plugin.rs`), for the system pointer.
    pub(in crate::frontend) map_chrome: super::world_map::Chrome,
}

impl Frontend {
    pub fn new(screens: Screens, dir: GameDir, start: Start, script: Option<UiScript>) -> Self {
        // `run_front_end` starts over a built `MenuScene` (`app/render`); a viewer starts over its own scene.
        let menu_in_window = !matches!(start, Start::Browse(_));
        super::quick_race::load_in_background(&dir);
        Self {
            screens,
            dir,
            start: Some(start),
            stage: Stage::Menus,
            boot: VecDeque::new(),
            script,
            paused: PauseFlag::default(),
            conceal: ConcealFlag::default(),
            load: None,
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
            prelude: false,
            origin: QuitTo::MainMenu,
            look: Look::Main,
            auto_shop_requested: false,
            quick_race: None,
            quick_car: None,
            map_chrome: Default::default(),
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
            Start::Aliases => self.after_boot(host, env),
            Start::Screen(name, args) => {
                self.show_backdrop(host);
                let commands = self.screens.open(&name, args, true, env);
                self.apply(commands, host, env);
                self.stage = Stage::Menus;
            }
            Start::Browse(name) => self.show_browsed(&name, host, env),
            Start::Drive => self.start_free_roam(LoadingScreen::Plain, host, env),
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
        if !self.replace_scene(host, Box::new(MenuScene::with_look(Some(self.dir.clone()), self.look.clone())), false) {
            return;
        }
        self.menu_in_window = true;
    }

    /// Goes on in the boot flow; after the last step the main menu.
    pub(super) fn next_boot(&mut self, host: &mut Host, env: &mut Env) {
        self.latch = true;
        self.screens.clear();
        let Some(step) = self.boot.pop_front() else {
            self.after_boot(host, env);
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
        self.show_menu_at(MAIN_MENU, Args::default(), host, env);
    }

    /// A menu screen over the backdrop it wants (`look_for`), with nothing else up and no game running.
    pub(super) fn show_menu_at(&mut self, name: &str, args: Args, host: &mut Host, env: &mut Env) {
        self.latch = true;
        self.paused.set(false);
        self.screens.clear();
        self.career.end_play();
        self.prelude = false;
        self.in_safe_house = false;
        self.open_menu(name, args, host, env);
    }

    /// Opens a menu screen over the backdrop it wants, with the menu stage on.
    pub(super) fn open_menu(&mut self, name: &str, args: Args, host: &mut Host, env: &mut Env) {
        if let Some(look) = super::factory::look_for(name, &args) {
            self.look = look;
        }
        self.show_backdrop(host);
        let commands = self.screens.open(name, args, true, env);
        self.stage = Stage::Menus;
        self.apply(commands, host, env);
    }

    /// Swaps the backdrop when the screen on top wants another look (`look.rs`). Nothing happens while the world has the
    /// window, or a viewer runs the front end over its own scene.
    pub(super) fn sync_backdrop(&mut self, host: &mut Host) {
        if self.stage != Stage::Menus || !self.menu_in_window {
            return;
        }
        let Some(look) = self.screens.look().filter(|look| *look != self.look) else { return };
        log::info!("menu backdrop: {look:?}");
        self.look = look;
        let scene = MenuScene::with_look(Some(self.dir.clone()), self.look.clone());
        self.replace_scene(host, Box::new(scene), false);
        self.menu_in_window = true;
    }

    /// After the boot flow: the list of saved aliases to load one from, or the main menu when there are none
    /// (`docs/specs/career-parity-menus.md`, section 2).
    fn after_boot(&mut self, host: &mut Host, env: &mut Env) {
        if self.career.is_loaded() || list_aliases().is_empty() {
            return self.show_menu(host, env);
        }
        log::info!("boot: the alias list");
        set_list_mode(ListMode::Load);
        self.latch = true;
        self.screens.clear();
        self.show_backdrop(host);
        let commands = self.screens.open(super::ids::screen::MC_LIST, Args::default(), true, env);
        self.stage = Stage::Menus;
        self.apply(commands, host, env);
    }

    /// Opens the pause menu over the frozen game.
    pub(super) fn pause(&mut self, host: &mut Host, env: &mut Env) {
        log::info!("paused");
        self.latch = true;
        self.paused.set(true);
        self.stage = Stage::Paused;
        let race = host.scene.race_active();
        let pause_ctx = PauseCtx { race, quit: self.origin };
        let commands = self.screens.open(PAUSE_MENU, Args { pause: true, pause_ctx, ..Args::default() }, true, env);
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
                    self.career.end_play();
                    self.origin = QuitTo::MainMenu;
                    self.start_free_roam(LoadingScreen::Plain, host, env);
                }
                Command::StartQuickRace(race) => {
                    self.career.end_play();
                    self.origin = QuitTo::QuickRaceMenu;
                    self.quick_race = Some(race);
                    self.start_free_roam(LoadingScreen::Plain, host, env);
                }
                Command::StartQuickPlay { race, car } => self.start_quick_play(race, car, host, env),
                Command::ResumeCareer => self.resume_career(host, env),
                Command::LeaveSafeHouse => self.leave_prelude(host, env),
                Command::RestartRace => self.restart_race(host),
                Command::Resume if self.prelude => self.start_from_safe_house(host, env),
                Command::Resume => self.resume_driving(env),
                Command::SetWaypoint(target) => {
                    host.scene.set_waypoint(Some(target));
                    self.resume_driving(env);
                }
                Command::QuitTo(to) => {
                    self.save_settings(env.changed);
                    self.quit_to(to, host, env);
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
                Command::AliasCreate(alias) => self.alias_create(&alias, host, env),
                Command::PayBust { marker } => self.pay_bust(marker, host),
                Command::RaceAnswer(answer) => self.race_answer(answer, host),
                Command::ShowResults(report) => self.show_results(report, host, env),
                Command::Switch(..)
                | Command::Push(..)
                | Command::Pop
                | Command::Confirm(_)
                | Command::Alert(_)
                | Command::EnterName(_) => {}
            }
        }
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
