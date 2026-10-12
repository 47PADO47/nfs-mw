//! `nfsmw`: entry point of the NFS: Most Wanted rewrite.

mod app;
mod audio;
mod cli;
mod commands;
mod devtools;
mod frontend;
mod gui;
mod hud;
mod input;
mod movie;
mod save_format;
mod saves;
mod scenes;
mod settings;
mod ui;
mod viewer;
// The renderer does not call the weather model yet; it is tested on its own.
#[cfg_attr(not(test), allow(dead_code))]
mod weather;

use clap::Parser;

fn main() -> anyhow::Result<()> {
    devtools::install_logging();
    #[cfg(feature = "trace")]
    blackbox_bevy_render::init_tracing();
    commands::run(cli::Cli::parse())
}
