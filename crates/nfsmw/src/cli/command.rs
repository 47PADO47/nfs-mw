//! The subcommands: what `nfsmw <command>` runs.

use clap::{Parser, Subcommand};

use super::Cli;
use super::parse::parse_xy;
use super::view_args::ViewArgs;

#[derive(Subcommand)]
pub enum Command {
    /// Find the install and check that it is usable.
    CheckInstall,
    /// First-run setup: asks for the install folder and window mode, then saves them to the config file.
    Setup,
    /// List every key, button and stick binding.
    #[command(visible_alias = "bindings")]
    Keys,
    /// List the cars in the install.
    ListCars,
    /// List the shared scenery models (poles, cones...) of the city, one name per line.
    ListAssets,
    /// Show a car assembled from its stock parts (drag to orbit, scroll to zoom, Esc to quit).
    ViewCar {
        /// Car folder name under CARS/, e.g. BMWM3GTR.
        #[arg(default_value = "BMWM3GTR")]
        car: String,
        /// Level of detail to show (A = highest ... E = lowest).
        #[arg(long, default_value = "A")]
        lod: char,
        /// Show every solid of the LOD at the origin (decals, damage, every kit) instead of the assembled car.
        #[arg(long)]
        all_parts: bool,
        /// Build a preset car from the game's PresetRides (e.g. CE_GTRSTREET) instead of the stock car.
        #[arg(long, value_name = "NAME")]
        preset: Option<String>,
        /// Camera yaw in degrees.
        #[arg(long, default_value_t = 35.0, allow_negative_numbers = true)]
        yaw: f32,
        #[command(flatten)]
        view: ViewArgs,
    },
    /// Browse every car in the install: a searchable list on the left, the chosen car on the right with its
    /// wheels turning and the front pair steering (Esc quits).
    AssetViewer {
        /// The car to start with: a folder name under CARS/, or a unique start of one.
        #[arg(default_value = "BMWM3GTR")]
        car: String,
        #[command(flatten)]
        view: ViewArgs,
    },
    /// Browse the shared scenery models (poles, cones, bins...): a searchable list on the left, the chosen model
    /// on the right (Esc quits).
    ViewAssets {
        /// The model to start with, or a unique start of its name; the first one when left out.
        name: Option<String>,
        #[command(flatten)]
        view: ViewArgs,
    },
    /// Play a movie from MOVIES/ full screen (Esc quits): `nfsmw play-movie ealogo`.
    PlayMovie {
        /// Movie name or a unique start of it, e.g. ealogo or blacklist_03.
        name: String,
        /// Start this many seconds in (the video before it is decoded and dropped; the sound still starts at 0).
        #[arg(long, default_value_t = 0.0)]
        start: f32,
        #[command(flatten)]
        view: ViewArgs,
    },
    /// List the movies in the install.
    ListMovies,
    /// Play (the default with no command): the boot movies, the title screen, the main menu, free roam (Esc or
    /// Start pauses) and the settings.
    Play {
        /// Start at the main menu instead of the boot movies, the title screen and the profile step.
        #[arg(long)]
        skip_boot: bool,
        /// Start driving at once (free roam with the stock car).
        #[arg(long, conflicts_with = "skip_boot")]
        drive: bool,
        /// Start at the profile step of the boot flow: the list of saved aliases, or the question to create one when none
        /// is saved, then the main menu (for tests and screenshots, with a --ui-script).
        #[arg(long, hide = true, conflicts_with_all = ["skip_boot", "drive"])]
        aliases: bool,
        /// A scripted pad for tests and screenshots, e.g. "wait 1;right;accept" (replaces the input and the clock).
        #[arg(long, hide = true, value_name = "SCRIPT")]
        ui_script: Option<String>,
        #[command(flatten)]
        view: ViewArgs,
    },
    /// Show one FEng screen on its own: `nfsmw view-screen MainMenu.fng --screenshot out.png`.
    ViewScreen {
        /// File name of the screen, in any case.
        name: String,
        /// Show it as the pause menu does (the in-game look of the option screens).
        #[arg(long)]
        pause: bool,
        /// For Pause_Main.fng and MainMenu_Sub.fng: the option categories.
        #[arg(long)]
        options: bool,
        /// For the option screens: audio, video, gameplay, player, controls or discord.
        #[arg(long, default_value = "audio")]
        category: String,
        /// A scripted pad for tests and screenshots (see play); with --screenshot the default is "wait 2".
        #[arg(long, value_name = "SCRIPT")]
        ui_script: Option<String>,
        /// Fix the 3D backdrop whatever the screen asks for, to check a look offscreen: main, car or safehouse.
        #[arg(long, hide = true, value_name = "LOOK")]
        look: Option<String>,
        /// For --look car or safehouse: the car folder under CARS/ and optionally a preset, as BMWM3GTRE46:M3GTRCAREERSTART.
        #[arg(long, hide = true, value_name = "CAR[:PRESET]", requires = "look")]
        car: Option<String>,
        #[command(flatten)]
        view: ViewArgs,
    },
    /// Browse every FEng screen in the install: a searchable list on the left, the chosen screen over a flat backdrop
    /// with the camera off (Esc quits). `nfsmw feng-viewer MainMenu.fng --screenshot out.png` shows one at once.
    FengViewer {
        /// The screen to start with, in any case; the first one when left out.
        name: Option<String>,
        #[command(flatten)]
        view: ViewArgs,
    },
    /// Browse every sound of the install: a searchable list on the left, and a mini player at the bottom (previous, next,
    /// pause or play, and a slider that follows the sound). Clicking a sound plays it; Space pauses, Left and Right step.
    /// `nfsmw sound-viewer FE_MB.abk` starts on that bank (Esc quits).
    SoundViewer {
        /// The bank to start on, with or without its folder, in any case; the first sound when left out.
        bank: Option<String>,
        #[command(flatten)]
        view: ViewArgs,
    },
    /// List the FEng screens (menus, in-game screens) in the install.
    ListScreens,
    /// Search the English language table: text containing FILTER, or one label as 0xHASH.
    Strings {
        /// Part of the text, or 0xHASH.
        filter: String,
    },
    /// Print the objects, scripts and message responses of one FEng screen: `nfsmw dump-screen MainMenu.fng`.
    DumpScreen {
        /// File name of the screen, in any case.
        name: String,
    },
    /// Fly through the city (WASD + mouse to look; Shift = fast; Esc frees the mouse, again to quit).
    ViewWorld {
        /// Start position on the map as X,Y (default: the centre of the city).
        #[arg(long, value_name = "X,Y", value_parser = parse_xy, allow_hyphen_values = true)]
        at: Option<[f32; 2]>,
        /// Start height above the ground, in metres.
        #[arg(long, default_value_t = 40.0)]
        height: f32,
        /// Initial heading in degrees (0 = +X).
        #[arg(long, default_value_t = 45.0, allow_negative_numbers = true)]
        heading: f32,
        /// Initial pitch in degrees (negative looks down).
        #[arg(long, default_value_t = -20.0, allow_negative_numbers = true)]
        pitch: f32,
        /// Distance in metres at which the fog is complete (it starts at half of it).
        #[arg(long, default_value_t = 3000.0)]
        fog_distance: f32,
        /// For --screenshot: wait until every tile of the camera's zone has loaded before capturing.
        #[arg(long)]
        wait_for_load: bool,
        /// Drive a car instead of flying: a car folder or unique prefix (default BMWM3GTR). The car
        /// is put on the road nearest to --at (WASD or arrows, Space handbrake, E/Q gears, Left Shift
        /// nitrous, R reset, F free camera; `nfsmw keys` lists them all).
        #[arg(long, value_name = "CAR", num_args = 0..=1, default_missing_value = "BMWM3GTR")]
        drive: Option<String>,
        /// With --drive: a scripted driver, e.g. "3:throttle=1;2:throttle=1,steer=0.4;1:brake=1".
        #[arg(long, value_name = "SCRIPT", hide = true, requires = "drive")]
        drive_script: Option<String>,
        #[command(flatten)]
        view: ViewArgs,
    },
}

impl Command {
    /// What running `nfsmw` with no command does: `play` with every option left to the settings.
    pub fn play() -> Self {
        let Some(play) = Cli::parse_from(["nfsmw", "play"]).command else { unreachable!("play is a command") };
        play
    }
}
