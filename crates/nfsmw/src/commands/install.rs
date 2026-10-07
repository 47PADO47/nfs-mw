//! `check-install`.

use std::path::Path;

use anyhow::Result;
use game_install::GameDir;
use nfsmw_data::game::SPEC;

pub fn check(explicit: Option<&Path>) -> Result<()> {
    let found = game_install::discover(&SPEC, explicit)?;
    println!("install:     {}", found.path.display());
    println!("found via:   {}", found.source);
    if let Some(config) = game_install::config_file_path(&SPEC) {
        let state = if config.exists() { "" } else { " (not created)" };
        println!("config file: {}{state}", config.display());
    }
    let dir = GameDir::open(&found.path)?;
    println!("files:       {}", dir.file_count());
    let v = dir.validate(&SPEC);
    match (&v.exe_sha256, v.exe_build) {
        (Some(_), Some(build)) => println!("speed.exe:   {build}"),
        (Some(hash), None) => println!("speed.exe:   unknown build (sha256 {hash})"),
        (None, _) => println!("speed.exe:   not found (not needed yet)"),
    }
    if v.is_usable() {
        println!("status:      OK");
        Ok(())
    } else {
        println!("status:      missing {}", v.missing.join(", "));
        std::process::exit(1);
    }
}
