//! The real [`Stage`]: a pursuit track plays on its own child track of the music group, so its fade is a volume
//! tween of that track and the music volume still applies.

use std::sync::Arc;
use std::sync::atomic::AtomicU8;
use std::time::Duration;

use kira::track::{TrackBuilder, TrackHandle};
use kira::{Decibels, Easing, StartTime, Tween};

use super::conductor::{Stage, Voice};
use super::live;
use super::pursuit::PursuitFiles;
use crate::audio::radio::stream::{StreamData, StreamHandle};

pub struct KiraStage<'a> {
    pub music: &'a mut TrackHandle,
    pub files: &'a PursuitFiles,
}

/// A pursuit track: its child track and the stream on it. Dropping it removes both.
struct KiraVoice {
    track: TrackHandle,
    stream: StreamHandle,
}

fn fade(secs: f32) -> Tween {
    Tween { start_time: StartTime::Immediate, duration: Duration::from_secs_f32(secs.max(0.0)), easing: Easing::Linear }
}

impl Voice for KiraVoice {
    fn fade_out(&mut self, secs: f32) {
        self.track.set_volume(Decibels::SILENCE, fade(secs));
    }

    fn finished(&self) -> bool {
        self.stream.finished()
    }
}

impl Stage for KiraStage<'_> {
    fn start(&mut self, set: u8, control: Arc<AtomicU8>, fade_in: f32) -> Result<Box<dyn Voice>, String> {
        let node = self.files.start_node(set)?;
        let files = self.files;
        let started = live::start(&files.mus, files.mpf.clone(), files.graph.clone(), node, control)?;
        let mut track =
            self.music.add_sub_track(TrackBuilder::new().volume(Decibels::SILENCE)).map_err(|e| e.to_string())?;
        track.set_volume(Decibels::IDENTITY, fade(fade_in));
        let data = StreamData { blocks: started.blocks, sample_rate: started.sample_rate };
        let stream = track.play(data).map_err(|e| e.to_string())?;
        Ok(Box::new(KiraVoice { track, stream }))
    }
}

/// The stage when no pursuit music can play (no sound device, or the music files did not load).
pub struct Unavailable(pub String);

impl Stage for Unavailable {
    fn start(&mut self, _set: u8, _control: Arc<AtomicU8>, _fade_in: f32) -> Result<Box<dyn Voice>, String> {
        Err(self.0.clone())
    }
}
