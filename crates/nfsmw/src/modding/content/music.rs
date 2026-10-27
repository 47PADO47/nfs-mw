//! The music of the mods: a mod's `music/` folder (its audio files) and the `.m3u` playlists in it, with the radio
//! contexts and the `always` flag from its `[music]` table. The custom music library reads the sealed sources
//! (`audio/radio/custom/library.rs`).

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use blackbox_modding::content::{MUSIC_FOLDER, parse_music};

use super::ModDir;
use crate::settings::CustomContext;

/// One mod's music, as the custom music library takes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MusicSource {
    /// The mod that ships the music.
    pub id: String,
    /// The mod's `music/` folder: its audio files.
    pub folder: PathBuf,
    /// The `.m3u` and `.m3u8` playlists in that folder, in name order.
    pub playlists: Vec<PathBuf>,
    /// The radio contexts the tracks may play in.
    pub contexts: Vec<CustomContext>,
    /// Whether the tracks play even when the custom music setting is off.
    pub always: bool,
}

/// The music sources of this run, set once at start-up.
static SEALED: OnceLock<Vec<MusicSource>> = OnceLock::new();

/// The music sources of this run: empty before start-up, or when no loaded mod ships music.
pub(crate) fn sources() -> &'static [MusicSource] {
    SEALED.get().map_or(&[], Vec::as_slice)
}

/// Read the `[music]` table of each mod and seal the sources. A mod without the table, or with a bad one, adds none.
pub(super) fn register(mods: &[ModDir]) {
    let sources: Vec<MusicSource> = mods.iter().filter_map(read_source).collect();
    if SEALED.set(sources).is_err() {
        log::warn!("mod music was already sealed for this run; the new set is not used");
    }
}

/// The source of one mod, or `None` when it ships no music.
fn read_source(m: &ModDir) -> Option<MusicSource> {
    let text = match std::fs::read_to_string(m.dir.join("mod.toml")) {
        Ok(text) => text,
        Err(error) => {
            log::warn!("mod {}: mod.toml is not read for its music: {error}", m.id);
            return None;
        }
    };
    let section = match parse_music(&text) {
        Ok(Some(section)) => section,
        Ok(None) => return None,
        Err(error) => {
            log::warn!("mod {}: [music] is not read: {error}", m.id);
            return None;
        }
    };
    let contexts = contexts_of(&m.id, &section.contexts);
    if contexts.is_empty() {
        log::warn!("mod {}: [music] names no context the game has; its music is not added", m.id);
        return None;
    }
    let folder = m.dir.join(MUSIC_FOLDER);
    if !folder.is_dir() {
        log::warn!("mod {}: [music] is set, but the mod has no {MUSIC_FOLDER}/ folder", m.id);
        return None;
    }
    let playlists = playlists_in(&folder);
    Some(MusicSource { id: m.id.clone(), folder, playlists, contexts, always: section.always })
}

/// The radio contexts that the names stand for. A name the game does not have is logged and skipped.
fn contexts_of(owner: &str, names: &[String]) -> Vec<CustomContext> {
    names
        .iter()
        .filter_map(|name| match name.as_str() {
            "menu" => Some(CustomContext::Menu),
            "free_roam" => Some(CustomContext::FreeRoam),
            other => {
                log::warn!("mod {owner}: music context {other:?} is not one the game has; skipped");
                None
            }
        })
        .collect()
}

/// The playlist files directly in `folder`, sorted by name.
fn playlists_in(folder: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(folder) else { return Vec::new() };
    let mut playlists: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && is_playlist(path))
        .collect();
    playlists.sort();
    playlists
}

fn is_playlist(path: &Path) -> bool {
    path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("m3u") || ext.eq_ignore_ascii_case("m3u8"))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    /// A mod folder under the temp directory, unique to this test, emptied first.
    fn mod_folder(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nfsmw-mod-music-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join(MUSIC_FOLDER)).unwrap();
        dir
    }

    fn source_of(dir: &Path, manifest: &str) -> Option<MusicSource> {
        fs::write(dir.join("mod.toml"), manifest).unwrap();
        read_source(&ModDir { id: "example.radio".to_owned(), dir: dir.to_path_buf(), priority: 0 })
    }

    #[test]
    fn a_music_folder_with_its_contexts_and_playlists_becomes_a_source() {
        let dir = mod_folder("source");
        fs::write(dir.join(MUSIC_FOLDER).join("track.mp3"), b"x").unwrap();
        fs::write(dir.join(MUSIC_FOLDER).join("set.m3u8"), b"#EXTM3U\ntrack.mp3\n").unwrap();
        let manifest = "[mod]\nid = \"example.radio\"\n\n[music]\ncontexts = [\"menu\"]\nalways = true\n";
        let source = source_of(&dir, manifest).expect("a source");
        assert_eq!(source.id, "example.radio");
        assert_eq!(source.contexts, [CustomContext::Menu]);
        assert!(source.always);
        assert_eq!(source.playlists, [dir.join(MUSIC_FOLDER).join("set.m3u8")]);
        assert_eq!(source.folder, dir.join(MUSIC_FOLDER));
    }

    #[test]
    fn no_music_table_gives_no_source_and_an_unknown_context_is_dropped() {
        let dir = mod_folder("none");
        assert!(source_of(&dir, "[mod]\nid = \"example.radio\"\n").is_none());
        let source = source_of(&dir, "[music]\ncontexts = [\"race\", \"free_roam\"]\n").expect("a source");
        assert_eq!(source.contexts, [CustomContext::FreeRoam]);
        assert!(!source.always, "always is off unless the mod sets it");
    }

    #[test]
    fn a_table_with_only_unknown_contexts_or_no_folder_gives_no_source() {
        let dir = mod_folder("unknown");
        assert!(source_of(&dir, "[music]\ncontexts = [\"pursuit\"]\n").is_none());
        let bare = mod_folder("bare");
        fs::remove_dir_all(bare.join(MUSIC_FOLDER)).unwrap();
        assert!(source_of(&bare, "[music]\n").is_none());
    }
}
