//! `setup`: a first-run questionnaire that writes the per-user config file.

use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use game_install::GameDir;
use nfsmw_data::game::SPEC;
use toml::{Table, Value};

use crate::settings::{Partial, Settings, WindowMode, write_file};

pub fn run(explicit: Option<&Path>) -> Result<()> {
    let Some(config) = Settings::config_path() else {
        bail!("this platform has no per-user config folder; set NFSMW_GAME_DIR instead");
    };
    println!("NFS: Most Wanted setup. Press Enter to accept the value in [brackets].");
    println!("Settings are saved to {}\n", config.display());

    let default = game_install::discover(&SPEC, explicit).ok().map(|found| found.path);
    let game_dir = ask_game_dir(default.as_deref())?;
    let mode = ask_window_mode()?;

    save_game_dir(&config, &game_dir)?;
    write_file(&config, &Partial { window_mode: Some(mode), ..Partial::default() })?;

    println!("\nSaved. Start the game with `nfsmw`.");
    Ok(())
}

/// Asks until the answer is a folder with a usable install in it.
fn ask_game_dir(default: Option<&Path>) -> Result<PathBuf> {
    loop {
        let hint = default.map_or(String::new(), |d| format!(" [{}]", d.display()));
        let answer = prompt(&format!("Where is the game installed?{hint}: "))?;
        let path = match (answer.is_empty(), default) {
            (true, Some(d)) => d.to_path_buf(),
            (true, None) => continue,
            (false, _) => expand_home(&answer),
        };
        let dir = match GameDir::open(&path) {
            Ok(dir) => dir,
            Err(e) => {
                println!("  {e}");
                continue;
            }
        };
        let check = dir.validate(&SPEC);
        if check.is_usable() {
            return Ok(path);
        }
        println!("  that folder is missing: {}", check.missing.join(", "));
    }
}

/// Asks for windowed, borderless or fullscreen; fullscreen is the game's exclusive mode.
fn ask_window_mode() -> Result<WindowMode> {
    loop {
        let answer = prompt("Window mode? 1) windowed  2) borderless  3) fullscreen [1]: ")?;
        match answer.as_str() {
            "" | "1" => return Ok(WindowMode::Windowed),
            "2" => return Ok(WindowMode::Borderless),
            "3" => return Ok(WindowMode::Exclusive),
            _ => println!("  enter 1, 2 or 3"),
        }
    }
}

/// Sets `game_dir` in the config file and keeps every other key.
fn save_game_dir(config: &Path, game_dir: &Path) -> Result<()> {
    let existing = match std::fs::read_to_string(config) {
        Ok(text) => text,
        Err(e) if e.kind() == io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e).with_context(|| format!("reading {}", config.display())),
    };
    let mut table: Table = existing.parse().context("the config file is not valid TOML")?;
    table.insert("game_dir".to_owned(), Value::String(game_dir.display().to_string()));
    if let Some(dir) = config.parent() {
        std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    }
    let text = toml::to_string(&table).context("serializing the config file")?;
    std::fs::write(config, text).with_context(|| format!("writing {}", config.display()))
}

/// Reads one line from the terminal, without the line break and without quotes (dragged-in paths have them).
fn prompt(question: &str) -> Result<String> {
    print!("{question}");
    io::stdout().flush()?;
    let mut line = String::new();
    if io::stdin().lock().read_line(&mut line)? == 0 {
        bail!("no answer (setup needs an interactive terminal)");
    }
    Ok(line.trim().trim_matches(|c| c == '"' || c == '\'').trim().to_owned())
}

/// Replaces a leading `~` with the home folder.
fn expand_home(answer: &str) -> PathBuf {
    let Some(rest) = answer.strip_prefix("~/") else {
        return PathBuf::from(answer);
    };
    match std::env::var_os("HOME") {
        Some(home) => PathBuf::from(home).join(rest),
        None => PathBuf::from(answer),
    }
}
