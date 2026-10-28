//! The radio inside [`Audio`]: loaded on first use, updated every frame.

use kira::track::TrackHandle;

use super::custom::Library;
use super::{Control, NowPlaying, Radio};
use crate::audio::Audio;
use crate::settings::CustomMusic;
use crate::settings::eatrax::Jukebox;

/// The radio of an [`Audio`]: not loaded yet, failed to load, or ready.
#[derive(Default)]
pub enum RadioSlot {
    #[default]
    Unloaded,
    Failed(String),
    Ready(Box<Radio>),
}

/// The music file and its map, relative to the install.
const MAP: &str = "SOUND/PFDATA/MW_Music.mpf";
const STREAMS: &str = "SOUND/PFDATA/MW_Music.mus";

impl Audio {
    /// Plays a temporary EA Trax selection, or restores the paused menu song when the jukebox closes.
    pub fn preview_eatrax(&mut self, song: Option<usize>) -> Result<(), String> {
        let Some(song) = song else {
            if let RadioSlot::Ready(radio) = &mut self.radio {
                radio.end_preview();
            }
            return Ok(());
        };
        let (radio, music) = self.radio_and_music()?;
        let music = music.ok_or_else(|| "there is no sound device".to_owned())?;
        radio.preview(song, music)
    }

    /// Updates a loaded radio immediately; an unloaded one reads the saved table on first use.
    pub fn apply_jukebox(&mut self, jukebox: &Jukebox) {
        if let RadioSlot::Ready(radio) = &mut self.radio {
            radio.set_jukebox(jukebox);
        }
    }

    /// The radio, loading it the first time (the songs from the attribute database, the map from the install).
    pub(in crate::audio) fn radio(&mut self) -> Result<&mut Radio, String> {
        if matches!(self.radio, RadioSlot::Unloaded) {
            self.radio = match self.load_radio() {
                Ok(mut radio) => {
                    radio.set_enabled(self.radio_wanted);
                    RadioSlot::Ready(Box::new(radio))
                }
                Err(e) => {
                    log::warn!("no radio: {e}");
                    RadioSlot::Failed(e)
                }
            };
        }
        match &mut self.radio {
            RadioSlot::Ready(radio) => Ok(radio),
            RadioSlot::Failed(e) => Err(format!("the radio could not be loaded: {e}")),
            RadioSlot::Unloaded => Err("the radio is not loaded".into()),
        }
    }

    fn load_radio(&mut self) -> Result<Radio, String> {
        let db = self.database()?;
        let songs = nfsmw_data::music::songs(&db);
        if songs.is_empty() {
            return Err("the attribute database has no song list".into());
        }
        let map = self.dir.read(MAP).map_err(|e| format!("{MAP}: {e}"))?;
        let mus = self.dir.resolve(STREAMS).ok_or_else(|| format!("{STREAMS} is not in the install"))?;
        let jukebox = Jukebox::load(&Jukebox::defaults_for(&songs));
        let mut radio = Radio::from_files(songs, &map, mus.to_path_buf())?;
        let config = CustomMusic::load();
        radio.set_custom(&config, Library::build(&config));
        // After the custom sources: the jukebox's order flag is the EA Trax order, and it wins over `shuffle`.
        radio.set_jukebox(&jukebox);
        Ok(radio)
    }

    /// The radio and the music track, for the commands. The track is `None` without a sound device.
    pub(in crate::audio) fn radio_and_music(&mut self) -> Result<(&mut Radio, Option<&mut TrackHandle>), String> {
        self.radio()?;
        let RadioSlot::Ready(radio) = &mut self.radio else { return Err("the radio is not loaded".into()) };
        Ok((radio, self.output.as_mut().map(|out| &mut out.music)))
    }

    /// Once per frame: `driving` is true while a car is being driven and `menus` while the main menu is up. Starts the
    /// radio when driving begins or the menu comes up, and moves on to the next song when one ends. `held` is true
    /// while a pursuit has the songs' place. Does nothing before that unless the player used `radio`.
    pub fn update_radio(&mut self, driving: bool, menus: bool, held: bool) {
        let loaded = matches!(self.radio, RadioSlot::Ready(_));
        if !loaded && !((driving || menus) && self.radio_wanted) {
            return;
        }
        let audible = self.volumes.music > 0.001 && self.volumes.master > 0.001;
        if self.radio().is_err() {
            return;
        }
        let (Some(out), RadioSlot::Ready(radio)) = (self.output.as_mut(), &mut self.radio) else { return };
        radio.update_held(driving, menus, held, audible, &mut out.music);
    }

    /// The `radio` setting: off keeps the radio from loading and silences a loaded one; on lets it play again.
    pub fn set_radio_wanted(&mut self, on: bool) {
        self.radio_wanted = on;
        if let RadioSlot::Ready(radio) = &mut self.radio {
            radio.set_enabled(on);
        }
    }

    /// A player's request to the radio (pause, next, previous). Returns what happened, for the log.
    pub fn radio_control(&mut self, control: Control) -> Result<String, String> {
        let (radio, music) = self.radio_and_music()?;
        let music = music.ok_or_else(|| "there is no sound device".to_owned())?;
        radio.apply(control, music)
    }

    /// The song on the air, for the HUD.
    pub fn now_playing(&self) -> Option<&NowPlaying> {
        match &self.radio {
            RadioSlot::Ready(radio) if !radio.held => radio.now.as_ref(),
            _ => None,
        }
    }
}
