//! The M3U and M3U8 text: entries, their `#EXTINF` length and caption, and the path each entry names.

use std::path::{Path, PathBuf};

/// The byte-order mark that some editors put at the start of a UTF-8 file.
const UTF8_BOM: &[u8] = &[0xEF, 0xBB, 0xBF];
/// The separator between artist and title in an `#EXTINF` caption.
const CAPTION_SEPARATOR: &str = " - ";

/// One file named by a playlist.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Entry {
    /// The name as the playlist writes it (relative or absolute; see [`resolve`]).
    pub location: String,
    /// Length in seconds from `#EXTINF`; `None` when the playlist gives none or says `-1`.
    pub length_secs: Option<f32>,
    /// The artist from the caption (the part before ` - `).
    pub artist: Option<String>,
    /// The title from the caption: the part after ` - `, or the whole caption when there is no separator.
    pub title: Option<String>,
}

/// What an `#EXTINF` line says, until the entry it belongs to is read.
#[derive(Debug, Default)]
struct Info {
    length_secs: Option<f32>,
    artist: Option<String>,
    title: Option<String>,
}

/// The entries of an M3U or M3U8 file, in order. Blank lines and comments are skipped; an `#EXTINF` line gives the
/// entry after it its length and caption. Other `#` lines (such as `#EXTM3U`) are ignored.
pub fn parse(bytes: &[u8]) -> Vec<Entry> {
    let text = decode_text(bytes);
    let mut entries = Vec::new();
    let mut pending: Option<Info> = None;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix("#EXTINF:") {
            pending = Some(info(rest));
            continue;
        }
        if line.starts_with('#') {
            continue;
        }
        let info = pending.take().unwrap_or_default();
        entries.push(Entry {
            location: line.to_owned(),
            length_secs: info.length_secs,
            artist: info.artist,
            title: info.title,
        });
    }
    entries
}

/// The text of a playlist file: UTF-8 with a leading byte-order mark dropped, or, when the bytes are not UTF-8,
/// each byte read as one Latin-1 character (older playlists were written in the system code page).
pub fn decode_text(bytes: &[u8]) -> String {
    let bytes = bytes.strip_prefix(UTF8_BOM).unwrap_or(bytes);
    match std::str::from_utf8(bytes) {
        Ok(text) => text.to_owned(),
        Err(_) => bytes.iter().map(|&b| char::from(b)).collect(),
    }
}

/// The path an entry names. Backslashes are read as slashes (playlists written on Windows use them). A relative name
/// is relative to `playlist_dir`, the folder of the playlist file; an absolute name is used as it is.
pub fn resolve(playlist_dir: &Path, location: &str) -> PathBuf {
    playlist_dir.join(location.replace('\\', "/"))
}

/// Reads the `#EXTINF` text after its colon: `seconds,caption`.
fn info(rest: &str) -> Info {
    let (seconds, caption) = rest.split_once(',').unwrap_or((rest, ""));
    let length_secs = seconds.trim().parse::<f32>().ok().filter(|secs| *secs >= 0.0);
    let (artist, title) = caption_parts(caption.trim());
    Info { length_secs, artist, title }
}

/// Splits a caption into artist and title. Without ` - ` the whole caption is the title. The caption is trimmed
/// first, so neither side of a separator can be empty.
fn caption_parts(caption: &str) -> (Option<String>, Option<String>) {
    let caption = caption.trim();
    if caption.is_empty() {
        return (None, None);
    }
    let Some((artist, title)) = caption.split_once(CAPTION_SEPARATOR) else {
        return (None, Some(caption.to_owned()));
    };
    (Some(artist.trim().to_owned()), Some(title.trim().to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(location: &str) -> Entry {
        Entry { location: location.into(), ..Entry::default() }
    }

    fn entry_with_length(location: &str, secs: f32) -> Entry {
        Entry { length_secs: Some(secs), ..entry(location) }
    }

    #[test]
    fn plain_names_come_in_order_and_comments_and_blank_lines_are_skipped() {
        let text = "#EXTM3U\n\n# a comment\ntrack1.mp3\r\n   \nsub/track2.ogg  \n";
        assert_eq!(parse(text.as_bytes()), [entry("track1.mp3"), entry("sub/track2.ogg")]);
    }

    #[test]
    fn extinf_gives_the_length_and_the_artist_and_title() {
        let text = "#EXTM3U\n#EXTINF:215,Artist Name - Song Title\nsong.mp3\n";
        let got = parse(text.as_bytes());
        assert_eq!(
            got,
            [Entry {
                location: "song.mp3".into(),
                length_secs: Some(215.0),
                artist: Some("Artist Name".into()),
                title: Some("Song Title".into()),
            }]
        );
    }

    #[test]
    fn a_caption_without_a_separator_is_the_title_and_a_negative_length_is_unknown() {
        let got = parse(b"#EXTINF:-1,Just A Title\nx.wav\n");
        assert_eq!(got[0].title.as_deref(), Some("Just A Title"));
        assert_eq!(got[0].artist, None);
        assert_eq!(got[0].length_secs, None);
    }

    #[test]
    fn a_caption_that_is_only_spaces_gives_no_caption() {
        let got = parse(b"#EXTINF:10,   \na.mp3\n#EXTINF:10\nb.mp3\n");
        assert_eq!(got[0], entry_with_length("a.mp3", 10.0));
        assert_eq!(got[1], entry_with_length("b.mp3", 10.0));
    }

    #[test]
    fn the_extinf_applies_only_to_the_next_entry() {
        let got = parse(b"#EXTINF:30,A - B\nfirst.mp3\nsecond.mp3\n");
        assert_eq!(got[0].artist.as_deref(), Some("A"));
        assert_eq!(got[1], entry("second.mp3"));
    }

    #[test]
    fn utf8_with_a_byte_order_mark_reads_without_it() {
        let mut bytes = UTF8_BOM.to_vec();
        bytes.extend_from_slice("#EXTINF:5,Mañana - Canción\nmañana.mp3\n".as_bytes());
        let got = parse(&bytes);
        assert_eq!(got[0].location, "mañana.mp3");
        assert_eq!(got[0].artist.as_deref(), Some("Mañana"));
        assert_eq!(got[0].title.as_deref(), Some("Canción"));
    }

    #[test]
    fn bytes_that_are_not_utf8_are_read_as_latin_1() {
        // 0xE9 is "é" in Latin-1 and not valid UTF-8 on its own.
        let bytes = b"caf\xE9.mp3\n";
        assert_eq!(decode_text(bytes), "café.mp3\n");
        assert_eq!(parse(bytes), [entry("café.mp3")]);
    }

    #[test]
    fn relative_names_resolve_against_the_playlist_folder_and_absolute_ones_stay() {
        let dir = Path::new("music");
        assert_eq!(resolve(dir, "a/b.mp3"), Path::new("music").join("a").join("b.mp3"));
        assert_eq!(resolve(dir, "..\\up.flac"), Path::new("music").join("..").join("up.flac"));
        let absolute = if cfg!(windows) { "C:\\Songs\\x.mp3" } else { "/songs/x.mp3" };
        assert_eq!(resolve(dir, absolute), Path::new(&absolute.replace('\\', "/")));
    }
}
