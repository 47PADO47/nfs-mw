//! `nfsmw`: entry point of the NFS: Most Wanted rewrite.

mod app;
mod audio;
mod cli;
mod commands;
mod devtools;
mod gui;
mod hud;
mod input;
mod movie;
mod scenes;
mod settings;
mod viewer;

use clap::Parser;

fn main() -> anyhow::Result<()> {
    devtools::install_logging();
    commands::run(cli::Cli::parse())
}
