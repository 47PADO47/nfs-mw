//! Joining the `sound` command's output to the bike's manifest (`docs/modding/ride6-bikes-sound.md`).
//!
//! `ride6-bike-import sound --out <dir> --car-id KTM450 ...` writes `<dir>/files/CARS/KTM450/SOUND/*.WAV` and the two tables
//! `[engine_sound]` and `[car_sound]` to `<dir>/cars/ktm450.sound.toml`. `--sound-dir <dir>` of the importer reads that file
//! and appends its tables to the manifest, and copies the sound files into the mod when `<dir>` is not the mod folder.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};

/// The text of the sound tables of the car `folder` written under `sound_dir`.
pub fn read_tables(sound_dir: &Path, folder: &str) -> Result<String> {
    let file = sound_dir.join("cars").join(format!("{}.sound.toml", folder.to_ascii_lowercase()));
    if !file.is_file() {
        bail!(
            "{} does not exist: run the `sound` command with --out {} and --car-id {folder} first",
            file.display(),
            sound_dir.display()
        );
    }
    fs::read_to_string(&file).with_context(|| format!("reading {}", file.display()))
}

fn copy_tree(from: &Path, to: &Path) -> Result<usize> {
    fs::create_dir_all(to).with_context(|| format!("creating {}", to.display()))?;
    let mut copied = 0;
    for entry in fs::read_dir(from).with_context(|| format!("reading {}", from.display()))?.flatten() {
        let (source, target) = (entry.path(), to.join(entry.file_name()));
        if source.is_dir() {
            copied += copy_tree(&source, &target)?;
            continue;
        }
        fs::copy(&source, &target).with_context(|| format!("copying {}", source.display()))?;
        copied += 1;
    }
    Ok(copied)
}

/// Copy `<sound_dir>/files/CARS/<ID>/SOUND` into the mod folder; nothing to do when both are the same folder. Returns the
/// number of files copied.
pub fn copy_files(sound_dir: &Path, mod_dir: &Path, folder: &str) -> Result<usize> {
    let id = folder.to_ascii_uppercase();
    let source = sound_dir.join("files").join("CARS").join(&id).join("SOUND");
    let target = mod_dir.join("files").join("CARS").join(&id).join("SOUND");
    if !source.is_dir() {
        return Ok(0);
    }
    let same = match (fs::canonicalize(&source), fs::canonicalize(&target)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    };
    if same {
        return Ok(0);
    }
    copy_tree(&source, &target)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("ride6-bikes-msound-{name}-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_tables_are_read_and_the_files_copied_unless_the_folder_is_the_mod() {
        let (sound, modd) = (temp("sound"), temp("mod"));
        fs::create_dir_all(sound.join("cars")).unwrap();
        fs::create_dir_all(sound.join("files/CARS/KTM450/SOUND")).unwrap();
        fs::write(sound.join("cars/ktm450.sound.toml"), "[engine_sound]\n").unwrap();
        fs::write(sound.join("files/CARS/KTM450/SOUND/ENG0.WAV"), b"riff").unwrap();
        assert_eq!(read_tables(&sound, "KTM450").unwrap(), "[engine_sound]\n");
        assert_eq!(copy_files(&sound, &modd, "KTM450").unwrap(), 1);
        assert_eq!(fs::read(modd.join("files/CARS/KTM450/SOUND/ENG0.WAV")).unwrap(), b"riff");
        assert_eq!(copy_files(&sound, &sound, "KTM450").unwrap(), 0);
        assert!(read_tables(&modd, "KTM450").is_err());
        fs::remove_dir_all(sound).unwrap();
        fs::remove_dir_all(modd).unwrap();
    }
}
