//! The `pak-sound` command: run the external pak reader (a CUE4Parse based tool the user builds, see
//! `docs/modding/ride6-bikes-sound.md`) with the pak key, to extract RIDE 6's Wwise banks and media and to resolve every
//! chassis's audio data. The key is passed in the reader's environment (`R6_AES`), so the user does not export it.
//!
//! ```text
//! pak-sound --tool <reader.exe> --usmap <ride6.usmap> --out <folder> [--resolved <folder>] [--game GAME_UE5_6]
//!           [--pattern <regex>] [--pak-key <hex>] [--config <config.toml>] [--print-key-source]
//! ```
//!
//! `--out` receives `ride6/Content/WwiseAudio/...` (every file whose path matches `--pattern`, default `WwiseAudio/`);
//! `--resolved` receives one `<chassis>__<variant>.json` per row of the chassis audio table. The reader's modes are
//! `raw <regex> <folder>` and `audiomap <folder>`.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use clap::Parser;

use super::pak_key::{self, PakKey};
use crate::config::Config;

#[derive(Parser)]
#[command(about = "Extract RIDE 6's Wwise banks and resolve every bike's audio data with the external pak reader")]
struct Args {
    /// The reader's executable (default: $R6_PAK_TOOL).
    #[arg(long)]
    tool: Option<PathBuf>,
    /// The game's `.usmap` mappings file (default: $R6_USMAP).
    #[arg(long)]
    usmap: Option<PathBuf>,
    /// Where the Wwise files are extracted (`ride6/Content/WwiseAudio/...` under it).
    #[arg(long)]
    out: Option<PathBuf>,
    /// Where the resolved audio data (one JSON per chassis and variant) is written.
    #[arg(long)]
    resolved: Option<PathBuf>,
    /// The engine version name the reader takes.
    #[arg(long, default_value = "GAME_UE5_6")]
    game: String,
    /// The paths to extract, a regular expression.
    #[arg(long, default_value = "WwiseAudio/")]
    pattern: String,
    /// The pak key as 64 hexadecimal digits; replaces $R6_AES, the `pak_key` setting and the built-in key.
    #[arg(long)]
    pak_key: Option<String>,
    /// A mod `config.toml` to read `pak_key` from.
    #[arg(long)]
    config: Option<PathBuf>,
    /// Print which key is used (never the key) and stop.
    #[arg(long)]
    print_key_source: bool,
}

fn key_for(args: &Args) -> Result<PakKey> {
    if let Some(text) = &args.pak_key {
        return Ok(PakKey { hex: pak_key::normalize(text)?, source: pak_key::Source::Argument });
    }
    let from_config = match &args.config {
        Some(path) => Config::load(path)?.pak_key,
        None => None,
    };
    pak_key::resolve(from_config.as_deref())
}

fn run_tool(tool: &Path, usmap: &Path, key: &PakKey, tool_args: &[String]) -> Result<()> {
    let mut command = Command::new(tool);
    command.args(tool_args).env("R6_USMAP", usmap);
    pak_key::apply(&mut command, key);
    let status = command.status().with_context(|| format!("starting {}", tool.display()))?;
    if !status.success() {
        bail!("{} failed ({status})", tool.display());
    }
    Ok(())
}

/// Run the command on the arguments after the word `pak-sound`.
pub fn run(args: &[String]) -> Result<()> {
    let args = Args::try_parse_from(std::iter::once("pak-sound".to_owned()).chain(args.iter().cloned()))?;
    let key = key_for(&args)?;
    println!("pak key: {:?} ({} digits)", key.source, key.hex.len());
    if args.print_key_source {
        return Ok(());
    }
    let tool = args
        .tool
        .clone()
        .or_else(|| std::env::var_os("R6_PAK_TOOL").map(PathBuf::from))
        .context("--tool or $R6_PAK_TOOL")?;
    let usmap = args
        .usmap
        .clone()
        .or_else(|| std::env::var_os("R6_USMAP").map(PathBuf::from))
        .context("--usmap or $R6_USMAP")?;
    if args.out.is_none() && args.resolved.is_none() {
        bail!("nothing to do: give --out and/or --resolved");
    }
    if let Some(out) = &args.out {
        let job = [args.game.clone(), "raw".into(), args.pattern.clone(), out.to_string_lossy().into_owned()];
        run_tool(&tool, &usmap, &key, &job)?;
    }
    if let Some(resolved) = &args.resolved {
        let job = [args.game.clone(), "audiomap".into(), resolved.to_string_lossy().into_owned()];
        run_tool(&tool, &usmap, &key, &job)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(items: &[&str]) -> Vec<String> {
        items.iter().map(|item| (*item).to_owned()).collect()
    }

    #[test]
    fn the_key_source_is_printed_without_running_anything() {
        run(&args(&["--print-key-source"])).unwrap();
        run(&args(&["--print-key-source", "--pak-key", &"ab".repeat(32)])).unwrap();
        assert!(run(&args(&["--print-key-source", "--pak-key", "12"])).is_err());
    }

    #[test]
    fn a_run_with_no_job_or_no_tool_is_refused() {
        assert!(run(&args(&["--tool", "x", "--usmap", "y"])).is_err());
        assert!(run(&args(&["--out", "z", "--usmap", "y", "--tool", "definitely-not-a-program-ride6"])).is_err());
    }
}
