//! Authored PC rain module activation and release. Spec: `rain-pc.md`.
use super::{Audio, Slot};
use crate::audio::AudioView;

#[derive(Default)]
pub(super) struct RainPlayback {
    slot: Slot,
    fade: Option<f32>,
}
pub(super) fn params(inside: bool, volume: i32, off: bool) -> [i32; 10] {
    [i32::from(inside), 0, volume.clamp(0, 32767), 7000, 0, i32::from(off), 25000, 0, 32767, 0]
}
impl RainPlayback {
    pub fn update(&mut self, audio: &mut Audio, view: AudioView, levels: [i32; 2], dt: f32) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        let active = view.rain_intensity > 0.01;
        if active && self.slot.module.is_none() {
            self.slot.ensure(audio, "IG_GLOBAL/FX_Rain_MB.abk", "FX_Weather", 61);
            self.fade = None;
        }
        if !active && self.slot.module.is_some() && self.fade.is_none() {
            self.fade = Some(10.0);
        }
        if let Some(remaining) = &mut self.fade {
            *remaining -= dt;
            if *remaining <= 0.0 {
                self.slot.module = None;
                self.fade = None;
                return;
            }
        }
        let inside = matches!(view.camera_state, 1 | 2);
        let volume = levels[usize::from(inside)];
        self.slot.update(audio, dt, &params(inside, volume, self.fade.is_some()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_member_order_keeps_release_separate_from_hood_mode() {
        assert_eq!(params(false, 12345, false), [0, 0, 12345, 7000, 0, 0, 25000, 0, 32767, 0]);
        assert_eq!(params(true, 23456, true), [1, 0, 23456, 7000, 0, 1, 25000, 0, 32767, 0]);
    }
    #[test]
    #[ignore = "requires the installed original sound bank"]
    fn installed_rain_module_plays_and_releases() {
        let dir =
            game_install::GameDir::open(std::path::PathBuf::from(std::env::var_os("NFSMW_GAME_DIR").unwrap())).unwrap();
        let mut audio = Audio::replay(dir, 48_000);
        let mut mixer = crate::audio::mixer::CarMixer::load(&audio.dir, false).unwrap();
        let mut rain = RainPlayback::default();
        let view = AudioView { rain_intensity: 1.0, ..Default::default() };
        let mut energy = 0.0;
        let input = blackbox_carsound::CarInput::default();
        let engine = blackbox_carsound::EngineOutput::default();
        for _ in 0..120 {
            mixer.set_view(view);
            mixer.update(
                1.0 / 60.0,
                &crate::audio::mixer::Frame {
                    input: &input,
                    engine: &engine,
                    effects: Default::default(),
                    master_volume: 25500,
                    sparks: false,
                },
            );
            rain.update(&mut audio, view, mixer.ambient().rain, 1.0 / 60.0);
            energy += audio.render_replay(800).iter().map(|v| f64::from(*v).powi(2)).sum::<f64>();
        }
        assert!(energy > 0.01, "rain must produce non-silent PCM, got {energy}");
        assert!(rain.slot.module.is_some(), "installed FX_Weather must execute");
        assert!(!rain.slot.failed);
        for _ in 0..660 {
            rain.update(&mut audio, AudioView::default(), [32767; 2], 1.0 / 60.0);
        }
        assert!(rain.slot.module.is_none(), "rain voice must stop after release");
    }
}
