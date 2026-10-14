//! Per-voice filtering and panning, without allocating mixer tracks for every bank sample.
use super::filter::{CutoffHandle, VoiceFilter};
use super::{Audio, Group};
use kira::Frame;
use kira::info::Info;
use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};
use kira::sound::{Sound, SoundData};

pub(super) struct ShapedHandle {
    pub voice: StaticSoundHandle,
    pub low: CutoffHandle,
    pub high: CutoffHandle,
}

pub(super) struct ShapedData(pub StaticSoundData);

struct ShapedSound {
    source: Box<dyn Sound>,
    filters: [VoiceFilter; 2],
}

impl SoundData for ShapedData {
    type Error = <StaticSoundData as SoundData>::Error;
    type Handle = ShapedHandle;
    fn into_sound(self) -> Result<(Box<dyn Sound>, Self::Handle), Self::Error> {
        let (source, voice) = self.0.into_sound()?;
        let (low_filter, low) = VoiceFilter::new(false);
        let (high_filter, high) = VoiceFilter::new(true);
        Ok((Box::new(ShapedSound { source, filters: [low_filter, high_filter] }), ShapedHandle { voice, low, high }))
    }
}

pub(super) fn set_filters(low_handle: &CutoffHandle, high_handle: &CutoffHandle, low: f32, high: f32) {
    low_handle.set(low);
    high_handle.set(high);
}

impl Sound for ShapedSound {
    fn on_start_processing(&mut self) {
        self.source.on_start_processing();
    }
    fn process(&mut self, out: &mut [Frame], dt: f64, info: &Info) {
        self.source.process(out, dt, info);
        for filter in &mut self.filters {
            filter.process(out, dt, info);
        }
    }
    fn finished(&self) -> bool {
        self.source.finished()
    }
}

impl Audio {
    pub(super) fn play_shaped(&mut self, group: Group, data: StaticSoundData) -> Result<ShapedHandle, String> {
        let out = self.output()?;
        let track = match group {
            Group::Sfx => &mut out.sfx,
            Group::Music => &mut out.music,
            Group::Engine => &mut out.engine,
        };
        track.play(ShapedData(data)).map_err(|e| e.to_string())
    }
}

/// Original azimuth: 0 ahead, quarter-turn right. Kira panning: 0 left, 1 right.
pub fn pan(azimuth: u16) -> f32 {
    0.5 + 0.5 * (f32::from(azimuth) / 65536.0 * std::f32::consts::TAU).sin()
}
