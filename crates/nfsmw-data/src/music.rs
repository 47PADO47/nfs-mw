//! The licensed songs: the AttribSys class `music`, in the order of the song list.
//!
//! The list is the `PFMapping` array of the `audiosystem` collection that `LicensedMusic` of
//! `audiosystem/0x7E4B0ED2` names; an entry's position is the song's number everywhere the radio counts songs.
//! Spec: `docs/specs/music-graph.md` §6.

use blackbox_attrib::{CollectionRef, Database, Value, vlt_hash};

/// The collection of class `audiosystem` that holds the music settings.
const MUSIC_SETTINGS: u32 = 0x7E4B_0ED2;

/// Where a song plays by default (`DefPlay` of the `music` class).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Playability {
    /// `FE`: in the front end (menus) only.
    FrontEnd,
    /// `IG`: while driving only.
    InGame,
    /// `AL`: both.
    Both,
    /// Anything else: nowhere until the player turns it on in the jukebox.
    Off,
}

impl Playability {
    /// Read the `Defplay` string.
    pub fn from_code(code: &str) -> Self {
        match code {
            "FE" => Self::FrontEnd,
            "IG" => Self::InGame,
            "AL" => Self::Both,
            _ => Self::Off,
        }
    }

    pub fn front_end(self) -> bool {
        matches!(self, Self::FrontEnd | Self::Both)
    }

    pub fn in_game(self) -> bool {
        matches!(self, Self::InGame | Self::Both)
    }
}

/// One licensed song.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Song {
    pub artist: String,
    pub title: String,
    pub album: String,
    /// `PathEvent`: the event of the music graph that starts the song (the low 24 bits identify it).
    pub event: u32,
    pub playability: Playability,
}

fn text(c: CollectionRef<'_>, field: &str) -> String {
    c.get(field).and_then(Value::as_str).unwrap_or_default().trim().to_owned()
}

fn song(c: CollectionRef<'_>) -> Song {
    let code = c.get("DefPlay").and_then(Value::as_str).unwrap_or_default();
    Song {
        artist: text(c, "Artist"),
        title: text(c, "SongName"),
        album: text(c, "Album"),
        event: c.get_u32("PathEvent").unwrap_or(0),
        playability: Playability::from_code(code.trim()),
    }
}

/// The licensed songs in list order; empty when the database has no song list.
pub fn songs(db: &Database) -> Vec<Song> {
    let Some(settings) = db.collection_by_key(vlt_hash("audiosystem"), MUSIC_SETTINGS) else { return Vec::new() };
    let Some(list) = settings.follow("LicensedMusic") else { return Vec::new() };
    let Some(Value::Array(items)) = list.get("PFMapping") else { return Vec::new() };
    items.iter().filter_map(Value::as_ref_spec).filter_map(|r| db.resolve(r)).map(song).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defplay_codes_decide_where_a_song_plays() {
        assert!(Playability::from_code("FE").front_end() && !Playability::from_code("FE").in_game());
        assert!(Playability::from_code("IG").in_game() && !Playability::from_code("IG").front_end());
        assert!(Playability::from_code("AL").front_end() && Playability::from_code("AL").in_game());
        for off in ["", "xx", "fe"] {
            let p = Playability::from_code(off);
            assert!(!p.front_end() && !p.in_game(), "{off:?}");
        }
    }
}
