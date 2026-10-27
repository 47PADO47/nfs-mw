use super::*;

/// Harmonics of `pitch` (1, 2 and 3 times it) at one second long.
fn tone(pitch: f32, rate: u32) -> Vec<f32> {
    (0..rate as usize)
        .map(|i| {
            let t = i as f32 / rate as f32;
            (0..3)
                .map(|h| (2.0 * std::f32::consts::PI * pitch * (h + 1) as f32 * t).sin() / (h + 1) as f32)
                .sum::<f32>()
                * 0.3
        })
        .collect()
}

fn looped(pitch: f32) -> Loop {
    let samples = tone(pitch, 48_000);
    Loop { pitch: pitch_of(&samples, 48_000), samples, rate: 48_000 }
}

#[test]
fn the_pitch_of_a_harmonic_tone_is_its_fundamental() {
    for pitch in [41.0, 120.0, 255.0, 510.0] {
        let found = pitch_of(&tone(pitch, 48_000), 48_000);
        assert!((found - pitch).abs() / pitch < 0.03, "{pitch} Hz found as {found}");
    }
}

#[test]
fn silence_and_noise_have_no_pitch() {
    assert_eq!(pitch_of(&vec![0.0; 48_000], 48_000), 0.0);
    let mut state = 12_345u32;
    let noise: Vec<f32> = (0..48_000)
        .map(|_| {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (state >> 8) as f32 / (1u32 << 24) as f32 - 0.5
        })
        .collect();
    assert_eq!(pitch_of(&noise, 48_000), 0.0);
}

#[test]
fn layers_are_slowest_first_with_the_rpm_their_pitch_stands_for() {
    let layers = layers(vec![looped(170.0), looped(41.0), looped(85.0)], 800.0, 7500.0);
    let rpms: Vec<f32> = layers.iter().map(|layer| layer.rpm).collect();
    assert_eq!(rpms.len(), 3);
    assert!((rpms[0] - 800.0).abs() < 1.0, "{rpms:?}");
    assert!((rpms[1] / 800.0 - 85.0 / 41.0).abs() < 0.1, "{rpms:?}");
    assert!((rpms[2] / 800.0 - 170.0 / 41.0).abs() < 0.2, "{rpms:?}");
}

#[test]
fn a_layer_the_engine_never_reaches_and_a_repeated_pitch_are_left_out() {
    let layers = layers(vec![looped(41.0), looped(41.5), looped(510.0)], 800.0, 7500.0);
    assert_eq!(layers.len(), 1, "510 Hz is {}x the idle", 510.0 / 41.0);
}

#[test]
fn a_car_without_a_pitched_loop_has_no_layers() {
    let dull = Loop { samples: vec![0.0; 48_000], rate: 48_000, pitch: 0.0 };
    assert!(layers(vec![dull], 800.0, 7500.0).is_empty());
}

#[test]
fn a_layer_is_a_16_bit_mono_wav_at_a_common_level() {
    let layer = &layers(vec![looped(41.0)], 800.0, 7500.0)[0];
    assert_eq!(&layer.wav[..4], b"RIFF");
    assert_eq!(u16::from_le_bytes([layer.wav[22], layer.wav[23]]), 1);
    assert_eq!(u32::from_le_bytes([layer.wav[24], layer.wav[25], layer.wav[26], layer.wav[27]]), 48_000);
    let samples: Vec<f32> =
        layer.wav[44..].as_chunks::<2>().0.iter().map(|b| i16::from_le_bytes(*b) as f32 / 32768.0).collect();
    let rms = (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt();
    assert!((rms - TARGET_RMS).abs() < 0.01, "{rms}");
}
