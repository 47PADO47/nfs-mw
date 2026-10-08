//! Command-line interface.

use std::path::PathBuf;

use blackbox_render::Backend;
use clap::{Args, Parser, Subcommand};

use crate::app::pacing::MaxFps;
use crate::devtools::ShowMetrics;
use crate::settings::Partial;

#[derive(Parser)]
#[command(version, about = "NFS: Most Wanted rewrite (reads data from your own install)")]
pub struct Cli {
    /// Install directory (overrides $NFSMW_GAME_DIR, .env, the config file and the registry).
    #[arg(long, global = true, value_name = "PATH")]
    pub game_dir: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Find the install and check that it is usable.
    CheckInstall,
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
        /// is put on the road nearest to --at (WASD or arrows, Space handbrake, Shift/Ctrl gears, N
        /// nitrous, R reset, F free camera).
        #[arg(long, value_name = "CAR", num_args = 0..=1, default_missing_value = "BMWM3GTR")]
        drive: Option<String>,
        /// With --drive: a scripted driver, e.g. "3:throttle=1;2:throttle=1,steer=0.4;1:brake=1".
        #[arg(long, value_name = "SCRIPT", hide = true, requires = "drive")]
        drive_script: Option<String>,
        #[command(flatten)]
        view: ViewArgs,
    },
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
    /// Render one frame to this PNG file and exit instead of opening an interactive window.
    #[arg(long, value_name = "FILE.png")]
    pub screenshot: Option<PathBuf>,
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
    /// Show the in-game HUD in any viewer (while driving it is on unless the `hud` setting is off)
    /// [env NFSMW_HUD; config `hud`].
    #[arg(long)]
    pub hud: bool,
    /// Hide the in-game HUD even while driving.
    #[arg(long, conflicts_with = "hud")]
    pub no_hud: bool,
    /// Numbers for a HUD that has no car behind it: "speed_kmh,rpm,max_rpm,gear" (for reference screenshots).
    #[arg(long, hide = true, value_name = "SPEED,RPM,MAX_RPM,GEAR", value_parser = parse_hud_demo, allow_hyphen_values = true)]
    pub hud_demo: Option<crate::hud::HudState>,
}

impl ViewArgs {
    /// `hud_default` turns the HUD on without `--hud` (driving).
    pub fn run_options(&self, dir: &game_install::GameDir, hud_default: bool) -> crate::app::RunOptions {
        crate::app::RunOptions {
            screenshot: self.screenshot.clone(),
            exec: self.exec.clone(),
            open_console: self.open_console,
            hud: (self.hud || hud_default || self.hud_demo.is_some()).then(|| dir.clone()),
            hud_demo: self.hud_demo.clone(),
            audio: (!self.no_sound && self.screenshot.is_none()).then(|| dir.clone()),
        }
    }

    /// The command-line layer of the settings: the top layer, above the environment and the config file.
    pub fn settings_layer(&self) -> Partial {
        Partial {
            backend: self.backend,
            vsync: self.no_vsync.then_some(false),
            max_fps: self.max_fps,
            show_metrics: self.show_metrics,
            master_volume: self.volume,
            hud: if self.no_hud { Some(false) } else { self.hud.then_some(true) },
            ..Partial::default()
        }
    }
}

fn parse_hud_demo(s: &str) -> Result<crate::hud::HudState, String> {
    let n: Vec<f32> =
        s.split(',').map(|v| v.trim().parse::<f32>().map_err(|e| e.to_string())).collect::<Result<_, _>>()?;
    let [speed, rpm, max_rpm, gear] = n[..] else { return Err("expected SPEED,RPM,MAX_RPM,GEAR".into()) };
    Ok(crate::hud::HudState {
        speed: speed / 3.6,
        rpm,
        max_rpm,
        gear: gear as i32,
        shift_light: rpm > 0.93 * max_rpm,
        ..Default::default()
    })
}

fn parse_xy(s: &str) -> Result<[f32; 2], String> {
    let (x, y) = s.split_once(',').ok_or("expected X,Y")?;
    let n = |v: &str| v.trim().parse::<f32>().map_err(|e| e.to_string());
    Ok([n(x)?, n(y)?])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xy() {
        assert_eq!(parse_xy("1.5,-2").unwrap(), [1.5, -2.0]);
        assert!(parse_xy("3").is_err());
    }

    #[test]
    fn cli_is_consistent() {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }
}
