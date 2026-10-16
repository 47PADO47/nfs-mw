//! The caption of a custom track: the playlist's title and artist first, then the file's own tags, then the file name.

use std::path::Path;

use super::CustomTrack;
use super::decode::Tags;

/// What the HUD shows for a custom track.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Caption {
    pub title: String,
    pub artist: String,
    pub album: String,
}

/// The caption of `track`, which the file `tags` describe. The title falls back to the file name and the artist to
/// nothing; blank texts count as missing.
pub fn caption(track: &CustomTrack, tags: &Tags) -> Caption {
    Caption {
        title: first_text([track.title.as_deref(), tags.title.as_deref()]).unwrap_or_else(|| file_stem(&track.path)),
        artist: first_text([track.artist.as_deref(), tags.artist.as_deref()]).unwrap_or_default(),
        album: first_text([tags.album.as_deref()]).unwrap_or_default(),
    }
}

/// The first of `values` that is not blank, trimmed.
fn first_text<const N: usize>(values: [Option<&str>; N]) -> Option<String> {
    values.into_iter().flatten().map(str::trim).find(|text| !text.is_empty()).map(str::to_owned)
}

fn file_stem(path: &Path) -> String {
    path.file_stem().and_then(|stem| stem.to_str()).unwrap_or_default().to_owned()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn track(title: Option<&str>, artist: Option<&str>) -> CustomTrack {
        CustomTrack {
            path: PathBuf::from("music/Road Song.mp3"),
            title: title.map(str::to_owned),
            artist: artist.map(str::to_owned),
            length_secs: None,
        }
    }

    fn tags(title: Option<&str>, artist: Option<&str>, album: Option<&str>) -> Tags {
        Tags { title: title.map(str::to_owned), artist: artist.map(str::to_owned), album: album.map(str::to_owned) }
    }

    #[test]
    fn the_playlist_caption_comes_before_the_tags() {
        let got =
            caption(&track(Some("From List"), Some("List Artist")), &tags(Some("Tag"), Some("Tag Artist"), Some("A")));
        assert_eq!(got, Caption { title: "From List".into(), artist: "List Artist".into(), album: "A".into() });
    }

    #[test]
    fn the_tags_come_before_the_file_name() {
        let got = caption(&track(None, None), &tags(Some("Tagged"), None, Some("Album")));
        assert_eq!(got, Caption { title: "Tagged".into(), artist: String::new(), album: "Album".into() });
    }

    #[test]
    fn with_nothing_else_the_file_name_is_the_title() {
        let got = caption(&track(None, None), &tags(Some("  "), Some(""), None));
        assert_eq!(got, Caption { title: "Road Song".into(), artist: String::new(), album: String::new() });
    }
}
