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

pub(super) struct VoiceFilter {
    effect: Box<dyn Effect>,
    handle: FilterHandle,
    cutoff: Arc<AtomicU32>,
    high: bool,
    applied: Option<f64>,
}

impl VoiceFilter {
    pub fn new(high: bool) -> (Self, CutoffHandle) {
        let (effect, handle) = FilterBuilder::new()
            .cutoff(1.0)
            .mode(if high { FilterMode::HighPass } else { FilterMode::LowPass })
            .build();
        let cutoff = Arc::new(AtomicU32::new((if high { 0.0f32 } else { 25_000.0 }).to_bits()));
        (Self { effect, handle, cutoff: cutoff.clone(), high, applied: None }, CutoffHandle(cutoff))
    }

    pub fn process(&mut self, out: &mut [Frame], dt: f64, info: &Info) {
        if !dt.is_finite() || dt <= 0.0 {
            out.fill(Frame::ZERO);
            return;
        }
        let raw = f32::from_bits(self.cutoff.load(Ordering::Relaxed));
        let value = if raw.is_finite() { f64::from(raw.clamp(0.0, 25_000.0)) } else { 0.0 };
        let nyquist = 0.5 / dt;
        let open = if self.high { value <= 0.0 } else { value >= 25_000.0 || value >= nyquist };
        if open {
            return;
        }
        let closed = if self.high { value >= nyquist } else { value <= 0.0 };
        if closed {
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
                    if !high && cutoff == 25_000.0 {
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
}
