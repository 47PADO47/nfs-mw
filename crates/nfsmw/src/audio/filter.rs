//! Cutoff controls with explicit open/closed states and device-rate guards.
use kira::effect::filter::{FilterBuilder, FilterHandle, FilterMode};
use kira::effect::{Effect, EffectBuilder};
use kira::info::Info;
use kira::{Frame, Tween};
use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};

pub(super) struct CutoffHandle(Arc<AtomicU32>);
impl CutoffHandle {
    pub fn set(&self, cutoff: f32) {
        self.0.store(cutoff.to_bits(), Ordering::Relaxed);
    }
}

/// Shared music-track filter, so newly started songs inherit the current mode.
pub(super) struct MusicFilterBuilder;

impl EffectBuilder for MusicFilterBuilder {
    type Handle = CutoffHandle;
    fn build(self) -> (Box<dyn Effect>, CutoffHandle) {
        let (filter, handle) = VoiceFilter::new(false);
        (Box::new(MusicFilter(filter)), handle)
    }
}

struct MusicFilter(VoiceFilter);
impl Effect for MusicFilter {
    fn process(&mut self, frames: &mut [Frame], dt: f64, info: &Info) {
        self.0.process(frames, dt, info);
    }
}

pub(super) struct VoiceFilter {
    effect: Box<dyn Effect>,
    handle: FilterHandle,
    cutoff: Arc<AtomicU32>,
    high: bool,
    applied: Option<f64>,
    low: [blackbox_fxedit::LowPass; 2],
}

impl VoiceFilter {
    pub fn new(high: bool) -> (Self, CutoffHandle) {
        let (effect, handle) = FilterBuilder::new().cutoff(1.0).mode(FilterMode::HighPass).build();
        let cutoff = Arc::new(AtomicU32::new((if high { 0.0f32 } else { 65_535.0 }).to_bits()));
        (
            Self { effect, handle, cutoff: cutoff.clone(), high, applied: None, low: Default::default() },
            CutoffHandle(cutoff),
        )
    }

    pub fn process(&mut self, out: &mut [Frame], dt: f64, info: &Info) {
        if !dt.is_finite() || dt <= 0.0 {
            out.fill(Frame::ZERO);
            return;
        }
        let raw = f32::from_bits(self.cutoff.load(Ordering::Relaxed));
        if !self.high {
            let cutoff = if raw.is_finite() { raw.clamp(0.0, 65_535.0).round_ties_even() as u32 } else { 0 };
            let rate = dt.recip().round() as u32;
            for filter in &mut self.low {
                filter.set_cutoff(rate, cutoff);
            }
            self.low[0].process(out.iter_mut().map(|frame| &mut frame.left));
            self.low[1].process(out.iter_mut().map(|frame| &mut frame.right));
            return;
        }
        let value = if raw.is_finite() { f64::from(raw.clamp(0.0, 25_000.0)) } else { 0.0 };
        let nyquist = 0.5 / dt;
        if value <= 0.0 {
            return;
        }
        if value >= nyquist {
            out.fill(Frame::ZERO);
            return;
        }
        // Kira's coefficient uses tan(pi*f/Fs); never evaluate at the Nyquist pole, including bypassed voices.
        let cutoff = value.min(0.49 / dt).max(1.0);
        if self.applied != Some(cutoff) {
            self.handle.set_cutoff(cutoff, Tween::default());
            self.applied = Some(cutoff);
        }
        self.effect.on_start_processing();
        self.effect.process(out, dt, info);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn open_closed_and_near_nyquist_are_finite_at_supported_device_rates() {
        let info = kira::info::MockInfoBuilder::new().build();
        for rate in [44_100.0, 48_000.0, 96_000.0] {
            for high in [false, true] {
                let (mut filter, handle) = VoiceFilter::new(high);
                for cutoff in [0.0, 1000.0, 22_050.0, 24_000.0, 25_000.0, f32::NAN] {
                    handle.set(cutoff);
                    let mut frames = [Frame::from_mono(0.5); 256];
                    filter.process(&mut frames, 1.0 / rate, &info);
                    assert!(frames.iter().all(|f| f.left.is_finite() && f.right.is_finite()));
                    if !high && cutoff == 0.0 {
                        assert!(frames.iter().all(|f| f.left == 0.0));
                    }
                    if !high && cutoff >= rate as f32 * 0.5 {
                        assert!(frames.iter().all(|f| f.left == 0.5));
                    }
                }
            }
        }
    }

    #[test]
    fn low_pass_attenuates_high_frequencies_without_muting_the_passband() {
        let info = kira::info::MockInfoBuilder::new().build();
        let energy = |hz: f64| {
            let (mut filter, handle) = VoiceFilter::new(false);
            handle.set(1000.0);
            let mut sum = 0.0f64;
            for chunk in 0..100 {
                let mut frames: Vec<Frame> = (0..128)
                    .map(|i| {
                        Frame::from_mono(((chunk * 128 + i) as f64 * hz * std::f64::consts::TAU / 48000.0).sin() as f32)
                    })
                    .collect();
                filter.process(&mut frames, 1.0 / 48000.0, &info);
                if chunk > 50 {
                    sum += frames.iter().map(|f| f64::from(f.left).powi(2)).sum::<f64>();
                }
            }
            sum
        };
        let low = energy(100.0);
        let high = energy(8000.0);
        assert!(low > 100.0, "passband remains audible");
        assert!(high < low * 0.01, "high frequencies attenuated: {high}/{low}");
    }

    #[test]
    fn music_track_filter_reaches_new_sources_without_filtering_other_groups() {
        use super::super::replay::ReplayBackend;
        use kira::track::TrackBuilder;
        use kira::{AudioManager, AudioManagerSettings};
        let render = |filtered: bool, cutoff: f32, rate: u32| {
            let mut manager = AudioManager::<ReplayBackend>::new(AudioManagerSettings {
                backend_settings: rate,
                ..Default::default()
            })
            .unwrap();
            let mut music = TrackBuilder::new();
            let filter = music.add_effect(MusicFilterBuilder);
            let mut music = manager.add_sub_track(music).unwrap();
            let mut sfx = manager.add_sub_track(TrackBuilder::new()).unwrap();
            filter.set(cutoff);
            let frames: Vec<_> = (0..rate)
                .map(|i| Frame::from_mono((i as f32 * 8000.0 * std::f32::consts::TAU / rate as f32).sin() * 0.1))
                .collect();
            let sound = kira::sound::static_sound::StaticSoundData {
                sample_rate: rate,
                frames: Arc::from(frames),
                settings: Default::default(),
                slice: None,
            };
            // Set mode before starting this source: new songs inherit the music bus.
            let _voice = match filtered {
                true => music.play(sound).unwrap(),
                false => sfx.play(sound).unwrap(),
            };
            manager.backend_mut().render(rate as usize / 2);
            manager.backend_mut().render(rate as usize / 4).iter().map(|v| f64::from(*v).powi(2)).sum::<f64>()
        };
        for rate in [44100, 48000] {
            let open = render(true, 65535.0, rate);
            let held = render(true, 1800.0, rate);
            let other = render(false, 1800.0, rate);
            assert!(open > 1.0 && held < open * 0.01, "music attenuation at {rate}");
            assert!((other - open).abs() < open * 0.001, "effects remain open at {rate}");
        }
    }
}
