//! Command-line interface.

use std::path::PathBuf;

use clap::Parser;

pub use command::Command;
pub use view_args::ViewArgs;

const AFTER_HELP: &str = "With no command the game starts as `play` does: the boot movies, the title screen,
the main menu, then free roam in your car. Give options to `play`
(`nfsmw play --window-mode borderless --transmission manual`) or set them in the config file.

Examples:
  nfsmw                                  start the game
  nfsmw play --skip-boot                 start at the main menu
  nfsmw play --drive                     start driving at once
  nfsmw view-car BMWM3GTR                look at a car
  nfsmw asset-viewer                     browse every car, click one to show it
  nfsmw view-assets                      browse the poles, cones and other scenery models
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

mod command;
mod hud_demo;
mod parse;
#[cfg(test)]
mod tests;
mod view_args;
