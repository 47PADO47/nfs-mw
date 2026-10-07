//! `nfsmw`: entry point of the NFS: Most Wanted rewrite.
//!
//! Milestone 1 can find and check the install, list cars and show one car model.

mod car;
mod viewer;

use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use nfsmw_install::{GameDir, discover};
use nfsmw_render::Backend;

#[derive(Parser)]
#[command(version, about = "NFS: Most Wanted rewrite (reads data from your own install)")]
struct Cli {
    /// Install directory (overrides $NFSMW_GAME_DIR, .env, the config file and the registry).
    #[arg(long, global = true, value_name = "PATH")]
    game_dir: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Find the install and check that it is usable.
    CheckInstall,
    /// List the cars in the install.
    ListCars,
    /// Show a car model in a window (drag to orbit, scroll to zoom, Esc to quit).
    ViewCar {
        /// Car folder name under CARS/, e.g. BMWM3GTR.
        #[arg(default_value = "BMWM3GTR")]
        car: String,
        /// Graphics backend: auto, vulkan, dx12, dx11 (not implemented yet) or gl.
        #[arg(long, default_value = "auto")]
        backend: Backend,
        /// Level of detail to show (A = highest ... D = lowest).
        #[arg(long, default_value = "A")]
        lod: char,
        /// Show every part of the LOD (decals, damaged parts, every body kit), not just the stock car.
        #[arg(long)]
        all_parts: bool,
        /// Disable vsync.
        #[arg(long)]
        no_vsync: bool,
        /// Render one frame to this PNG file and exit instead of opening an interactive window.
        #[arg(long, value_name = "FILE.png")]
        screenshot: Option<PathBuf>,
        /// Camera yaw in degrees (screenshots / starting view).
        #[arg(long, default_value_t = 35.0, allow_negative_numbers = true)]
        yaw: f32,
    },
}

fn open_install(explicit: Option<&std::path::Path>) -> Result<GameDir> {
    let found = discover(explicit)?;
    log::info!("install: {} (from {})", found.path.display(), found.source);
    let dir = GameDir::open(&found.path).with_context(|| format!("indexing {}", found.path.display()))?;
    let v = dir.validate();
    if !v.is_usable() {
        bail!(
            "{} does not look like an NFS: Most Wanted install; missing: {}",
            dir.root().display(),
            v.missing.join(", ")
        );
    }
    Ok(dir)
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info,wgpu_core=warn,wgpu_hal=warn,naga=warn"),
    )
    .init();
    let cli = Cli::parse();

    match cli.command {
        Command::CheckInstall => {
            let found = discover(cli.game_dir.as_deref())?;
            println!("install:     {}", found.path.display());
            println!("found via:   {}", found.source);
            let dir = GameDir::open(&found.path)?;
            println!("files:       {}", dir.file_count());
            let v = dir.validate();
            match (&v.exe_sha256, v.exe_version) {
                (Some(_), Some(version)) => println!("speed.exe:   {version}"),
                (Some(hash), None) => println!("speed.exe:   unknown build (sha256 {hash})"),
                (None, _) => println!("speed.exe:   not found (not needed yet)"),
            }
            if v.is_usable() {
                println!("status:      OK");
            } else {
                println!("status:      missing {}", v.missing.join(", "));
                std::process::exit(1);
            }
        }
        Command::ListCars => {
            let dir = open_install(cli.game_dir.as_deref())?;
            for car in dir.subdirectories("CARS") {
                if dir.exists(&format!("CARS/{car}/GEOMETRY.BIN")) {
                    println!("{car}");
                }
            }
        }
        Command::ViewCar { car, backend, lod, all_parts, no_vsync, screenshot, yaw } => {
            let dir = open_install(cli.game_dir.as_deref())?;
            let model = car::load(&dir, &car, lod.to_ascii_uppercase(), all_parts)?;
            viewer::run(viewer::Options { model, backend, vsync: !no_vsync, screenshot, yaw_degrees: yaw })?;
        }
    }
    Ok(())
}
