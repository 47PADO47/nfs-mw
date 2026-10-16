//! The custom tracks the config names: the folders listed and the playlists read, each file once.
//!
//! Only the front-end and in-game lists are built here; the `race` and `pursuit` sources are read by the config and
//! wait for the radio to have those contexts.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use blackbox_playlist::{Entry, parse, resolve, select};

use super::CustomTrack;
use crate::modding::content::{MusicSource, music_sources};
use crate::settings::{CustomContext, CustomMusic, CustomSources};

/// The extensions a folder source may hold: the formats the decoder reads.
pub const EXTENSIONS: [&str; 4] = ["mp3", "ogg", "flac", "wav"];

/// Every custom track the config names, and the lists of them. A file named twice is one track.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Library {
    pub tracks: Vec<CustomTrack>,
    /// Track numbers of the front-end list and of the in-game list, in the order of the sources.
    lists: [Vec<usize>; 2],
}

impl Library {
    /// The library `config` describes, with the music of the loaded mods. Empty when the feature is off and no mod asks
    /// for its music, so nothing is read from disk.
    pub fn build(config: &CustomMusic) -> Self {
        Self::build_with(config, music_sources())
    }

    /// [`Library::build`] with the mods' sources given. The player's own folders and playlists are read only when the
    /// feature is on. A mod's music is read when the feature is on, or when the mod sets `always`.
    pub fn build_with(config: &CustomMusic, mods: &[MusicSource]) -> Self {
        let mut builder = Builder::default();
        let mut lists: [Vec<usize>; 2] = [Vec::new(), Vec::new()];
        if config.enabled {
            lists[0] = builder.add_sources(config.sources_for(CustomContext::Menu));
            lists[1] = builder.add_sources(config.sources_for(CustomContext::FreeRoam));
        }
        for source in mods.iter().filter(|source| config.enabled || source.always) {
            let numbers = builder.add_mod(source);
            for context in &source.contexts {
                let list = match context {
                    CustomContext::Menu => 0,
                    CustomContext::FreeRoam => 1,
                };
                extend_unique(&mut lists[list], &numbers);
            }
        }
        Self { tracks: builder.tracks, lists }
    }

    /// The track numbers of list `list` (0 front end, 1 in game).
    pub fn list(&self, list: usize) -> &[usize] {
        match self.lists.get(list) {
            Some(numbers) => numbers,
            None => &[],
        }
    }

    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty()
    }
}

/// Append the numbers of `numbers` that `list` does not have yet, in order.
fn extend_unique(list: &mut Vec<usize>, numbers: &[usize]) {
    for &number in numbers {
        if !list.contains(&number) {
            list.push(number);
        }
    }
}

/// The files of one build: numbers by path, and the tracks that a playlist captioned (the first caption wins).
#[derive(Default)]
struct Builder {
    tracks: Vec<CustomTrack>,
    numbers: HashMap<PathBuf, usize>,
    captioned: HashSet<usize>,
}

impl Builder {
    /// The track numbers of one source list: its folders in order, then its playlists in order, each track once.
    fn add_sources(&mut self, sources: &CustomSources) -> Vec<usize> {
        let mut numbers = Vec::new();
        for folder in &sources.folders {
            numbers.extend(self.add_folder(folder));
        }
        for playlist in &sources.playlists {
            numbers.extend(self.add_playlist(playlist));
        }
        let mut seen = HashSet::new();
        numbers.into_iter().filter(|n| seen.insert(*n)).collect()
    }

    /// The track numbers of one mod: its `music/` folder, then its playlists, each track once.
    fn add_mod(&mut self, source: &MusicSource) -> Vec<usize> {
        let mut numbers = self.add_folder(&source.folder);
        for playlist in &source.playlists {
            numbers.extend(self.add_playlist(playlist));
        }
        let mut unique = Vec::new();
        extend_unique(&mut unique, &numbers);
        unique
    }

    fn add_folder(&mut self, folder: &Path) -> Vec<usize> {
        let entries = match std::fs::read_dir(folder) {
            Ok(entries) => entries,
            Err(e) => {
                log::warn!("custom music: cannot read the folder {}: {e}", folder.display());
                return Vec::new();
            }
        };
        let files = entries.filter_map(Result::ok).map(|entry| entry.path()).filter(|path| path.is_file());
        select(files, &EXTENSIONS).into_iter().map(|path| self.number(path, None)).collect()
    }

    fn add_playlist(&mut self, playlist: &Path) -> Vec<usize> {
        let bytes = match std::fs::read(playlist) {
            Ok(bytes) => bytes,
            Err(e) => {
                log::warn!("custom music: cannot read the playlist {}: {e}", playlist.display());
                return Vec::new();
            }
        };
        let dir = playlist.parent().unwrap_or(Path::new("."));
        let mut numbers = Vec::new();
        for entry in parse(&bytes) {
            let file = resolve(dir, &entry.location);
            if !file.is_file() {
                log::warn!(
                    "custom music: {} names {}, which is not a file; skipped",
                    playlist.display(),
                    file.display()
                );
                continue;
            }
            numbers.push(self.number(file, Some(&entry)));
        }
        numbers
    }

    /// The number of `path`'s track, made if it is new. A playlist entry with a caption sets the caption of a track
    /// that has none from a playlist yet.
    fn number(&mut self, path: PathBuf, caption: Option<&Entry>) -> usize {
        let n = match self.numbers.get(&path) {
            Some(&n) => n,
            None => {
                let n = self.tracks.len();
                self.numbers.insert(path.clone(), n);
                self.tracks.push(CustomTrack { path, title: None, artist: None, length_secs: None });
                n
            }
        };
        let Some(entry) = caption else { return n };
        let has_caption = entry.title.is_some() || entry.artist.is_some() || entry.length_secs.is_some();
        if !has_caption || !self.captioned.insert(n) {
            return n;
        }
        let track = &mut self.tracks[n];
        track.title = entry.title.clone();
        track.artist = entry.artist.clone();
        track.length_secs = entry.length_secs;
        n
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    /// A folder under the system temp folder, unique to this test, emptied first.
    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nfsmw-custom-library-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn config(folders: Vec<PathBuf>, playlists: Vec<PathBuf>) -> CustomMusic {
        CustomMusic { enabled: true, global: CustomSources { folders, playlists }, ..CustomMusic::default() }
    }

    fn names(library: &Library, list: usize) -> Vec<String> {
        library
            .list(list)
            .iter()
            .map(|&n| library.tracks[n].path.file_name().unwrap().to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn a_disabled_feature_builds_nothing() {
        let dir = scratch("disabled");
        fs::write(dir.join("a.mp3"), b"x").unwrap();
        let library = Library::build(&CustomMusic { enabled: false, ..config(vec![dir], Vec::new()) });
        assert!(library.is_empty());
        assert!(library.list(0).is_empty() && library.list(1).is_empty());
    }

    #[test]
    fn a_folder_gives_its_audio_files_in_name_order_and_leaves_out_the_rest() {
        let dir = scratch("folder");
        for name in ["b.ogg", "A.MP3", "notes.txt", "c.wav"] {
            fs::write(dir.join(name), b"x").unwrap();
        }
        fs::create_dir_all(dir.join("sub.mp3")).unwrap();
        let library = Library::build(&config(vec![dir], Vec::new()));
        assert_eq!(names(&library, 0), ["A.MP3", "b.ogg", "c.wav"]);
        assert_eq!(names(&library, 1), ["A.MP3", "b.ogg", "c.wav"]);
        assert!(library.tracks.iter().all(|t| t.title.is_none()));
    }

    #[test]
    fn a_playlist_gives_its_captions_and_skips_missing_files() {
        let dir = scratch("playlist");
        fs::create_dir_all(dir.join("songs")).unwrap();
        fs::write(dir.join("songs").join("one.mp3"), b"x").unwrap();
        fs::write(dir.join("two.flac"), b"x").unwrap();
        let text = "#EXTM3U\n#EXTINF:120,Band - First\nsongs/one.mp3\n#EXTINF:-1,Second\ntwo.flac\nmissing.mp3\n";
        let list = dir.join("list.m3u8");
        fs::write(&list, text).unwrap();
        let library = Library::build(&config(Vec::new(), vec![list]));
        assert_eq!(names(&library, 0), ["one.mp3", "two.flac"]);
        let one = &library.tracks[library.list(0)[0]];
        assert_eq!(one.artist.as_deref(), Some("Band"));
        assert_eq!(one.title.as_deref(), Some("First"));
        assert_eq!(one.length_secs, Some(120.0));
        assert_eq!(library.tracks[library.list(0)[1]].title.as_deref(), Some("Second"));
    }

    #[test]
    fn a_file_named_twice_is_one_track_and_the_playlist_caption_wins_over_the_folder() {
        let dir = scratch("twice");
        fs::write(dir.join("song.mp3"), b"x").unwrap();
        let list = dir.join("pick.m3u");
        fs::write(&list, "#EXTINF:10,Captioned - Title\nsong.mp3\n").unwrap();
        let library = Library::build(&config(vec![dir.clone()], vec![list.clone(), list]));
        assert_eq!(library.tracks.len(), 1);
        assert_eq!(library.list(0), [0]);
        assert_eq!(library.tracks[0].title.as_deref(), Some("Title"));
    }

    #[test]
    fn a_missing_folder_is_skipped_and_the_others_still_count() {
        let dir = scratch("missing");
        fs::write(dir.join("ok.wav"), b"x").unwrap();
        let library = Library::build(&config(vec![dir.join("gone"), dir], Vec::new()));
        assert_eq!(names(&library, 0), ["ok.wav"]);
    }

    #[test]
    fn a_context_with_its_own_sources_gets_only_those() {
        let own = scratch("own");
        let global = scratch("global");
        fs::write(own.join("menu.mp3"), b"x").unwrap();
        fs::write(global.join("everywhere.mp3"), b"x").unwrap();
        let mut config = config(vec![global], Vec::new());
        config.menu = CustomSources { folders: vec![own], playlists: Vec::new() };
        let library = Library::build(&config);
        assert_eq!(names(&library, 0), ["menu.mp3"]);
        assert_eq!(names(&library, 1), ["everywhere.mp3"]);
    }

    /// A mod's source over a temp `music/` folder with one track and one playlist that names it.
    fn mod_source(name: &str, contexts: Vec<CustomContext>, always: bool) -> MusicSource {
        let dir = scratch(name);
        fs::write(dir.join("mod.mp3"), b"x").unwrap();
        fs::write(dir.join("set.m3u"), "#EXTINF:30,Band - Set\nmod.mp3\n").unwrap();
        MusicSource { id: name.to_owned(), folder: dir.clone(), playlists: vec![dir.join("set.m3u")], contexts, always }
    }

    #[test]
    fn a_mod_source_is_read_when_custom_music_is_off_only_if_it_asks_for_it() {
        let quiet = config(Vec::new(), Vec::new());
        let off = Library::build_with(
            &CustomMusic { enabled: false, ..quiet.clone() },
            &[mod_source("off", vec![CustomContext::Menu], false)],
        );
        assert!(off.is_empty());
        let always = Library::build_with(
            &CustomMusic { enabled: false, ..quiet },
            &[mod_source("always", vec![CustomContext::FreeRoam], true)],
        );
        assert_eq!(names(&always, 1), ["mod.mp3"]);
        assert!(always.list(0).is_empty());
        assert_eq!(always.tracks[0].title.as_deref(), Some("Set"), "the playlist caption is kept");
    }

    #[test]
    fn a_mod_source_joins_the_contexts_it_names_once_each() {
        let contexts = vec![CustomContext::Menu, CustomContext::FreeRoam];
        let own = scratch("joins-own");
        fs::write(own.join("mine.mp3"), b"x").unwrap();
        let mut config = config(Vec::new(), Vec::new());
        config.menu = CustomSources { folders: vec![own], playlists: Vec::new() };
        let library = Library::build_with(&config, &[mod_source("joins", contexts, false)]);
        assert_eq!(names(&library, 0), ["mine.mp3", "mod.mp3"]);
        assert_eq!(names(&library, 1), ["mod.mp3"]);
        assert_eq!(library.tracks.len(), 2, "the mod's file is one track in both lists");
    }
}
