//! `nfsmw`: entry point of the NFS: Most Wanted rewrite.

mod cli;
mod commands;
mod scenes;
mod viewer;

use clap::Parser;

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info,wgpu_core=warn,wgpu_hal=warn,naga=warn"),
    )
    .init();
    commands::run(cli::Cli::parse())
}
