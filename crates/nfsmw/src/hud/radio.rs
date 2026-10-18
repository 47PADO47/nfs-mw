//! The radio's card data: the song on the air (title, artist and album, the time) and how long it has been on the air
//! since it was last announced. The card is announced when a song starts, is paused or is resumed. The original shows
//! the song the same way when it starts (a "chyron", spec `docs/specs/music-graph.md` section 7); the package has no
//! object for it, so the Custom theme draws its own card (`ui/theme/custom/radio_card`).
//!
//! [`RadioHud`] is plain data (what to show and how long ago it was announced). Nothing here asks the radio anything:
//! the plugin copies `Audio::now_playing()` into the data every frame. Nothing here draws either.

use bevy_ecs::prelude::*;
use bevy_time::Time;

use crate::audio::{Audio, NowPlaying};

/// What the card says.
#[derive(Debug, Clone, PartialEq)]
pub struct Card {
    pub title: String,
    pub artist: String,
    pub album: String,
    /// `m:ss / m:ss`.
    pub progress: String,
    pub paused: bool,
}

impl Card {
    fn of(now: &NowPlaying) -> Self {
        Self {
            title: now.title.clone(),
            artist: now.artist.clone(),
            album: now.album.clone(),
            progress: now.progress(),
            paused: now.paused,
        }
    }

    /// The artist, and the album when the song has one.
    pub fn credit(&self) -> String {
        if self.album.is_empty() {
            return self.artist.clone();
        }
        format!("{} - {}", self.artist, self.album)
    }

    /// The time played, with the state when the song is held.
    pub fn status(&self) -> String {
        if self.paused {
            return format!("{}  PAUSED", self.progress);
        }
        self.progress.clone()
    }
}

/// The radio's card and how long it has been up.
#[derive(Resource, Debug, Default)]
pub struct RadioHud {
    card: Option<Card>,
    /// The `serial` of the song event shown last.
    seen: Option<u32>,
    /// Seconds since the card was last announced.
    age: f32,
}

impl RadioHud {
    /// One frame: `now` is the song on the air, `dt` the seconds since the last frame.
    pub fn update(&mut self, now: Option<&NowPlaying>, dt: f32) {
        let Some(now) = now else {
            *self = Self::default();
            return;
        };
        if self.seen != Some(now.serial) {
            self.seen = Some(now.serial);
            self.age = 0.0;
        }
        self.age += dt.max(0.0);
        self.card = Some(Card::of(now));
    }

    /// The song on the air, whether or not its card is up.
    pub fn song(&self) -> Option<&Card> {
        self.card.as_ref()
    }

    /// The serial of the announcement shown last (see `NowPlaying::serial`).
    pub fn serial(&self) -> Option<u32> {
        self.seen
    }

    /// Seconds since the card was last announced. The Custom card fades on this (`ui/theme/custom/radio_card`).
    pub fn age(&self) -> f32 {
        self.age
    }
}

/// Copy the song on the air into the card.
pub fn update(audio: Option<NonSend<Audio>>, time: Res<Time>, mut hud: ResMut<RadioHud>) {
    hud.update(audio.as_deref().and_then(Audio::now_playing), time.delta_secs());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now(serial: u32, paused: bool) -> NowPlaying {
        NowPlaying {
            song: 3,
            artist: "Artist".into(),
            title: "Title".into(),
            album: "Album".into(),
            elapsed_secs: 65.0,
            length_secs: 200.0,
            paused,
            serial,
        }
    }

    #[test]
    fn a_new_song_is_the_card_on_the_air_and_its_age_restarts() {
        let mut hud = RadioHud::default();
        assert!(hud.song().is_none());
        hud.update(Some(&now(1, false)), 0.016);
        let card = hud.song().unwrap();
        assert_eq!((card.title.as_str(), card.credit().as_str()), ("Title", "Artist - Album"));
        assert_eq!(card.status(), "1:05 / 3:20");
        hud.update(Some(&now(1, false)), 2.0);
        assert!((hud.age() - 2.016).abs() < 1e-4, "{}", hud.age());
        hud.update(Some(&now(2, false)), 0.5);
        assert_eq!(hud.age(), 0.5, "a new serial restarts the age");
    }

    #[test]
    fn a_pause_is_a_new_announcement_with_the_state_in_the_card() {
        let mut hud = RadioHud::default();
        hud.update(Some(&now(1, false)), 10.0);
        hud.update(Some(&now(2, true)), 0.016);
        assert_eq!(hud.song().unwrap().status(), "1:05 / 3:20  PAUSED");
        assert_eq!(hud.serial(), Some(2));
        assert_eq!(hud.age(), 0.016);
    }

    #[test]
    fn the_card_goes_when_the_song_does_and_comes_back_with_the_next() {
        let mut hud = RadioHud::default();
        hud.update(Some(&now(1, false)), 0.1);
        hud.update(None, 0.1);
        assert!(hud.song().is_none());
        assert_eq!(hud.serial(), None);
        // The same serial after a stop is a new announcement: the state was forgotten.
        hud.update(Some(&now(1, false)), 0.1);
        assert!(hud.song().is_some());
        assert_eq!(hud.age(), 0.1);
    }

    #[test]
    fn an_album_less_song_shows_only_the_artist() {
        let card = Card { album: String::new(), ..Card::of(&now(1, false)) };
        assert_eq!(card.credit(), "Artist");
    }
}
