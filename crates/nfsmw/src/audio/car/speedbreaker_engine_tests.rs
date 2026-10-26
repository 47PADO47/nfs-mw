//! Audible engine-mode routing through the actual installed synth and host mixer.
use super::*;
use crate::audio::{AudioView, capture};

#[test]
#[ignore = "requires installed audio banks; set NFSMW_GAME_DIR"]
fn installed_speedbreaker_attenuates_actual_engine_pcm_and_restores_it_at_high_fps() {
    let dir =
        game_install::GameDir::open(std::path::PathBuf::from(std::env::var_os("NFSMW_GAME_DIR").unwrap())).unwrap();
    for rate in [44_100, 48_000] {
        for fps in [5_u32, 60, 144, 360] {
            let mut audio = Audio::replay(dir.clone(), rate);
            let mut control = Audio::replay(dir.clone(), rate);
            audio.set_volumes(crate::audio::Volumes { sfx: 0.0, music: 0.0, ..Default::default() });
            control.set_volumes(crate::audio::Volumes { sfx: 0.0, music: 0.0, ..Default::default() });
            let input = CarInput { rpm_pct: 0.65, throttle: 0.5, speed: 30.0, gear: 3, ..Default::default() };
            let mut recorded = Vec::new();
            let mut baseline = Vec::new();
            let mut tick = 0;
            for frame in 0..fps * 6 {
                let until = (frame + 1) * 60 / fps;
                let mut ticks = Vec::new();
                while tick < until {
                    ticks.push(CarSoundTick { telemetry: Default::default(), input, contacts: vec![], surface: 0 });
                    tick += 1;
                }
                let mut state = CarSoundState {
                    nitrous_fx: None,
                    telemetry: Default::default(),
                    car: "BMWM3GTR".into(),
                    upgrades: Default::default(),
                    input,
                    events: vec![],
                    scrape: None,
                    surface: 0,
                    ticks,
                    view: AudioView { speedbreaker: (fps * 2..fps * 4).contains(&frame), ..Default::default() },
                };
                audio.drive_car(Some(&state), 1.0 / fps as f32);
                state.view.speedbreaker = false;
                control.drive_car(Some(&state), 1.0 / fps as f32);
                let from = u64::from(frame) * u64::from(rate) / u64::from(fps);
                let to = u64::from(frame + 1) * u64::from(rate) / u64::from(fps);
                recorded.extend_from_slice(audio.render_replay((to - from) as usize));
                baseline.extend_from_slice(control.render_replay((to - from) as usize));
            }
            let rms = |pcm: &[f32], end: usize| {
                let from = (end * rate as usize - rate as usize / 2) * 2;
                capture::levels(&pcm[from..end * rate as usize * 2]).rms
            };
            let normal = rms(&recorded, 2);
            let held = rms(&recorded, 4);
            let restored = rms(&recorded, 6);
            let held_control = rms(&baseline, 4);
            let restored_control = rms(&baseline, 6);
            eprintln!(
                "{rate}Hz {fps}FPS engine PCM normal{normal} held{held}/{held_control} restored{restored}/{restored_control}"
            );
            assert!(normal > 0.005, "installed engine must be audible before mode entry");
            assert!(held < held_control * 0.15, "authored engine attenuation must reach actual PCM");
            // Engine recordings and sample grains need not retain identical phase.
            // Check audible restoration and exact gain/filter routing independently.
            assert!(restored > held * 10.0 && restored > 0.005, "mode exit must restore audible engine output");
            let actual = audio.car.as_ref().unwrap().levels;
            let expected = control.car.as_ref().unwrap().levels;
            for (actual, expected) in [
                (actual.engine_volume, expected.engine_volume),
                (actual.engine_samples, expected.engine_samples),
                (actual.engine_pitch, expected.engine_pitch),
                (actual.engine_low_pass_hz, expected.engine_low_pass_hz),
            ] {
                assert!((actual - expected).abs() < 1e-4, "restored controller {actual} must equal control {expected}");
            }
            assert!(audio.missing.is_empty(), "no missing banks");
            assert!(control.missing.is_empty(), "no missing banks in control replay");
            if let Some(folder) = std::env::var_os("NFSMW_AUDIO_CAPTURE_DIR") {
                capture::write_float(
                    &std::path::PathBuf::from(folder).join(format!("speedbreaker-engine-{rate}-{fps}.wav")),
                    &recorded,
                    rate,
                    2,
                );
            }
        }
    }
}
