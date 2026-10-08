//! The radio inside [`Audio`]: loaded on first use, updated every frame.

use kira::track::TrackHandle;

use super::{NowPlaying, Radio};
use crate::audio::Audio;

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
    /// The radio, loading it the first time (the songs from the attribute database, the map from the install).
    pub(in crate::audio) fn radio(&mut self) -> Result<&mut Radio, String> {
        if matches!(self.radio, RadioSlot::Unloaded) {
            self.radio = match self.load_radio() {
                Ok(radio) => RadioSlot::Ready(Box::new(radio)),
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
        Radio::from_files(songs, &map, mus.to_path_buf())
    }

    /// The radio and the music track, for the commands. The track is `None` without a sound device.
    pub(in crate::audio) fn radio_and_music(&mut self) -> Result<(&mut Radio, Option<&mut TrackHandle>), String> {
        self.radio()?;
        let RadioSlot::Ready(radio) = &mut self.radio else { return Err("the radio is not loaded".into()) };
        Ok((radio, self.output.as_mut().map(|out| &mut out.music)))
    }

    /// Once per frame: `driving` is true while a car is being driven. Starts the radio when driving begins and moves
    /// on to the next song when one ends. Does nothing before the first drive unless the player used `radio`.
    pub fn update_radio(&mut self, driving: bool) {
        if !driving && !matches!(self.radio, RadioSlot::Ready(_)) {
            return;
        }
        let audible = self.volumes.music > 0.001 && self.volumes.master > 0.001;
        if self.radio().is_err() {
            return;
        }
        let (Some(out), RadioSlot::Ready(radio)) = (self.output.as_mut(), &mut self.radio) else { return };
        radio.update(driving, audible, &mut out.music);
    }

    /// The song on the air, for the HUD (which does not draw it yet).
    #[allow(dead_code)]
    pub fn now_playing(&self) -> Option<&NowPlaying> {
        match &self.radio {
            RadioSlot::Ready(radio) => radio.now.as_ref(),
            _ => None,
        }
    }
}
