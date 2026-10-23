//! Starting free roam behind a loading screen (docs/specs/loading-screen.md). The screen is up at once: the world is
//! built on a thread (the install's tables, the physics, the car, the streaming index take seconds) while the screen
//! animates, then it is put in the window concealed and streams until it reports ready, and the front end reveals it.

use std::thread::JoinHandle;

use anyhow::Result;

use super::*;
use crate::scenes::world::{DriveOptions, Options, WorldLoad, WorldScene};

/// How free roam starts: the stock car on the start of the track, and the world waits for its load so the
/// loading screen covers it (`wait_for_load`).
pub(in crate::frontend) fn free_roam_options(car: &str) -> Options {
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

/// The loading screen a start shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum LoadingScreen {
    /// `WS_Loading.fng`: free roam and Quick Race.
    Plain,
    /// `Loading_Tips.fng`: the career, with a tip and its picture.
    Tips,
}

impl LoadingScreen {
    pub(super) fn file(self) -> &'static str {
        match self {
            Self::Plain => LOADING,
            Self::Tips => LOADING_TIPS,
        }
    }
}

/// Where the world starts when it replaces the safe house menu that stood before it (Resume Career, the first visit): at the
/// marker of the last safe house the player came to, else at the career's start point. The zone is not kept across runs.
#[derive(Default)]
pub(super) struct SafeHouseStart {
    /// The last safe house zone the player came to in this run.
    pub(super) zone: Option<String>,
    /// The next world starts there (`start_from_safe_house`).
    pub(super) pending: bool,
}

/// Where a start is. The state is the front end's; the world is the scene's.
pub(super) enum Load {
    /// The world is being built on a thread.
    Building(JoinHandle<Result<WorldLoad>>),
    /// The world is in the window, concealed, and streaming.
    Streaming,
}

impl Frontend {
    /// Starts free roam behind `screen`: the screen opens now, the world is built on a thread and is not shown or heard
    /// until it reports ready (`loading_frame`).
    pub(super) fn start_free_roam(&mut self, screen: LoadingScreen, host: &mut Host, env: &mut Env) {
        self.latch = true;
        self.screens.clear();
        log::info!("starting free roam");
        let dir = self.dir.clone();
        let car = self.quick_car.take().unwrap_or_else(|| DEFAULT_CAR.to_owned());
        let spawned = std::thread::Builder::new()
            .name("world-load".to_owned())
            .spawn(move || WorldScene::load(&dir, free_roam_options(&car)));
        let world = match spawned {
            Ok(world) => world,
            Err(e) => {
                log::error!("cannot start free roam: {e}");
                return;
            }
        };
        self.load = Some(Load::Building(world));
        self.stage = Stage::Loading;
        let commands = self.screens.open(screen.file(), Args::default(), true, env);
        self.apply(commands, host, env);
    }

    /// One frame of the loading stage: the screen animates, and the start goes on a step when its state allows. Nothing
    /// pauses or skips it, so the pad is not read.
    pub(in crate::frontend) fn loading_frame(&mut self, dt: f32, host: &mut Host, env: &mut Env) {
        let commands = self.screens.update(dt, 0, env);
        self.apply(commands, host, env);
        let Some(load) = self.load.take() else { return self.reveal() };
        match load {
            Load::Building(world) => {
                if !world.is_finished() {
                    self.load = Some(Load::Building(world));
                    return;
                }
                self.world_built(world, host, env);
            }
            Load::Streaming => {
                // A race asked for is part of the start: the screen stays up until the car stands on its grid.
                if !loading::start_over(host.scene.ready(), host.scene.race_staging()) {
                    self.load = Some(Load::Streaming);
                    return;
                }
                self.reveal();
            }
        }
    }

    /// The world is built: it goes in the window, concealed, to stream. A world that could not be built takes the
    /// player back to the main menu.
    fn world_built(&mut self, world: JoinHandle<Result<WorldLoad>>, host: &mut Host, env: &mut Env) {
        let mut world = match world.join() {
            Ok(Ok(load)) => load.build(),
            Ok(Err(e)) => return self.loading_failed(&format!("{e:#}"), host, env),
            Err(_) => return self.loading_failed("the loading thread panicked", host, env),
        };
        world.set_career(self.career.clone(), self.profiles.clone());
        if let Some(setup) = self.coop.as_ref().map(|session| session.setup.clone()) {
            world.queue_coop(setup);
        }
        if let Some(race) = self.quick_race.take() {
            // The challenge series, Quick Play and a custom race set their options; a race started by hand runs as the data has it.
            let options = self.race_options.take().unwrap_or_default();
            match self.split.clone() {
                Some(setup) => world.queue_split_race(race, options, setup),
                None => world.queue_race(race, options),
            }
        }
        if std::mem::take(&mut self.safe_house_start.pending) {
            world.start_at_safe_house(self.safe_house_start.zone.as_deref());
        }
        if std::mem::take(&mut self.safe_house_start.pending) {
            world.start_at_safe_house(self.safe_house_start.zone.as_deref());
        }
        if std::mem::take(&mut self.drive_to_safe_house) {
            world.start_drive_to_safe_house();
        }
        self.paused.set(false);
        self.conceal.set(true);
        let scene = Pausable::new(Box::new(world), self.paused.clone()).with_conceal(self.conceal.clone());
        // The menu backdrop is parked, not freed: the menus that come back after the race reuse it.
        if !self.replace_scene(host, Box::new(scene), true) {
            return self.loading_failed("the world could not be put in the window", host, env);
        }
        self.load = Some(Load::Streaming);
    }

    fn loading_failed(&mut self, reason: &str, host: &mut Host, env: &mut Env) {
        log::error!("cannot start free roam: {reason}");
        self.load = None;
        self.show_menu(host, env);
    }

    /// Loading is over: the world is shown and the stage goes on to driving.
    fn reveal(&mut self) {
        loading::step(&self.conceal, true);
        self.latch = true;
        self.screens.clear();
        self.stage = Stage::Playing;
        log::info!("loaded: free roam starts");
    }
}
