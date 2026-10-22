//! The game flow: boot movies, the title screen, the menus, free roam and the pause menu
//! (docs/specs/frontend-menus.md, section 4).

use std::collections::VecDeque;

use bevy_ecs::prelude::*;
use game_install::GameDir;

use flow_loading::{Load, LoadingScreen};

use super::browse::Browse;
use super::camera_rig::CameraRig;
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
use nfsmw_career::rapsheet::EndGame;

#[path = "flow_backdrop.rs"]
mod flow_backdrop;
// Career on the main menu: starting or resuming a career (`start_career`).
#[path = "flow_arrival.rs"]
mod flow_arrival;
#[path = "flow_bust.rs"]
mod flow_bust;
#[path = "flow_career.rs"]
mod flow_career;
#[path = "flow_coop.rs"]
mod flow_coop;
#[path = "flow_cover.rs"]
mod flow_cover;
#[path = "flow_custom.rs"]
mod flow_custom;
#[path = "flow_endgame.rs"]
mod flow_endgame;
#[path = "flow_first_visit.rs"]
pub(in crate::frontend) mod flow_first_visit;
#[path = "flow_loading.rs"]
mod flow_loading;
#[path = "flow_movie.rs"]
pub(super) mod flow_movie;
#[path = "flow_prologue.rs"]
mod flow_prologue;
#[path = "flow_rival.rs"]
mod flow_rival;
#[path = "flow_save.rs"]
mod flow_save;
#[path = "flow_series.rs"]
pub(super) mod flow_series;
#[path = "flow_sms.rs"]
mod flow_sms;
#[path = "flow_split.rs"]
mod flow_split;
#[path = "flow_startup.rs"]
mod flow_startup;

/// Where the front end starts.
#[derive(Clone, Debug, PartialEq)]
pub enum Start {
    /// The whole boot flow: movies (when `movies`), the title screen, the profile step, the main menu.
    Boot { movies: bool },
    /// The main menu.
    Menu,
    /// One screen, as `view-screen` shows it.
    Screen(String, Args),
    /// One screen over the scene already in the window, which stays (`feng-viewer`). Later screens come from the
    /// [`Browse`] handle the front end was given.
    Browse(String),
    /// Straight into free roam.
    Drive,
    /// Where the boot flow ends: the profile step (the list of saved aliases, or the question to create one when there is
    /// none), then the main menu. The explicit way to run that step without the movies and the title screen.
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

#[derive(Clone, Debug, PartialEq)]
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
    /// A movie (a tutorial or a story movie) plays full screen over the menus, then `then` says what the flow does
    /// (`flow_movie.rs`).
    Movie {
        then: flow_movie::AfterMovie,
    },
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
    /// The profile step of the boot flow is running: when its last screen closes the main menu follows (`poll_startup`).
    startup: bool,
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
    /// The menu the game in progress was started from: where Quit goes and what the item says (`pause`).
    pub(super) origin: QuitTo,
    /// The end-game chain is running: its next page opens once the page it opened last is no longer on top (`flow_endgame.rs`).
    end_game: Option<EndGame>,
    /// The screen the end-game chain opened last.
    end_game_shown: Option<String>,
    /// What the menu backdrop shows, in the window or parked while the world has it (`sync_backdrop`).
    pub(super) look: Look,
    /// A look the command line fixed (`view-screen --look`): the screens do not change it.
    pub(super) look_override: Option<Look>,
    /// The camera of the menu backdrops, which the screens move (`camera_rig.rs`).
    pub(super) camera: CameraRig,
    /// An auto shop was engaged in career mode: the update system asks the garage to open (`poll_career`).
    pub(in crate::frontend) auto_shop_requested: bool,
    /// The race a quick race asked for: the world starts it as soon as it stands (`start_free_roam`).
    quick_race: Option<String>,
    pub(super) series: flow_series::Series,
    /// The car Quick Play picked for the world to drive (`start_free_roam`).
    quick_car: Option<String>,
    /// What the race above runs with: the challenge series, Quick Play and a custom race set it; the world takes it with the race
    /// (`world_built`).
    race_options: Option<crate::scenes::world::RaceOptions>,
    /// The two players of the split-screen race in progress (`flow_split.rs`); the world takes it with the race.
    split: Option<crate::scenes::world::SplitSetup>,
    /// The co-op session in progress: player 2 and both players' devices (`flow_coop.rs`); the world takes player 2 when built.
    coop: Option<flow_coop::CoopSession>,
    /// The world starts the drive to the first safe house once it is built (`flow_loading.rs`, `flow_prologue.rs`).
    drive_to_safe_house: bool,
    /// The race on the grid was started again from the pause menu or the results: it stays on its start, so no loading screen
    /// covers it (`flow_cover.rs`).
    pub(super) restarting: bool,
    /// Where the next world puts the car after the safe house menu that stood before it (`flow_loading.rs`).
    safe_house_start: flow_loading::SafeHouseStart,
    /// How the active theme dressed the map the last frame it was open (`plugin.rs`), for the system pointer.
    pub(in crate::frontend) map_chrome: super::world_map::Chrome,
}

impl Frontend {
    pub fn new(screens: Screens, dir: GameDir, start: Start, script: Option<UiScript>) -> Self {
        // `run_front_end` starts over a built `MenuScene` (`app/render`); a viewer starts over its own scene.
        let menu_in_window = !matches!(start, Start::Browse(_));
        super::quick_race::load_in_background(&dir);
        super::challenge_series::load_in_background(&dir);
        super::quick_play::load_in_background(&dir);
        Self {
            screens,
            dir,
            start: Some(start),
            stage: Stage::Menus,
            boot: VecDeque::new(),
            startup: false,
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
            origin: QuitTo::Main,
            look: Look::Main,
            look_override: None,
            camera: CameraRig::default(),
            auto_shop_requested: false,
            end_game: None,
            end_game_shown: None,
            quick_race: None,
            series: Default::default(),
            quick_car: None,
            race_options: None,
            split: None,
            coop: None,
            drive_to_safe_house: false,
            restarting: false,
            safe_house_start: Default::default(),
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
            Start::Boot { movies } => {
                self.boot = flow_startup::boot_steps(movies);
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
        self.startup = false;
        self.screens.clear();
        self.career.end_play();
        self.in_safe_house = false;
        self.open_menu(name, args, host, env);
    }

    /// Opens a menu screen over the backdrop it wants, with the menu stage on.
    pub(super) fn open_menu(&mut self, name: &str, args: Args, host: &mut Host, env: &mut Env) {
        self.show_backdrop(host);
        let commands = self.screens.open(name, args, true, env);
        self.stage = Stage::Menus;
        // The screen may want another look than the backdrop that is up: the swap happens before the first frame.
        self.sync_backdrop(host);
        self.apply(commands, host, env);
    }

    /// Swaps the backdrop when the screen on top wants another look (`look.rs`). Nothing happens while the world has the
    /// window, or a viewer runs the front end over its own scene.
    pub(super) fn sync_backdrop(&mut self, host: &mut Host) {
        if self.stage != Stage::Menus || !self.menu_in_window {
            return;
        }
        let wanted = self.look_override.clone().or_else(|| self.screens.look());
        if let Some(look) = wanted.filter(|look| *look != self.look) {
            log::info!("menu backdrop: {look:?}");
            self.look = look;
            let scene = MenuScene::with_look(Some(self.dir.clone()), self.look.clone()).with_rig(self.camera.clone());
            self.replace_scene(host, Box::new(scene), false);
            self.menu_in_window = true;
        }
        // The screen on top moves the camera to its pose, also when it comes back to the top as the one above closes.
        if let Some(pose) = self.screens.pose() {
            self.camera.show(pose);
        }
    }

    /// Opens the pause menu over the frozen game.
    pub(super) fn pause(&mut self, host: &mut Host, env: &mut Env) {
        log::info!("paused");
        self.latch = true;
        self.paused.set(true);
        self.stage = Stage::Paused;
        let race = host.scene.race_active();
        let pause_ctx = PauseCtx { race, quit: self.origin, ..flow_coop::coop_pause(host) };
        let commands = self.screens.open(PAUSE_MENU, Args { pause: true, pause_ctx, ..Args::default() }, true, env);
        self.apply(commands, host, env);
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
                    self.origin = QuitTo::Main;
                    self.start_free_roam(LoadingScreen::Plain, host, env);
                }
                Command::StartChallenge { race, transmission } => self.start_challenge(race, transmission, host, env),
                Command::PlayTutorial { movie, back, args } => {
                    self.play_movie(&movie, flow_movie::AfterMovie::Open(back, args), host)
                }
                Command::PlayMovie { movie, then } => self.play_movie(&movie, then, host),
                Command::CustomDefaults => crate::frontend::custom_race::state::with(|state| state.reset_settings()),
                Command::StartCustomRace(start) => self.start_custom_race(*start, host, env),
                Command::SplitFirstCar(transmission) => self.split_first_car(transmission, host, env),
                Command::StartSplitRace { race, options, setup } => {
                    self.start_split_race(race, options, setup, host, env)
                }
                Command::CoopSecondCar { handle, transmission } => {
                    self.coop_second_car(handle, transmission, host, env)
                }
                Command::StartQuickPlay { race, car } => self.start_quick_play(race, car, host, env),
                Command::ResumeCareer => self.resume_career(host, env),
                Command::LeaveSafeHouse => self.leave_prelude(host, env),
                Command::ChallengeRival { bin } => self.challenge_rival(bin, host, env),
                Command::BlacklistStep(stepper) => self.blacklist_step(stepper, host, env),
                Command::StartFirstDrive => self.start_first_drive(host, env),
                Command::RestartRace => self.restart_race(host),
                Command::Resume if self.career.is_prelude() => self.start_from_safe_house(host, env),
                Command::Resume => self.resume_driving(env),
                Command::EndCoop => self.end_coop_from_pause(host, env),
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
                Command::CareerCar(action) => self.career_car_action(action, host, env),
                Command::SaveSettings => self.save_settings(env.changed),
                Command::OpenGraphics => self.graphics_requested = true,
                Command::Remap(control, slot) => {
                    log::info!("remap: waiting for a key for {control:?} slot {slot}");
                    self.remap = Some((control, slot));
                }
                Command::AliasCreate(alias) => self.alias_create(&alias, host, env),
                Command::PayBust { marker } => self.pay_bust(marker, host, env),
                Command::RaceAnswer(answer) => self.race_answer(answer, host, env),
                Command::ShowResults(report) => self.show_results(report, host, env),
                Command::ShowPhoto(report) => self.show_photo(report, host, env),
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
