//! Command-line interface.

use std::path::PathBuf;

use blackbox_render::Backend;
use clap::{Args, Parser, Subcommand};

use crate::viewer::MaxFps;

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
    /// Show a car model (drag to orbit, scroll to zoom, Esc to quit).
    ViewCar {
        /// Car folder name under CARS/, e.g. BMWM3GTR.
        #[arg(default_value = "BMWM3GTR")]
        car: String,
        /// Level of detail to show (A = highest ... D = lowest).
        #[arg(long, default_value = "A")]
        lod: char,
        /// Show every part of the LOD (decals, damaged parts, every body kit), not just the stock car.
        #[arg(long)]
        all_parts: bool,
        /// Camera yaw in degrees.
        #[arg(long, default_value_t = 35.0, allow_negative_numbers = true)]
        yaw: f32,
        #[command(flatten)]
        view: ViewArgs,
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
        /// Load map tiles within this many metres of the camera.
        #[arg(long, default_value_t = 700.0)]
        load_radius: f32,
        /// For --screenshot: wait until every tile in range has loaded before capturing.
        #[arg(long)]
        wait_for_load: bool,
        #[command(flatten)]
        view: ViewArgs,
    },
}

/// Options shared by the viewers.
#[derive(Args, Clone)]
pub struct ViewArgs {
    /// Graphics backend: auto, vulkan, dx12 or gl.
    #[arg(long, default_value = "auto")]
    pub backend: Backend,
    /// Disable vsync.
    #[arg(long)]
    pub no_vsync: bool,
    /// Frame-rate cap: a number such as 60, or `unlocked` (vsync still applies unless --no-vsync).
    #[arg(long, value_name = "FPS|unlocked", default_value = "unlocked")]
    pub max_fps: MaxFps,
    /// Render one frame to this PNG file and exit instead of opening an interactive window.
    #[arg(long, value_name = "FILE.png")]
    pub screenshot: Option<PathBuf>,
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
