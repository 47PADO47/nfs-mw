//! `nfsmw-import2012`: builds the mod that adds the 2012 game's cars to NFS: Most Wanted (2005).

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use clap::Parser;
use nfsmw_import2012::config::{Config, Selection};
use nfsmw_import2012::{default_config_path, run, source_dir, write_default_config};

/// Convert the cars of your NFS Most Wanted (2012) install into a mod for NFS: Most Wanted (2005).
#[derive(Parser)]
#[command(version)]
struct Args {
    /// The settings file. Default: `mw2012-import.toml` next to the game's config.toml.
    #[arg(long)]
    config: Option<PathBuf>,
    /// The 2012 install folder (overrides the settings and MW2012_GAME_DIR).
    #[arg(long)]
    source: Option<PathBuf>,
    /// The 2005 install folder.
    #[arg(long)]
    game_dir: Option<PathBuf>,
    /// Where the mod folder goes.
    #[arg(long, short)]
    out: Option<PathBuf>,
    /// Only these cars (ids or name fragments, comma separated).
    #[arg(long, value_delimiter = ',')]
    cars: Vec<String>,
    /// Leave these cars out (ids or name fragments, comma separated).
    #[arg(long, value_delimiter = ',')]
    exclude: Vec<String>,
    /// Longest texture side in pixels.
    #[arg(long)]
    max_texture_size: Option<u32>,
    /// List the cars in the 2012 install and stop.
    #[arg(long)]
    list: bool,
    /// Write a commented default settings file and stop.
    #[arg(long)]
    write_config: bool,
}

fn main() -> ExitCode {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).format_target(false).init();
    match go(Args::parse()) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn go(args: Args) -> Result<ExitCode> {
    let config_path = args.config.clone().or_else(default_config_path);
    if args.write_config {
        let Some(path) = config_path else { anyhow::bail!("no settings folder is known on this system") };
        let made = write_default_config(&path)?;
        println!("{} {}", if made { "wrote" } else { "already there:" }, path.display());
        return Ok(ExitCode::SUCCESS);
    }
    let mut config = match &config_path {
        Some(path) => Config::load(path)?,
        None => Config::default(),
    };
    config.source = args.source.or(config.source);
    config.game_dir = args.game_dir.or(config.game_dir);
    config.output = args.out.or(config.output);
    config.max_texture_size = args.max_texture_size.unwrap_or(config.max_texture_size);
    if !args.cars.is_empty() {
        config.cars = Selection::Only(args.cars);
    }
    config.exclude.extend(args.exclude);
    config.check()?;
    let Some(root) = source_dir(&config) else {
        println!("No NFS Most Wanted (2012) install is set, so there is nothing to import.");
        println!("Set `source = '<folder>'` in the settings file, pass --source, or set MW2012_GAME_DIR.");
        return Ok(ExitCode::SUCCESS);
    };
    if args.list {
        let source = nfsmw_import2012::Source::open(&root)?;
        for car in nfsmw_import2012::list(&source) {
            println!("{:<14} {:>8}  {}", car.id, car.number, car.name);
        }
        return Ok(ExitCode::SUCCESS);
    }
    let report = run(&config)?;
    println!("{} cars written to {}", report.imported.len(), report.output.display());
    for (car, reason) in &report.skipped {
        println!("skipped {car}: {reason}");
    }
    Ok(ExitCode::SUCCESS)
}
