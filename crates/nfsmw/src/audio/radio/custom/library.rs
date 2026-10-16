//! The custom tracks the config names: the folders listed and the playlists read, each file once.
//!
//! Only the front-end and in-game lists are built here; the `race` and `pursuit` sources are read by the config and
//! wait for the radio to have those contexts.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use blackbox_playlist::{Entry, parse, resolve, select};

use super::CustomTrack;
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
    /// The library `config` describes. Empty when the feature is off, so nothing is read from disk.
    pub fn build(config: &CustomMusic) -> Self {
        if !config.enabled {
            return Self::default();
        }
        let mut builder = Builder::default();
        let front_end = builder.add_sources(config.sources_for(CustomContext::Menu));
        let in_game = builder.add_sources(config.sources_for(CustomContext::FreeRoam));
        Self { tracks: builder.tracks, lists: [front_end, in_game] }
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
}
