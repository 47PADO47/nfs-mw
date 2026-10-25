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
    pub spatial: super::spatial::Control,
}

pub(super) struct ShapedData {
    pub(super) data: StaticSoundData,
    pub(super) wet: super::wet::Sender,
    pub(super) params: super::spatial::Params,
}

struct ShapedSound {
    source: Box<dyn Sound>,
    filters: [VoiceFilter; 2],
    spatial: super::spatial::Control,
    ramp: super::spatial::Ramp,
    wet: super::wet::Sender,
    first: bool,
}

impl SoundData for ShapedData {
    type Error = <StaticSoundData as SoundData>::Error;
    type Handle = ShapedHandle;
    fn into_sound(self) -> Result<(Box<dyn Sound>, Self::Handle), Self::Error> {
        let (source, voice) = self.data.into_sound()?;
        let (low_filter, low) = VoiceFilter::new(false);
        let (high_filter, high) = VoiceFilter::new(true);
        let spatial = super::spatial::Control::from_params(self.params);
        Ok((
            Box::new(ShapedSound {
                source,
                filters: [low_filter, high_filter],
                spatial: spatial.clone(),
                ramp: super::spatial::Ramp::new([0.0; 3]),
                wet: self.wet,
                first: true,
            }),
            ShapedHandle { voice, low, high, spatial },
        ))
    }
}

pub(super) fn set_filters(low_handle: &CutoffHandle, high_handle: &CutoffHandle, low: f32, high: f32) {
    low_handle.set(low);
    high_handle.set(high);
}

impl Sound for ShapedSound {
    fn on_start_processing(&mut self) {
        self.source.on_start_processing();
        if let Some(params) = self.spatial.snapshot() {
            if self.first {
                self.ramp = super::spatial::Ramp::new(params.gains());
                self.first = false;
            }
            self.ramp.set(params.gains());
        }
    }
    fn process(&mut self, out: &mut [Frame], dt: f64, info: &Info) {
        self.source.process(out, dt, info);
        for filter in &mut self.filters {
            filter.process(out, dt, info);
        }
        let mut wet = [0.0_f32; 256];
        for (i, frame) in out.iter_mut().enumerate() {
            let gains = self.ramp.next();
            wet[i] = (frame.left + frame.right) * 0.5 * gains[2];
            frame.left *= gains[0];
            frame.right *= gains[1];
        }
        self.wet.mono(0, &wet[..out.len()]);
    }
    fn finished(&self) -> bool {
        self.source.finished()
    }
}

impl Audio {
    pub(super) fn play_shaped(
        &mut self,
        group: Group,
        data: StaticSoundData,
        params: super::spatial::Params,
    ) -> Result<ShapedHandle, String> {
        let out = self.output()?;
        let wet = out.wet.sender(group);
        let track = match group {
            Group::Sfx => &mut out.sfx,
            Group::Music => &mut out.music,
            Group::Engine => &mut out.engine,
            Group::Hud => &mut out.hud,
        };
        track.play(ShapedData { data, wet, params }).map_err(|e| e.to_string())
    }
}

/// Original azimuth: 0 ahead, quarter-turn right. Kira uses -1 left, 0 centre, +1 right.
pub fn pan(azimuth: u16) -> f32 {
    (f32::from(azimuth) / 65536.0 * std::f32::consts::TAU).sin()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cardinal_azimuths_use_the_signed_backend_range() {
        assert_eq!(pan(0), 0.0);
        assert!((pan(16384) - 1.0).abs() < 0.00001);
        assert!(pan(32768).abs() < 0.00001);
        assert!((pan(49152) + 1.0).abs() < 0.00001);
    }
}
