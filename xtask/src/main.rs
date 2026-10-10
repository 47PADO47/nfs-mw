//! `cargo xtask <command>`: repository chores.
//!
//! - `check [--staged]`: `leak-check` and `size-check` together (used by CI and the pre-commit hook).
//! - `leak-check [--staged]`: refuse game data, binaries and decompiler output.
//! - `size-check [--staged]`: refuse source and doc files over 500 lines.
//! - `img-diff A.png B.png [--out DIFF.png]`: compare two screenshots (mean, p99 and max difference).
//! - `install-hooks`: point git at `.githooks/` so the pre-commit hook runs.
//!
//! Without `--staged` a check covers every tracked file plus untracked files that
//! are not ignored (everything `git add .` could pick up); with `--staged` it
//! covers what is about to be committed. The whitelist `.gitignore` is the first
//! line of defence; these are the second. See CONTRIBUTING.md.

mod git;
mod imgdiff;
mod leak;
mod size;

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let staged = args.iter().any(|a| a == "--staged");
    let result = match args.first().map(String::as_str) {
        Some("check") => leak::run(staged).and_then(|leak_ok| Ok(size::run(staged)? && leak_ok)),
        Some("leak-check") => leak::run(staged),
        Some("size-check") => size::run(staged),
        Some("img-diff") => imgdiff::run(&args[1..]),
        Some("install-hooks") => git::git(&["config", "core.hooksPath", ".githooks"]).map(|_| {
            eprintln!("git hooks installed (core.hooksPath = .githooks)");
            true
        }),
        _ => {
            eprintln!(
                "usage: cargo xtask <check | leak-check | size-check> [--staged] | img-diff A.png B.png | install-hooks"
            );
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("xtask: {e}");
            ExitCode::FAILURE
        }
    }
}
