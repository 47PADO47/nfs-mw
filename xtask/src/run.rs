//! `cargo xtask run bevy|trace [-- GAME ARGS]`: launch `nfsmw` in release with the optional Bevy renderer.
//!
//! - `bevy`: the Bevy renderer (`renderer-bevy`).
//! - `trace`: the Bevy renderer plus Tracy profiling (`trace`); attach the Tracy profiler to read the spans.
//!
//! Without GAME ARGS it runs the world view on the Bevy renderer, driving at once and uncapped, which is the
//! run the profiling notes in docs/bevy-backend.md use. Anything after `--` replaces those defaults.

use std::process::Command;

const DEFAULT_GAME_ARGS: &[&str] = &["view-world", "--renderer", "bevy", "--drive", "--no-vsync"];

/// Returns whether the game exited successfully.
pub fn run(args: &[String]) -> Result<bool, String> {
    let feature = match args.first().map(String::as_str) {
        Some("bevy") => "renderer-bevy",
        Some("trace") => "trace",
        Some(other) => return Err(format!("unknown run mode `{other}`; use `bevy` or `trace`")),
        None => return Err("usage: cargo xtask run <bevy | trace> [-- GAME ARGS]".into()),
    };
    let mut game_args: Vec<&str> = args[1..].iter().map(String::as_str).collect();
    if game_args.first() == Some(&"--") {
        game_args.remove(0);
    }
    if game_args.is_empty() {
        game_args = DEFAULT_GAME_ARGS.to_vec();
    }
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let status = Command::new(cargo)
        .args(["run", "--release", "-p", "nfsmw", "--features", feature, "--"])
        .args(&game_args)
        .status()
        .map_err(|e| format!("could not start cargo: {e}"))?;
    Ok(status.success())
}
