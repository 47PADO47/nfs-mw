//! `cargo xtask install-mod [ID] [--debug] [--dir PATH]`: build a mod of the workspace and put it where the game looks.
//!
//! Cargo has no hook that runs after a build, so this is the one command that does both: it builds `mods/<ID>` (default
//! `nfsmw-2012-content`; the release profile, or `--debug`) and copies its `mod.toml` and library into the mod's folder under
//! the game's config folder. Nothing else in that folder is touched: the mod's own `config.toml`, `files/` and `cars/` stay.
//! `--dir PATH` installs into `PATH` instead (a mods folder named by `--mods-dir`, say). `BLACKBOX_PROFILE_DIR` moves the
//! default folder the way it moves the game's config file.

use std::path::{Path, PathBuf};
use std::process::Command;

const DEFAULT_MOD: &str = "nfsmw-2012-content";
/// The game's folder name under the per-user config folder.
const APP_DIR: &str = "nfsmw";

/// Returns whether the mod was built and installed.
pub fn run(args: &[String]) -> Result<bool, String> {
    let debug = args.iter().any(|arg| arg == "--debug");
    let dir = flag_value(args, "--dir")?;
    let id = args
        .iter()
        .enumerate()
        .find(|(at, arg)| !arg.starts_with("--") && (*at == 0 || args[*at - 1] != "--dir"))
        .map_or(DEFAULT_MOD, |(_, arg)| arg.as_str());

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().ok_or("the repository folder is unknown")?;
    let source = root.join("mods").join(id);
    if !source.join("mod.toml").is_file() {
        return Err(format!("`mods/{id}` has no mod.toml; the mods of the workspace are the folders of `mods/`"));
    }
    let destination = match dir {
        Some(dir) => PathBuf::from(dir),
        None => mods_folder()?.join(id),
    };

    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let mut build = Command::new(cargo);
    build.current_dir(root).args(["build", "-p", id]);
    if !debug {
        build.arg("--release");
    }
    let status = build.status().map_err(|e| format!("could not start cargo: {e}"))?;
    if !status.success() {
        return Ok(false);
    }

    let target = std::env::var_os("CARGO_TARGET_DIR").map_or_else(|| root.join("target"), PathBuf::from);
    let library = target.join(if debug { "debug" } else { "release" }).join(library_name(id));
    std::fs::create_dir_all(&destination).map_err(|e| format!("could not create {}: {e}", destination.display()))?;
    for from in [source.join("mod.toml"), library] {
        copy_into(&from, &destination)?;
    }
    eprintln!("installed {id} into {}", destination.display());
    Ok(true)
}

/// The value after `name`, when the flag is there.
fn flag_value(args: &[String], name: &str) -> Result<Option<String>, String> {
    let Some(at) = args.iter().position(|arg| arg == name) else {
        return Ok(None);
    };
    args.get(at + 1).cloned().map(Some).ok_or_else(|| format!("{name} needs a value"))
}

/// The file Cargo writes for the `cdylib` of package `id`.
fn library_name(id: &str) -> String {
    let stem = id.replace('-', "_");
    if cfg!(windows) {
        return format!("{stem}.dll");
    }
    if cfg!(target_os = "macos") {
        return format!("lib{stem}.dylib");
    }
    format!("lib{stem}.so")
}

/// The game's mods folder: next to its config file, as `modding/policy.rs` looks for it.
fn mods_folder() -> Result<PathBuf, String> {
    if let Some(root) = std::env::var_os("BLACKBOX_PROFILE_DIR") {
        return Ok(PathBuf::from(root).join(APP_DIR).join("config").join("mods"));
    }
    let config = if cfg!(windows) {
        std::env::var_os("APPDATA").map(|appdata| PathBuf::from(appdata).join(APP_DIR).join("config"))
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
            .map(|config| config.join(APP_DIR))
    };
    config.map(|dir| dir.join("mods")).ok_or_else(|| "the config folder is unknown; pass --dir PATH".into())
}

fn copy_into(from: &Path, folder: &Path) -> Result<(), String> {
    let name = from.file_name().ok_or_else(|| format!("{} has no file name", from.display()))?;
    let to = folder.join(name);
    std::fs::copy(from, &to).map(drop).map_err(|e| {
        format!(
            "could not copy {} to {} (is the game running and holding the library?): {e}",
            from.display(),
            to.display()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_library_is_named_after_the_package() {
        let name = library_name("nfsmw-2012-content");
        assert!(name.contains("nfsmw_2012_content"), "{name}");
        assert!(name.ends_with(".dll") || name.ends_with(".so") || name.ends_with(".dylib"), "{name}");
    }

    #[test]
    fn a_flag_value_is_the_word_after_the_flag() {
        let args: Vec<String> = ["a", "--dir", "x"].iter().map(ToString::to_string).collect();
        assert_eq!(flag_value(&args, "--dir"), Ok(Some("x".to_string())));
        assert_eq!(flag_value(&args, "--other"), Ok(None));
        assert!(flag_value(&args[..2], "--dir").is_err());
    }
}
