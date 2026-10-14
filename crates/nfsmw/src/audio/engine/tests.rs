use blackbox_ginsu::GinsuTables;

use super::*;

const RATE: u32 = 32_000;

/// A loop whose recording is a sine rising from 100 to 400 Hz, like a rev sweep, with the tables built
/// from it: cycle starts at every zero turn and 50 frequency segments.
fn data() -> Arc<GinsuData> {
    let n = (RATE as usize) * 2;
    let (mut pcm, mut phase) = (Vec::with_capacity(n), 0.0f32);
    let (mut cycle_pos, mut freq_at) = (vec![0u32], Vec::with_capacity(n));
    for i in 0..n {
        let hz = 100.0 + 300.0 * i as f32 / n as f32;
        let before = phase;
        phase += hz / RATE as f32;
        if phase.floor() > before.floor() {
            cycle_pos.push(i as u32);
        }
        freq_at.push(hz * 120.0);
        pcm.push((phase * std::f32::consts::TAU).sin() * 0.5);
    }
    let (lo, hi) = (freq_at[0], *freq_at.last().unwrap());
    let freq_pos = (0..=50)
        .map(|s| {
            let f = lo + (hi - lo) * s as f32 / 50.0;
            freq_at.iter().position(|&x| x >= f).unwrap_or(n - 1) as u32
        })
        .collect();
    let tables = GinsuTables::new(lo, hi, RATE, n as u32, freq_pos, cycle_pos).expect("tables");
    Arc::new(GinsuData::new(tables, pcm).expect("data"))
}

fn render(voice: EngineVoice, mixes: &[(EngineMix, usize)], out_rate: f64) -> Vec<f32> {
    let (mut sound, handle) = voice.into_sound().unwrap();
    let info = kira::info::MockInfoBuilder::new().build();
    let mut out = Vec::new();
    for &(mix, frames) in mixes {
        handle.set(mix);
        let mut block = vec![Frame::ZERO; frames];
        sound.process(&mut block, 1.0 / out_rate, &info);
        out.extend(block.iter().map(|f| f.left));
    }
    out
}

fn rising_zero_crossings_hz(samples: &[f32], rate: f64) -> f64 {
    let crossings = samples.windows(2).filter(|w| w[0] <= 0.0 && w[1] > 0.0).count();
    crossings as f64 / (samples.len() as f64 / rate)
}

fn mix(hz: f32, pitch: f32) -> EngineMix {
    EngineMix::shared(hz * 120.0, pitch, 1.0, 0.0)
}

#[test]
fn spatial_engine_control_preserves_centre_energy_and_applies_dry_gain() {
    let sample = |azimuth, dry| {
        let voice = EngineVoice { accel: data(), decel: None, start: EngineMix::default() };
        let (mut sound, handle) = voice.into_sound().unwrap();
        let mut mix = mix(200.0, 1.0);
        mix.azimuth = azimuth;
        mix.dry = dry;
        handle.set(mix);
        let mut out = vec![Frame::ZERO; 48000];
        sound.process(&mut out, 1.0 / 48000.0, &kira::info::MockInfoBuilder::new().build());
        out[24000..].iter().fold([0.0f64; 2], |mut energy, frame| {
            energy[0] += f64::from(frame.left).powi(2);
            energy[1] += f64::from(frame.right).powi(2);
            energy
        })
    };
    let centre = sample(0, 1.0);
    let right = sample(16384, 1.0);
    let dry = sample(0, 0.5);
    assert!(centre[0] > 0.01 && (centre[0] - centre[1]).abs() < 1e-9);
    assert_eq!(right[0], 0.0);
    assert!((right[1] / (centre[0] + centre[1]) - 1.0).abs() < 0.001);
    assert!((dry[0] / centre[0] - 0.25).abs() < 0.001);
}

#[test]
fn the_pitch_follows_the_requested_frequency_at_the_device_rate() {
    let d = data();
    for hz in [150.0f32, 250.0, 350.0] {
        let voice = EngineVoice { accel: d.clone(), decel: None, start: EngineMix::default() };
        let samples = render(voice, &[(mix(hz, 1.0), 48_000)], 48_000.0);
        let measured = rising_zero_crossings_hz(&samples[24_000..], 48_000.0);
        assert!((measured - f64::from(hz)).abs() < f64::from(hz) * 0.1, "asked {hz} Hz, heard {measured:.1} Hz");
    }
}

#[test]
fn a_playback_rate_shifts_the_pitch() {
    let voice = EngineVoice { accel: data(), decel: None, start: EngineMix::default() };
    let samples = render(voice, &[(mix(200.0, 1.5), 48_000)], 48_000.0);
    let measured = rising_zero_crossings_hz(&samples[24_000..], 48_000.0);
    assert!((measured - 300.0).abs() < 30.0, "{measured}");
}

#[test]
fn each_loop_follows_its_own_drive() {
    let d = data();
    let drive = |accel_hz: f32, decel_hz: f32, accel_vol: f32, decel_vol: f32| EngineMix {
        accel: LoopMix { frequency: accel_hz * 120.0, volume: accel_vol, pitch: 1.0 },
        decel: LoopMix { frequency: decel_hz * 120.0, volume: decel_vol, pitch: 1.0 },
        ..Default::default()
    };
    // Only the decelerate loop is audible: its pitch is the one asked of it, not the accelerate loop's.
    let voice = EngineVoice { accel: d.clone(), decel: Some(d.clone()), start: EngineMix::default() };
    let samples = render(voice, &[(drive(150.0, 350.0, 0.0, 1.0), 48_000)], 48_000.0);
    let measured = rising_zero_crossings_hz(&samples[24_000..], 48_000.0);
    assert!((measured - 350.0).abs() < 35.0, "heard {measured:.1} Hz");
    // And the accelerate loop alone plays its own.
    let voice = EngineVoice { accel: d.clone(), decel: Some(d), start: EngineMix::default() };
    let samples = render(voice, &[(drive(150.0, 350.0, 1.0, 0.0), 48_000)], 48_000.0);
    let measured = rising_zero_crossings_hz(&samples[24_000..], 48_000.0);
    assert!((measured - 150.0).abs() < 15.0, "heard {measured:.1} Hz");
}

#[test]
fn a_loop_pitch_is_its_own() {
    let d = data();
    let mix = EngineMix {
        accel: LoopMix { frequency: 200.0 * 120.0, volume: 0.0, pitch: 1.0 },
        decel: LoopMix { frequency: 200.0 * 120.0, volume: 1.0, pitch: 1.5 },
        ..Default::default()
    };
    let voice = EngineVoice { accel: d.clone(), decel: Some(d), start: EngineMix::default() };
    let samples = render(voice, &[(mix, 48_000)], 48_000.0);
    let measured = rising_zero_crossings_hz(&samples[24_000..], 48_000.0);
    assert!((measured - 300.0).abs() < 30.0, "heard {measured:.1} Hz");
}

#[test]
fn zero_volume_is_silence_and_bad_numbers_do_not_poison_the_output() {
    let d = data();
    let voice = EngineVoice { accel: d.clone(), decel: Some(d), start: EngineMix::default() };
    let quiet = EngineMix::shared(200.0 * 120.0, 1.0, 0.0, 0.0);
    let bad = EngineMix::shared(f32::NAN, f32::NAN, f32::INFINITY, -3.0);
    let out = render(voice, &[(quiet, 8_000), (bad, 8_000)], 48_000.0);
    assert!(out[2_000..8_000].iter().all(|s| s.abs() < 1e-3), "silent at zero volume");
    assert!(out.iter().all(|s| s.is_finite() && s.abs() <= 8.0));
}

#[test]
fn dropping_the_handle_finishes_the_voice() {
    let voice = EngineVoice { accel: data(), decel: None, start: EngineMix::default() };
    let (mut sound, handle) = voice.into_sound().unwrap();
    assert!(!sound.finished());
    drop(handle);
    assert!(!sound.finished(), "a changed car releases without an abrupt cut");
    let info = kira::info::MockInfoBuilder::new().build();
    sound.process(&mut [Frame::ZERO; 6400], 1.0 / 48_000.0, &info);
    assert!(sound.finished());
}

#[test]
fn a_stalled_callback_resumes_at_the_latest_request_without_permanent_backlog() {
    for rate in [44_100.0, 48_000.0, 96_000.0] {
        for block in [32, 128, 1024] {
            let voice = EngineVoice { accel: data(), decel: None, start: mix(150.0, 1.0) };
            let (mut sound, handle) = voice.into_sound().unwrap();
            assert!(handle.current_frequency() > 0.0);
            for revision in 1..=600 {
                handle.set_tick(mix(150.0, 1.0), revision);
            }
            handle.set_tick(mix(350.0, 1.5), 601);
            let info = kira::info::MockInfoBuilder::new().build();
            let mut frames = vec![Frame::ZERO; block];
            for _ in 0..(rate as usize * 2).div_ceil(block) {
                sound.process(&mut frames, 1.0 / rate, &info);
                assert!(frames.iter().all(|f| f.left.is_finite()));
            }
            let actual = handle.current_frequency();
            assert!((actual - 350.0 * 120.0).abs() < 1000.0, "actual synthesis must catch up: {actual}");
            assert!(actual < 400.0 * 120.0, "dynamic playback pitch is separate from feedback");
        }
    }
}
