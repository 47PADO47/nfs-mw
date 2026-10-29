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

/// The shared recordings the tables name: quoted `SOUND/...WAV` paths (the per-car files are under `CARS/`).
fn shared_paths(tables: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for (index, piece) in tables.split('"').enumerate() {
        let is_path = index % 2 == 1 && piece.starts_with("SOUND/") && piece.to_ascii_uppercase().ends_with(".WAV");
        if is_path && !piece.contains("..") && !out.iter().any(|known| known == piece) {
            out.push(piece.to_owned());
        }
    }
    out
}

/// Copy the files the car's tables name from `<sound_dir>/files` into the mod folder: the car's own
/// `CARS/<ID>/SOUND` folder, and the shared recordings (`SOUND/...`, written once for many bikes by `sound-bikes`), each
/// only when the mod folder does not already hold a file of that size. Nothing to do when both are the same folder. Returns
/// the number of files copied.
pub fn copy_files(sound_dir: &Path, mod_dir: &Path, folder: &str) -> Result<usize> {
    let same = match (fs::canonicalize(sound_dir), fs::canonicalize(mod_dir)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    };
    if same {
        return Ok(0);
    }
    let id = folder.to_ascii_uppercase();
    let source = sound_dir.join("files").join("CARS").join(&id).join("SOUND");
    let mut copied = 0;
    if source.is_dir() {
        copied += copy_tree(&source, &mod_dir.join("files").join("CARS").join(&id).join("SOUND"))?;
    }
    let Ok(tables) = read_tables(sound_dir, folder) else { return Ok(copied) };
    for path in shared_paths(&tables) {
        let (from, to) = (sound_dir.join("files").join(&path), mod_dir.join("files").join(&path));
        let Ok(meta) = fs::metadata(&from) else { continue };
        if fs::metadata(&to).is_ok_and(|have| have.len() == meta.len()) {
            continue;
        }
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
        }
        fs::copy(&from, &to).with_context(|| format!("copying {}", from.display()))?;
        copied += 1;
    }
    Ok(copied)
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

    #[test]
    fn shared_recordings_named_by_the_tables_are_copied_once() {
        let (sound, modd) = (temp("shared-sound"), temp("shared-mod"));
        fs::create_dir_all(sound.join("cars")).unwrap();
        fs::create_dir_all(sound.join("files/SOUND/RIDE6/FX")).unwrap();
        fs::write(sound.join("files/SOUND/RIDE6/FX/7.WAV"), b"riff7").unwrap();
        fs::write(sound.join("files/SOUND/RIDE6/FX/8.WAV"), b"not named").unwrap();
        let tables = "[engine_sound]
layers = [{ file = \"SOUND/RIDE6/FX/7.WAV\", rpm = 3000.0 }]
[car_sound]
shift_up = [\"SOUND/RIDE6/FX/7.WAV\", \"SOUND/../x.WAV\"]
";
        fs::write(sound.join("cars/t.sound.toml"), tables).unwrap();
        assert_eq!(copy_files(&sound, &modd, "T").unwrap(), 1);
        assert_eq!(fs::read(modd.join("files/SOUND/RIDE6/FX/7.WAV")).unwrap(), b"riff7");
        assert!(!modd.join("files/SOUND/RIDE6/FX/8.WAV").exists());
        assert_eq!(copy_files(&sound, &modd, "T").unwrap(), 0);
        fs::remove_dir_all(sound).unwrap();
        fs::remove_dir_all(modd).unwrap();
    }
}
