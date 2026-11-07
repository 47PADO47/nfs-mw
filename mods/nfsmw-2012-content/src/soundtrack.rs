//! The 2012 soundtrack, streamed: a playlist that names the install's own song files.
//!
//! `UI/SONGS/` of the 2012 install holds 58 full-length songs as EA Audio `.SPS` streams, and `SONGS.BNDL` lists them (one
//! song object each, importing the wave that names its stream). The game's custom music player decodes `.SPS` itself
//! (`crates/nfsmw/src/audio/radio/custom/decode.rs`), so **nothing is converted or copied**: the mod writes only a playlist,
//! `music/MW2012.m3u8` in its folder, that names each song by its path in the install and carries its title and artist as the
//! playlist's captions. The format of the table and how the captions are found: `docs/formats/chameleon-sound.md`, "The
//! soundtrack". The playlist is made at every start, before the game reads the music of its mods, and takes a few
//! milliseconds; it is a few kilobytes on disk.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use blackbox_modding::sdk::{Ctx, Level};
use chameleon_bundle::{Bundle, ResourceKind};
use chameleon_vehicle::localised_text;

/// The folder of the mod that the game reads music from.
pub const FOLDER: &str = "music";
/// The playlist's file name.
pub const PLAYLIST: &str = "MW2012.m3u8";
/// The gameplay option that switches the songs on, and its label (a message id of the mod's catalogue, `lang/en/`).
pub const OPTION: &str = "soundtrack_2012";
pub const LABEL: &str = "nfsmw2012-soundtrack";
/// Songs shorter than this are the menu loops of the 2012 front end, not songs.
const MIN_SECS: f32 = 60.0;
/// How many ids above a song's key its texts may be: the texts of a song get the ids right after the song's own.
const WINDOW: u32 = 8;
/// The size of the main block of a song object (the other objects of the bundle are lists and other sizes).
const SONG_OBJECT_SIZE: usize = 80;

/// One song of the install.
#[derive(Debug, Clone, PartialEq)]
pub struct Song {
    /// The song's stream, a `.SPS` file in the install.
    pub file: PathBuf,
    /// Length in seconds, as the song's wave says.
    pub secs: f32,
    pub title: Option<String>,
    pub artist: Option<String>,
}

/// A song object of the table, before its texts are looked up.
struct Listed {
    /// The object's key: the low half of its resource id. The ids of its texts follow it.
    key: u32,
    file: PathBuf,
    secs: f32,
}

/// Keep the playlist in line with the settings: write it when `on` and a 2012 install is set, remove it otherwise. The
/// `music/` folder is always made, since the mod's `[music]` table names it. A problem is logged and costs only the songs.
pub fn sync(mod_dir: &Path, source: Option<&Path>, on: bool) {
    let folder = mod_dir.join(FOLDER);
    if let Err(error) = fs::create_dir_all(&folder) {
        log::warn!("the 2012 soundtrack: cannot make {}: {error}", folder.display());
        return;
    }
    let playlist = folder.join(PLAYLIST);
    let Some(source) = source.filter(|_| on) else {
        let _ = fs::remove_file(&playlist);
        return;
    };
    match write(&playlist, source) {
        Ok(songs) => log::info!("the 2012 soundtrack: {songs} songs, streamed from the install"),
        Err(error) => {
            log::warn!("the 2012 soundtrack is not added: {error:#}");
            let _ = fs::remove_file(&playlist);
        }
    }
}

/// Offer the songs as the gameplay option **2012 soundtrack** when the playlist was written (a 2012 install is set and `music` is
/// on). The option is **off by default**: the songs then join the game's music in place of the EA Trax songs only when the player
/// chooses them. While it is off the playlist is removed again here, before the game reads the music of its mods; a change in the
/// game applies at the next start. A game without `bb.settings` keeps the playlist, since it has no way to choose.
pub fn register(ctx: &Ctx, mod_dir: &Path) {
    let playlist = mod_dir.join(FOLDER).join(PLAYLIST);
    if !playlist.is_file() {
        return;
    }
    let Some(settings) = ctx.settings() else { return };
    if let Err(error) = settings.add_toggle(OPTION, LABEL, false, |_ctx, _on| {}) {
        ctx.log(Level::Warn, &format!("nfsmw-2012-content: the soundtrack setting is not added: {error}"));
        return;
    }
    if settings.get(OPTION).is_ok_and(|value| value == "on") {
        return;
    }
    let _ = fs::remove_file(&playlist);
}

/// Write the playlist of the songs of `source` (left alone when it is already right). Returns the number of songs.
fn write(playlist: &Path, source: &Path) -> Result<usize> {
    // Absolute, so that the playlist names the files wherever the game starts from.
    let source = std::path::absolute(source).context("the 2012 install folder")?;
    let songs = find(&source)?;
    let text = playlist_text(&songs);
    if fs::read_to_string(playlist).is_ok_and(|old| old == text) {
        return Ok(songs.len());
    }
    fs::write(playlist, text).with_context(|| format!("writing {}", playlist.display()))?;
    Ok(songs.len())
}

/// The songs of the install at `source`: those the table lists that have a stream in `UI/SONGS/`, at least a minute long and
/// with a title, by the order of their keys.
pub fn find(source: &Path) -> Result<Vec<Song>> {
    let songs = source.join("UI").join("SONGS");
    let table_bytes = fs::read(songs.join("SONGS.BNDL")).context("the install has no UI/SONGS/SONGS.BNDL")?;
    let table = Bundle::parse(&table_bytes).context("reading UI/SONGS/SONGS.BNDL")?;
    let language_bytes = fs::read(source.join("UI").join("LANGUAGE").join("0001.BNDL"))
        .context("the install has no UI/LANGUAGE/0001.BNDL")?;
    let language = Bundle::parse(&language_bytes).context("reading UI/LANGUAGE/0001.BNDL")?;
    let [texts, ..] = language.resource(0).context("reading the language table")?.blocks;
    let mut listed = listed_songs(&table, &songs);
    listed.sort_by_key(|song| song.key);
    let windows: Vec<Vec<String>> = listed
        .iter()
        .enumerate()
        .map(|(at, song)| {
            let next = listed.get(at + 1).map_or(u32::MAX, |next| next.key);
            let end = next.min(song.key.saturating_add(WINDOW + 1));
            (song.key + 1..end).filter_map(|id| localised_text(&texts, id)).collect()
        })
        .collect();
    let found = listed
        .into_iter()
        .zip(captions(&windows))
        .filter(|(song, (title, _))| song.secs >= MIN_SECS && title.is_some())
        .map(|(song, (title, artist))| Song { file: song.file, secs: song.secs, title, artist })
        .collect();
    Ok(found)
}

/// The song objects of `table` that name a stream present in `songs`.
fn listed_songs(table: &Bundle, songs: &Path) -> Vec<Listed> {
    let mut found: Vec<Listed> = Vec::new();
    for (index, entry) in table.entries().iter().enumerate() {
        if entry.kind != ResourceKind::GenesysObject.id() || entry.import_count != 2 {
            continue;
        }
        if entry.block_size(0) != SONG_OBJECT_SIZE {
            continue;
        }
        let Ok(object) = table.resource(index) else { continue };
        let Some((name, secs)) = object.imports.iter().find_map(|import| wave(table, import.id)) else { continue };
        let file = songs.join(name);
        if !file.is_file() || found.iter().any(|song| song.file == file) {
            continue;
        }
        found.push(Listed { key: entry.id as u32, file, secs });
    }
    found
}

/// The stream name and the length in seconds that the wave resource `id` of `table` holds: its main block has the length
/// in milliseconds as a float at 0x14 and the offset of the stream's path (`UI\SONGS\<id>.SPS`) at 0x08.
fn wave(table: &Bundle, id: u64) -> Option<(String, f32)> {
    let index = table.find(id)?;
    if table.entries()[index].kind != ResourceKind::Wave.id() {
        return None;
    }
    let resource = table.resource(index).ok()?;
    let block = &resource.blocks[0];
    let millis = f32::from_le_bytes(word(block, 0x14)?);
    let start = u32::from_le_bytes(word(block, 0x08)?) as usize;
    let text = block.get(start..)?;
    let end = text.iter().position(|&byte| byte == 0)?;
    let path = std::str::from_utf8(&text[..end]).ok()?;
    let name = path.rsplit(['\\', '/']).next()?;
    Some((name.to_owned(), millis / 1000.0))
}

fn word(block: &[u8], at: usize) -> Option<[u8; 4]> {
    block.get(at..at + 4)?.try_into().ok()
}

/// The title and artist of each song from the texts found in the ids after its key (`windows`, one per song in key order).
/// The texts of a song are its title and its artist, in that order; an artist that an earlier song already has is not stored
/// again and a window then holds the title alone. **[inferred]** from the table: a window of three texts is the song's two and
/// the artist of the next song, which the next song, with only its title, takes.
fn captions(windows: &[Vec<String>]) -> Vec<(Option<String>, Option<String>)> {
    let mut carried: Option<String> = None;
    windows
        .iter()
        .map(|texts| {
            let before = carried.take();
            match texts.as_slice() {
                [title, artist] => (Some(title.clone()), Some(artist.clone())),
                [title, artist, next] => {
                    carried = Some(next.clone());
                    (Some(title.clone()), Some(artist.clone()))
                }
                [title] => (Some(title.clone()), before),
                _ => (None, None),
            }
        })
        .collect()
}

/// The playlist of `songs`: an `#EXTINF` caption (`Artist - Title`) and the song's path, one pair a song.
pub fn playlist_text(songs: &[Song]) -> String {
    let mut text = String::from("#EXTM3U\n");
    for song in songs {
        let caption = match (&song.artist, &song.title) {
            (Some(artist), Some(title)) => format!("{artist} - {title}"),
            (None, Some(title)) => title.clone(),
            _ => String::new(),
        };
        if !caption.is_empty() {
            let caption = caption.replace(['\r', '\n'], " ");
            let _ = writeln!(text, "#EXTINF:{},{}", song.secs.round() as u32, caption.trim());
        }
        let _ = writeln!(text, "{}", song.file.display());
    }
    text
}

#[cfg(test)]
mod tests;
