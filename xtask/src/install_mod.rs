//! `cargo install-mod <MOD>` and `cargo install-mods`: build mods of the repository and put them where the game looks.
//!
//! Cargo has no hook that runs after a build, so these commands do both. `install-mod <MOD>` installs one folder of `mods/`;
//! `install-mods` installs every mod the workspace builds (the members under `mods/`, today `nfsmw-2012-content`), not the
//! examples. A mod with a library is built first (the release profile, or `--debug`; a mod outside the workspace builds
//! from its own manifest); a mod with no `cdylib` (a language pack, or a content mod with an importer) is only copied. The mod's folder under the game's
//! config folder gets `mod.toml`, the library and each folder of the mod except `src`, `tests` and `target`; nothing else in
//! it is touched, so the mod's own `config.toml`, `files/` and `cars/` stay.
//!
//! `--dir PATH` installs into the mods folder `PATH` (the one `--mods-dir` names) instead of the game's. The default is the
//! game's own, and `BLACKBOX_PROFILE_DIR` moves it the way it moves the game's config file.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The game's folder name under the per-user config folder.
const APP_DIR: &str = "nfsmw";
/// Folders of a mod that are source, not content.
const NOT_CONTENT: [&str; 3] = ["src", "tests", "target"];

struct Options {
    debug: bool,
    dir: Option<PathBuf>,
    mods: Vec<String>,
}

impl Options {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut options = Options { debug: false, dir: None, mods: Vec::new() };
        let mut args = args.iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--debug" => options.debug = true,
                "--dir" => options.dir = Some(args.next().ok_or("--dir needs a folder")?.into()),
                flag if flag.starts_with("--") => return Err(format!("unknown option `{flag}`")),
                id => options.mods.push(id.to_string()),
            }
        }
        Ok(options)
    }
}

/// `install-mod <MOD>`: returns whether the mod was installed.
pub fn run_one(args: &[String]) -> Result<bool, String> {
    let options = Options::parse(args)?;
    let root = repository()?;
    let [id] = options.mods.as_slice() else {
        return Err(format!(
            "usage: cargo install-mod <MOD> [--debug] [--dir MODS_FOLDER]; the mods are: {}",
            mods_of(&root).join(", ")
        ));
    };
    install(&root, id, &options)
}

/// `install-mods`: returns whether every mod of the workspace was installed.
pub fn run_all(args: &[String]) -> Result<bool, String> {
    let options = Options::parse(args)?;
    if !options.mods.is_empty() {
        return Err("`install-mods` takes no mod names; use `install-mod <MOD>` for one".into());
    }
    let root = repository()?;
    let ids = workspace_mods(&root)?;
    if ids.is_empty() {
        return Err("the workspace builds no mods".into());
    }
    let mut all = true;
    for id in ids {
        all &= install(&root, &id, &options)?;
    }
    Ok(all)
}

fn repository() -> Result<PathBuf, String> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "the repository folder is unknown".into())
}

/// The folders of `mods/` that hold a mod, sorted.
fn mods_of(root: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(root.join("mods")) else {
        return Vec::new();
    };
    let mut ids: Vec<String> = entries
        .flatten()
        .filter(|entry| entry.path().join("mod.toml").is_file())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect();
    ids.sort();
    ids
}

/// The mods the workspace builds: its `members` that live under `mods/`.
fn workspace_mods(root: &Path) -> Result<Vec<String>, String> {
    let manifest =
        std::fs::read_to_string(root.join("Cargo.toml")).map_err(|e| format!("could not read Cargo.toml: {e}"))?;
    Ok(members_under_mods(&manifest))
}

fn members_under_mods(manifest: &str) -> Vec<String> {
    let mut text = String::new();
    for line in manifest.lines().skip_while(|line| !line.trim_start().starts_with("members")) {
        text.push_str(line);
        if line.contains(']') {
            break;
        }
    }
    text.split('"').skip(1).step_by(2).filter_map(|member| member.strip_prefix("mods/")).map(str::to_string).collect()
}

/// The package name of a manifest, which names the library Cargo writes.
fn package_name(manifest: &str) -> Option<String> {
    let mut in_package = false;
    for line in manifest.lines().map(str::trim) {
        if line.starts_with('[') {
            in_package = line == "[package]";
            continue;
        }
        let Some(value) = line.strip_prefix("name").filter(|_| in_package) else {
            continue;
        };
        return value.split('"').nth(1).map(str::to_string);
    }
    None
}

fn install(root: &Path, id: &str, options: &Options) -> Result<bool, String> {
    let source = root.join("mods").join(id);
    if !source.join("mod.toml").is_file() {
        return Err(format!("`mods/{id}` has no mod.toml; the mods are: {}", mods_of(root).join(", ")));
    }
    let destination = match &options.dir {
        Some(dir) => dir.join(id),
        None => mods_folder()?.join(id),
    };

    let mut library = None;
    if builds_library(&source) {
        library = build(root, id, &source, options.debug)?;
        if library.is_none() {
            return Ok(false);
        }
    }

    std::fs::create_dir_all(&destination).map_err(|e| format!("could not create {}: {e}", destination.display()))?;
    copy_file(&source.join("mod.toml"), &destination)?;
    if let Some(library) = library {
        copy_file(&library, &destination)?;
    }
    copy_content(&source, &destination)?;
    eprintln!("installed {id} into {}", destination.display());
    Ok(true)
}

/// Whether the mod has a native library to build: its manifest asks for a `cdylib`. A package with only binaries (the importer
/// of a content mod) or no manifest at all is only copied.
fn builds_library(source: &Path) -> bool {
    std::fs::read_to_string(source.join("Cargo.toml")).is_ok_and(|manifest| manifest.contains("cdylib"))
}

/// Build the mod and return its library, or `None` when the build failed.
fn build(root: &Path, id: &str, source: &Path, debug: bool) -> Result<Option<PathBuf>, String> {
    let manifest = std::fs::read_to_string(source.join("Cargo.toml"))
        .map_err(|e| format!("could not read the manifest of {id}: {e}"))?;
    let package = package_name(&manifest).ok_or_else(|| format!("the manifest of {id} has no package name"))?;
    let member = workspace_mods(root)?.iter().any(|member| member == id);
    let profile = if debug { "debug" } else { "release" };

    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let mut command = Command::new(cargo);
    command.current_dir(root).arg("build");
    if member {
        command.args(["-p", &package]);
    } else {
        command.arg("--manifest-path").arg(source.join("Cargo.toml"));
    }
    if !debug {
        command.arg("--release");
    }
    let status = command.status().map_err(|e| format!("could not start cargo: {e}"))?;
    if !status.success() {
        return Ok(None);
    }
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| if member { root.join("target") } else { source.join("target") });
    Ok(Some(target.join(profile).join(library_name(&package))))
}

/// The file Cargo writes for the `cdylib` of a package.
fn library_name(package: &str) -> String {
    let stem = package.replace('-', "_");
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
    config.map(|dir| dir.join("mods")).ok_or_else(|| "the config folder is unknown; pass --dir MODS_FOLDER".into())
}

fn copy_file(from: &Path, folder: &Path) -> Result<(), String> {
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

/// Copy each folder of the mod that is content (`lang/`, say) into `destination`.
fn copy_content(source: &Path, destination: &Path) -> Result<(), String> {
    let entries = std::fs::read_dir(source).map_err(|e| format!("could not read {}: {e}", source.display()))?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let skipped = name.to_str().is_none_or(|name| NOT_CONTENT.contains(&name));
        if skipped || !entry.path().is_dir() {
            continue;
        }
        copy_tree(&entry.path(), &destination.join(&name))?;
    }
    Ok(())
}

fn copy_tree(from: &Path, to: &Path) -> Result<(), String> {
    std::fs::create_dir_all(to).map_err(|e| format!("could not create {}: {e}", to.display()))?;
    let entries = std::fs::read_dir(from).map_err(|e| format!("could not read {}: {e}", from.display()))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            copy_tree(&path, &to.join(entry.file_name()))?;
            continue;
        }
        copy_file(&path, to)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(args: &[&str]) -> Vec<String> {
        args.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn only_a_cdylib_package_is_built() {
        let folder = std::env::temp_dir().join(format!("install-mod-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        assert!(!builds_library(&folder));
        std::fs::write(folder.join("Cargo.toml"), "[package]\nname = \"tool\"\n").unwrap();
        assert!(!builds_library(&folder));
        std::fs::write(folder.join("Cargo.toml"), "[lib]\ncrate-type = [\"cdylib\"]\n").unwrap();
        assert!(builds_library(&folder));
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn the_library_is_named_after_the_package() {
        let name = library_name("nfsmw-2012-content");
        assert!(name.contains("nfsmw_2012_content"), "{name}");
        assert!(name.ends_with(".dll") || name.ends_with(".so") || name.ends_with(".dylib"), "{name}");
    }

    #[test]
    fn options_split_flags_from_mod_names() {
        let options = Options::parse(&strings(&["hello", "--debug", "--dir", "x"])).unwrap();
        assert!(options.debug);
        assert_eq!(options.dir, Some(PathBuf::from("x")));
        assert_eq!(options.mods, ["hello"]);
        assert!(Options::parse(&strings(&["--dir"])).is_err());
        assert!(Options::parse(&strings(&["--other"])).is_err());
    }

    #[test]
    fn the_workspace_mods_are_the_members_under_mods() {
        let manifest = "[workspace]\n# a comment\nmembers = [\"libs/*\", \"xtask\", \"mods/nfsmw-2012-content\"]\nresolver = \"3\"\n";
        assert_eq!(members_under_mods(manifest), ["nfsmw-2012-content"]);
        assert!(members_under_mods("[workspace]\nmembers = [\"libs/*\"]\n").is_empty());
    }

    #[test]
    fn the_package_name_is_the_one_of_the_package_table() {
        let manifest = "[lib]\nname = \"other\"\n\n[package]\nname = \"hello-mod\"\nversion = \"0.1.0\"\n";
        assert_eq!(package_name(manifest).as_deref(), Some("hello-mod"));
        assert_eq!(package_name("[lib]\nname = \"x\"\n"), None);
    }
}
