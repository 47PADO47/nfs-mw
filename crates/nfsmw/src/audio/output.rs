//! The mixer's output: the sound device, the silent output of hidden runs ([`super::silent`]) or the test replay.
use kira::backend::Backend;
use kira::track::{MainTrackBuilder, TrackBuilder, TrackHandle};
use kira::{AudioManager, AudioManagerSettings, DefaultBackend, Tween};

use super::silent::SilentBackend;
use super::{Output, filter, wet};

pub(super) enum OutputManager {
    Device(Box<AudioManager<DefaultBackend>>),
    Silent(Box<AudioManager<SilentBackend>>),
    #[cfg(test)]
    Replay(Box<AudioManager<super::replay::ReplayBackend>>),
}

impl OutputManager {
    pub(super) fn add_sub_track(&mut self, builder: TrackBuilder) -> Result<TrackHandle, String> {
        match self {
            Self::Device(manager) => manager.add_sub_track(builder).map_err(|e| e.to_string()),
            Self::Silent(manager) => manager.add_sub_track(builder).map_err(|e| e.to_string()),
            #[cfg(test)]
            Self::Replay(manager) => manager.add_sub_track(builder).map_err(|e| e.to_string()),
        }
    }
    pub(super) fn volume(&mut self, volume: kira::Decibels, tween: Tween) {
        match self {
            Self::Device(manager) => manager.main_track().set_volume(volume, tween),
            Self::Silent(manager) => manager.main_track().set_volume(volume, tween),
            #[cfg(test)]
            Self::Replay(manager) => manager.main_track().set_volume(volume, tween),
        }
    }
    /// The trace's name for this output.
    pub(super) fn kind(&self) -> &'static str {
        match self {
            Self::Device(_) => "device",
            Self::Silent(_) => "silent",
            #[cfg(test)]
            Self::Replay(_) => "replay",
        }
    }
}

/// The tracks every output has, in the order the mixer graph expects them.
fn tracks<B: Backend>(
    settings: B::Settings,
    wrap: impl FnOnce(AudioManager<B>) -> OutputManager,
) -> Result<Output, String>
where
    B::Error: std::fmt::Display,
    B::Settings: Default,
{
    let mut main = MainTrackBuilder::new();
    let wet = main.add_effect(wet::Bus::builder());
    let manager = AudioManager::<B>::new(AudioManagerSettings {
        main_track_builder: main,
        backend_settings: settings,
        ..Default::default()
    })
    .map_err(|e| e.to_string())?;
    let mut manager = wrap(manager);
    let sfx = manager.add_sub_track(TrackBuilder::new())?;
    let world = manager.add_sub_track(TrackBuilder::new())?;
    let mut music_builder = TrackBuilder::new();
    let music_filter = music_builder.add_effect(filter::MusicFilterBuilder);
    let music = manager.add_sub_track(music_builder)?;
    let engine = manager.add_sub_track(TrackBuilder::new())?;
    let hud = manager.add_sub_track(TrackBuilder::new())?;
    Ok(Output { manager, sfx, world, music, music_filter, engine, hud, wet })
}

impl Output {
    /// The default sound device, or with `silent` the silent output that opens no device.
    pub(super) fn open(silent: bool) -> Result<Self, String> {
        if silent {
            log::info!("audio: silent output (no sound device is opened)");
            return tracks::<SilentBackend>((), |manager| OutputManager::Silent(Box::new(manager)));
        }
        tracks::<DefaultBackend>(Default::default(), |manager| OutputManager::Device(Box::new(manager)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_silent_choice_builds_the_silent_manager() {
        let output = Output::open(true).unwrap();
        assert_eq!(output.manager.kind(), "silent");
    }
}
