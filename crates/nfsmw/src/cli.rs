//! Command-line interface.

use std::path::PathBuf;

use blackbox_render::Backend;
use clap::{Args, Parser, Subcommand};

use crate::app::pacing::MaxFps;
use crate::devtools::{ShowMetrics, ShowReadout};
use crate::settings::{Monitor, Partial, Resolution, WindowMode};

const AFTER_HELP: &str = "With no command the game starts as `play` does: the boot movies, the title screen,
the main menu, then free roam in your car. Give options to `play`
(`nfsmw play --window-mode borderless --transmission manual`) or set them in the config file.

Examples:
  nfsmw                                  start the game
  nfsmw play --skip-boot                 start at the main menu
  nfsmw play --drive                     start driving at once
  nfsmw view-car BMWM3GTR                look at a car
  nfsmw view-world --drive SKYLINEZT     drive through the city
  nfsmw keys                             list every key, button and stick binding

Settings: command line > environment (NFSMW_*) > config file > defaults. In the game, F12 opens a console (`help`).";

#[derive(Parser)]
#[command(
    version,
    about = "NFS: Most Wanted rewrite (reads data from your own install)",
    after_help = AFTER_HELP
)]
pub struct Cli {
    /// Install directory (overrides $NFSMW_GAME_DIR, .env, the config file and the registry).
    #[arg(long, global = true, value_name = "PATH")]
    pub game_dir: Option<PathBuf>,

    /// What to run; the game itself (`play`) when left out.
    #[command(subcommand)]
    pub command: Option<Command>,
}

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
        /// Start at the main menu instead of the boot movies and the title screen.
        #[arg(long)]
        skip_boot: bool,
        /// Start driving at once (free roam with the stock car).
        #[arg(long, conflicts_with = "skip_boot")]
        drive: bool,
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
        /// For the option screens: audio, video, gameplay or controls.
        #[arg(long, default_value = "audio")]
        category: String,
        /// A scripted pad for tests and screenshots (see play); with --screenshot the default is "wait 2".
        #[arg(long, value_name = "SCRIPT")]
        ui_script: Option<String>,
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

/// Options shared by the viewers.
#[derive(Args, Clone)]
pub struct ViewArgs {
    /// Graphics backend: auto, vulkan, dx12 or gl [env NFSMW_BACKEND; default auto].
    #[arg(long)]
    pub backend: Option<Backend>,
    /// Disable vsync [env NFSMW_VSYNC=off; config `vsync = false`].
    #[arg(long)]
    pub no_vsync: bool,
    /// Frame-rate cap: a number such as 60, or `unlocked` (vsync still applies unless --no-vsync)
    /// [env NFSMW_MAX_FPS; default unlocked].
    #[arg(long, value_name = "FPS|unlocked")]
    pub max_fps: Option<MaxFps>,
    /// Performance overlay: off, basic or advanced [env NFSMW_SHOW_METRICS; default off].
    #[arg(long, value_name = "off|basic|advanced")]
    pub show_metrics: Option<ShowMetrics>,
    /// Window mode: windowed, borderless or exclusive [env NFSMW_WINDOW_MODE; default windowed].
    #[arg(long, value_name = "MODE")]
    pub window_mode: Option<WindowMode>,
    /// Monitor: current, primary or a zero-based index [env NFSMW_MONITOR; default current].
    #[arg(long, value_name = "MONITOR")]
    pub monitor: Option<Monitor>,
    /// Physical resolution: WIDTHxHEIGHT or native [env NFSMW_RESOLUTION; default native].
    /// Borderless uses the desktop size; screenshots keep their fixed size.
    #[arg(long, value_name = "WIDTHxHEIGHT|native")]
    pub resolution: Option<Resolution>,
    /// The scene's debug readout, bottom left: off, minimal (one line next to the original HUD) or full
    /// [env NFSMW_SHOW_READOUT; config `show_readout`; default minimal].
    #[arg(long, value_name = "off|minimal|full")]
    pub show_readout: Option<ShowReadout>,
    /// Render one frame to this PNG file and exit instead of opening an interactive window.
    #[arg(long, value_name = "FILE.png")]
    pub screenshot: Option<PathBuf>,
    /// Physical render target for a screenshot (default 1280x720); independent of window preferences.
    #[arg(long, hide = true, requires = "screenshot", value_name = "WIDTHxHEIGHT", value_parser = parse_screenshot_size)]
    pub screenshot_size: Option<[u32; 2]>,
    /// For --screenshot: seconds to wait after the scene is ready before the first capture [default 0].
    #[arg(long, requires = "screenshot", value_name = "SECS")]
    pub screenshot_delay: Option<f32>,
    /// For --screenshot: how many captures to take; more than one numbers the files (out-1.png, out-2.png, ...) [default 1].
    #[arg(long, requires = "screenshot", value_name = "N")]
    pub screenshot_count: Option<u32>,
    /// For --screenshot with --screenshot-count: seconds between captures [default 1].
    #[arg(long, requires = "screenshot", value_name = "SECS")]
    pub screenshot_interval: Option<f32>,
    /// Run a console command once the window is up (repeatable), e.g. --exec "fps 60" --exec "car PORSCHE911".
    #[arg(long, value_name = "COMMAND")]
    pub exec: Vec<String>,
    /// Start with the developer console open.
    #[arg(long, hide = true)]
    pub open_console: bool,
    /// Play no sound (no output device is opened).
    #[arg(long)]
    pub no_sound: bool,
    /// Master volume, 0 to 100 [env NFSMW_MASTER_VOLUME; config `master_volume`; default 80].
    #[arg(long, value_name = "0-100")]
    pub volume: Option<crate::settings::Percent>,
    /// Mute all audio: everything keeps playing (the radio and the songs too), silently.
    #[arg(long, conflicts_with = "no_sound")]
    pub muted: bool,
    /// Show the in-game HUD in any viewer (while driving it is on unless the `hud` setting is off)
    /// [env NFSMW_HUD; config `hud`].
    #[arg(long)]
    pub hud: bool,
    /// Hide the in-game HUD even while driving.
    #[arg(long, conflicts_with = "hud")]
    pub no_hud: bool,
    /// HUD placement: pc, classic (centered) or xbox360 [env NFSMW_HUD_LAYOUT; default pc].
    #[arg(long, value_name = "pc|classic|xbox360")]
    pub hud_layout: Option<crate::settings::HudLayout>,
    /// How the radio announces songs on the HUD: ea_trax (the original chyron) or custom [env NFSMW_RADIO_HUD; default ea_trax].
    #[arg(long, value_name = "ea_trax|custom")]
    pub radio_hud: Option<crate::settings::RadioHudStyle>,
    /// Enable tire smoke [env NFSMW_TIRE_SMOKE; config `tire_smoke`; default on].
    #[arg(long, conflicts_with = "no_tire_smoke")]
    pub tire_smoke: bool,
    /// Disable tire smoke.
    #[arg(long)]
    pub no_tire_smoke: bool,
    /// Smoke presentation: standard or high [env NFSMW_SMOKE_QUALITY; config `smoke_quality`; default standard].
    #[arg(long, value_name = "standard|high")]
    pub smoke_quality: Option<crate::settings::SmokeQuality>,
    /// Turn the radio off [env NFSMW_RADIO=off; config `radio = false`; default on].
    #[arg(long, conflicts_with = "radio")]
    pub no_radio: bool,
    /// Turn the radio on (it is on unless the config file or environment turn it off).
    #[arg(long)]
    pub radio: bool,
    /// Enable skid marks [env NFSMW_SKID_MARKS; config `skid_marks`; default on].
    #[arg(long, conflicts_with = "no_skid_marks")]
    pub skid_marks: bool,
    /// Disable skid marks.
    #[arg(long)]
    pub no_skid_marks: bool,
    /// Enable optional impact/scrape sparks [env NFSMW_COLLISION_SPARKS; default off].
    #[arg(long, conflicts_with = "no_collision_sparks")]
    pub collision_sparks: bool,
    /// Select original PC particles or the experimental restored streaks.
    #[arg(long)]
    pub spark_style: Option<crate::settings::SparkStyle>,
    /// Disable impact/scrape sparks.
    #[arg(long)]
    pub no_collision_sparks: bool,
    /// Enable optional high-speed wind trails [env NFSMW_SPEED_TRAILS; default off].
    #[arg(long, conflicts_with = "no_speed_trails")]
    pub speed_trails: bool,
    /// Disable high-speed wind trails.
    #[arg(long)]
    pub no_speed_trails: bool,
    /// Enable the tail-pipe flames [env NFSMW_EXHAUST_FLAMES; config `exhaust_flames`; default on].
    #[arg(long, conflicts_with = "no_exhaust_flames")]
    pub exhaust_flames: bool,
    /// Disable the tail-pipe flames (nothing of them is loaded).
    #[arg(long)]
    pub no_exhaust_flames: bool,
    /// Who changes gear: automatic or manual (Q/E, the bumpers or the wheel paddles shift)
    /// [env NFSMW_TRANSMISSION; config `transmission`; default automatic].
    #[arg(long, value_name = "automatic|manual")]
    pub transmission: Option<crate::settings::Transmission>,
    /// Numbers for a HUD that has no car behind it: "speed_kmh,rpm,max_rpm,gear[,nos_percent[,boost_psi]]" (for
    /// reference screenshots; a nitrous or boost value shows that gauge).
    #[arg(long, hide = true, value_name = "SPEED,RPM,MAX_RPM,GEAR[,NOS[,PSI]]", value_parser = parse_hud_demo, allow_hyphen_values = true)]
    pub hud_demo: Option<crate::hud::HudState>,
}

impl ViewArgs {
    /// `hud_default` turns the HUD on without `--hud` (driving).
    pub fn run_options(&self, dir: &game_install::GameDir, hud_default: bool) -> crate::app::RunOptions {
        crate::app::RunOptions {
            screenshot: self.screenshot.clone(),
            screenshot_size: self.screenshot_size,
            screenshot_delay: self.screenshot_delay.unwrap_or(0.0),
            screenshot_count: self.screenshot_count.unwrap_or(1),
            screenshot_interval: self.screenshot_interval.unwrap_or(1.0),
            exec: self.exec.clone(),
            open_console: self.open_console,
            hud: (self.hud || hud_default || self.hud_demo.is_some()).then(|| dir.clone()),
            hud_demo: self.hud_demo.clone(),
            // A screenshot run has no sound unless the radio is asked for: the radio's songs drive the chyron.
            audio: (!self.no_sound && (self.screenshot.is_none() || self.radio)).then(|| dir.clone()),
            frontend: None,
        }
    }

    /// The command-line layer of the settings: the top layer, above the environment and the config file.
    pub fn settings_layer(&self) -> Partial {
        Partial {
            backend: self.backend,
            vsync: self.no_vsync.then_some(false),
            max_fps: self.max_fps,
            show_metrics: self.show_metrics,
            window_mode: self.window_mode,
            monitor: self.monitor,
            resolution: self.resolution,
            show_readout: self.show_readout,
            master_volume: if self.muted { Some(crate::settings::Percent(0)) } else { self.volume },
            hud: if self.no_hud { Some(false) } else { self.hud.then_some(true) },
            hud_layout: self.hud_layout,
            radio_hud: self.radio_hud,
            tire_smoke: switch(self.tire_smoke, self.no_tire_smoke),
            smoke_quality: self.smoke_quality,
            radio: switch(self.radio, self.no_radio),
            skid_marks: switch(self.skid_marks, self.no_skid_marks),
            collision_sparks: switch(self.collision_sparks, self.no_collision_sparks),
            spark_style: self.spark_style,
            speed_trails: switch(self.speed_trails, self.no_speed_trails),
            exhaust_flames: switch(self.exhaust_flames, self.no_exhaust_flames),
            transmission: self.transmission,
            ..Partial::default()
        }
    }
}

fn switch(on: bool, off: bool) -> Option<bool> {
    if off {
        return Some(false);
    }
    on.then_some(true)
}

fn parse_hud_demo(s: &str) -> Result<crate::hud::HudState, String> {
    let n: Vec<f32> =
        s.split(',').map(|v| v.trim().parse::<f32>().map_err(|e| e.to_string())).collect::<Result<_, _>>()?;
    let [speed, rpm, max_rpm, gear, rest @ ..] = &n[..] else { return Err(HUD_DEMO_FORMAT.into()) };
    let (speed, rpm, max_rpm, gear) = (*speed, *rpm, *max_rpm, *gear);
    if rest.len() > 2 {
        return Err(HUD_DEMO_FORMAT.into());
    }
    let red_line = max_rpm - 500.0;
    Ok(crate::hud::HudState {
        speed: speed / 3.6,
        rpm,
        max_rpm,
        red_line,
        gear: gear as i32,
        shift_light: rpm > red_line,
        has_nos: !rest.is_empty(),
        nos: rest.first().map_or(0.0, |p| p / 100.0),
        has_turbo: rest.len() > 1,
        boost_psi: rest.get(1).copied().unwrap_or(0.0),
        ..Default::default()
    })
}

const HUD_DEMO_FORMAT: &str = "expected SPEED,RPM,MAX_RPM,GEAR[,NOS_PERCENT[,BOOST_PSI]]";

fn parse_xy(s: &str) -> Result<[f32; 2], String> {
    let (x, y) = s.split_once(',').ok_or("expected X,Y")?;
    let n = |v: &str| v.trim().parse::<f32>().map_err(|e| e.to_string());
    Ok([n(x)?, n(y)?])
}

fn parse_screenshot_size(s: &str) -> Result<[u32; 2], String> {
    let Resolution::Pixels { width, height } = s.parse::<Resolution>()? else {
        return Err("expected WIDTHxHEIGHT, not native".into());
    };
    Ok([width, height])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hud_demo_takes_optional_gauges() {
        let plain = parse_hud_demo("100,4000,8000,3").unwrap();
        assert!(!plain.has_nos && !plain.has_turbo);
        assert!((plain.speed - 100.0 / 3.6).abs() < 1e-4);
        let nos = parse_hud_demo("100,4000,8000,3,60").unwrap();
        assert!(nos.has_nos && !nos.has_turbo && (nos.nos - 0.6).abs() < 1e-6);
        let both = parse_hud_demo("100,4000,8000,3,60,-5").unwrap();
        assert!(both.has_turbo && both.boost_psi == -5.0);
        assert!(parse_hud_demo("100,4000,8000").is_err());
        assert!(parse_hud_demo("1,2,3,4,5,6,7").is_err());
    }

    #[test]
    fn xy() {
        assert_eq!(parse_xy("1.5,-2").unwrap(), [1.5, -2.0]);
        assert!(parse_xy("3").is_err());
    }

    #[test]
    fn screenshot_size_is_explicit_and_independent_of_window_preferences() {
        for size in ["1920x1080", "2560x1440", "3840x2160"] {
            let cli =
                Cli::try_parse_from(["nfsmw", "view-world", "--screenshot", "out.png", "--screenshot-size", size])
                    .unwrap();
            let Some(Command::ViewWorld { view, .. }) = cli.command else { panic!("wrong command") };
            assert_eq!(view.screenshot_size, Some(parse_screenshot_size(size).unwrap()));
            assert_eq!(view.settings_layer().resolution, None);
        }
        assert!(Cli::try_parse_from(["nfsmw", "view-world", "--screenshot-size", "1920x1080"]).is_err());
        for invalid in ["native", "0x1080", "1920x0", "16385x1080", "1920"] {
            assert!(parse_screenshot_size(invalid).is_err(), "{invalid}");
        }
    }

    #[test]
    fn cli_is_consistent() {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }

    #[test]
    fn no_command_starts_the_game() {
        let bare = Cli::try_parse_from(["nfsmw"]).unwrap();
        assert!(bare.command.is_none());
        let cli = Cli::try_parse_from(["nfsmw", "--game-dir", "X"]).unwrap();
        assert!(cli.command.is_none() && cli.game_dir.is_some());
        assert!(Cli::try_parse_from(["nfsmw", "--game-dir", "X", "view-car"]).is_ok());
        assert!(Cli::try_parse_from(["nfsmw", "--no-sound"]).is_err(), "viewer options belong to a command");
    }

    #[test]
    fn help_and_keys_are_commands() {
        use clap::error::ErrorKind;
        for flag in ["-h", "--help"] {
            let err = Cli::try_parse_from(["nfsmw", flag]).err().expect("help ends parsing");
            assert_eq!(err.kind(), ErrorKind::DisplayHelp);
            assert!(err.to_string().contains("Examples:"), "the help carries the examples");
        }
        assert!(matches!(Cli::try_parse_from(["nfsmw", "keys"]).unwrap().command, Some(Command::Keys)));
        assert!(matches!(Cli::try_parse_from(["nfsmw", "bindings"]).unwrap().command, Some(Command::Keys)));
    }

    #[test]
    fn window_options_reach_the_cli_settings_layer() {
        let cli = Cli::try_parse_from([
            "nfsmw",
            "view-car",
            "--window-mode",
            "exclusive",
            "--monitor",
            "1",
            "--resolution",
            "1920x1080",
        ])
        .unwrap();
        let Some(Command::ViewCar { view, .. }) = cli.command else { panic!("wrong command") };
        let layer = view.settings_layer();
        assert_eq!(layer.window_mode, Some(WindowMode::Exclusive));
        assert_eq!(layer.monitor, Some(Monitor::Index(1)));
        assert_eq!(layer.resolution, Some(Resolution::pixels(1920, 1080).unwrap()));
        assert!(Cli::try_parse_from(["nfsmw", "view-car", "--resolution", "0x0"]).is_err());
    }
}
