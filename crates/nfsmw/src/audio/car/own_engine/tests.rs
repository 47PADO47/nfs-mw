use super::*;
use nfsmw_data::car::mods::EngineLayer;

fn sound() -> EngineSound {
    let layer = |rpm: f32| EngineLayer::classic(format!("SOUND/{rpm}.WAV"), rpm);
    EngineSound { idle_rpm: 800.0, redline_rpm: 7000.0, layers: vec![layer(800.0), layer(3200.0), layer(6400.0)] }
}

fn gains(rpm: f32) -> Vec<f32> {
    mix(&sound(), rpm).iter().map(|layer| layer.gain).collect()
}

#[test]
fn at_a_layers_own_rpm_only_that_layer_plays_at_its_natural_pitch() {
    let mixed = mix(&sound(), 3200.0);
    assert!((mixed[1].gain - 1.0).abs() < 1e-5 && mixed[0].gain.abs() < 1e-5 && mixed[2].gain == 0.0);
    assert!((mixed[1].rate - 1.0).abs() < 1e-5);
}

#[test]
fn between_two_layers_the_power_stays_constant_and_each_is_pitched_to_the_engine() {
    for rpm in [1000.0, 1600.0, 2400.0, 4500.0, 6000.0] {
        let g = gains(rpm);
        let power: f32 = g.iter().map(|gain| gain * gain).sum();
        assert!((power - 1.0).abs() < 1e-4, "{rpm} rpm: {g:?}");
    }
    let mixed = mix(&sound(), 1600.0);
    assert!((mixed[0].rate - 2.0f32.min(RATE_RANGE.1)).abs() < 1e-5, "{:?}", mixed[0]);
    assert!((mixed[1].rate - 0.55).abs() < 1e-5, "the next layer is slowed, but only so far: {:?}", mixed[1]);
}

#[test]
fn below_the_lowest_and_above_the_highest_layer_one_layer_carries_the_sound() {
    assert_eq!(gains(300.0), [1.0, 0.0, 0.0]);
    assert_eq!(gains(7000.0), [0.0, 0.0, 1.0]);
    let low = mix(&sound(), 300.0);
    assert!((low[0].rate - 0.55).abs() < 1e-5, "{:?}", low[0]);
    assert!((mix(&sound(), 7000.0)[2].rate - 7000.0 / 6400.0).abs() < 1e-5);
}

#[test]
fn a_single_layer_follows_the_whole_range() {
    let one = EngineSound { idle_rpm: 800.0, redline_rpm: 7000.0, layers: sound().layers[..1].to_vec() };
    assert_eq!(mix(&one, 4000.0)[0].gain, 1.0);
    assert!((mix(&one, 1000.0)[0].rate - 1.25).abs() < 1e-5);
}

#[test]
fn rpm_runs_from_idle_to_redline_over_the_fraction() {
    let sound = sound();
    assert_eq!((sound.rpm_at(0.0), sound.rpm_at(1.0), sound.rpm_at(0.5)), (800.0, 7000.0, 3900.0));
    assert_eq!(sound.rpm_at(2.0), 7000.0);
}

fn wav(channels: u16, samples: &[i16]) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut bytes = b"RIFF".to_vec();
    bytes.extend((36 + data_len).to_le_bytes());
    bytes.extend(b"WAVEfmt ");
    bytes.extend(16u32.to_le_bytes());
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(channels.to_le_bytes());
    bytes.extend(48_000u32.to_le_bytes());
    bytes.extend((48_000 * u32::from(channels) * 2).to_le_bytes());
    bytes.extend((channels * 2).to_le_bytes());
    bytes.extend(16u16.to_le_bytes());
    bytes.extend(b"data");
    bytes.extend(data_len.to_le_bytes());
    bytes.extend(samples.iter().flat_map(|sample| sample.to_le_bytes()));
    bytes
}

#[test]
fn a_wav_loops_over_its_whole_length() {
    let pcm = parse_wav(&wav(1, &[1, -2, 3, -4])).unwrap();
    assert_eq!(
        (pcm.sample_rate, pcm.channels, pcm.samples.clone(), pcm.loop_range),
        (48_000, 1, vec![1, -2, 3, -4], Some((0, 4)))
    );
    assert_eq!(parse_wav(&wav(2, &[1, 2, 3, 4])).unwrap().loop_range, Some((0, 2)));
}

#[test]
fn files_that_are_not_16_bit_pcm_wav_are_refused() {
    assert!(parse_wav(b"RIFFxxxxAVI ").is_err());
    let mut float = wav(1, &[0, 0]);
    float[20] = 3;
    assert!(parse_wav(&float).is_err());
    assert!(parse_wav(&wav(1, &[])).is_err());
    assert!(parse_wav(&wav(6, &[0; 6])).is_err());
}

#[test]
fn the_built_in_engine_has_loops_from_idle_up_that_the_mixer_can_blend() {
    let engine = built_in();
    assert!(engine.idle_rpm < engine.redline_rpm);
    let rpms: Vec<f32> = engine.layers.iter().map(|layer| layer.rpm).collect();
    assert_eq!(rpms[0], engine.idle_rpm, "the lowest loop is the idle");
    let spacing_ok = rpms.windows(2).all(|pair| pair[0] < pair[1] && pair[1] / pair[0] < 2.0);
    assert!(spacing_ok, "{rpms:?}: sorted, never an octave apart");
    assert!(*rpms.last().unwrap() <= engine.redline_rpm);
    assert!(engine.layers.iter().all(|layer| layer.file == BUILT_IN_SOURCE), "no layer is a file of any car");
    for fraction in [0.0, 0.3, 0.7, 1.0] {
        let mixed = mix(&engine, engine.rpm_at(fraction));
        let power: f32 = mixed.iter().map(|layer| layer.gain * layer.gain).sum();
        assert!((power - 1.0).abs() < 1e-4, "{fraction}: {mixed:?}");
        assert!(mixed.iter().filter(|layer| layer.gain > 0.0).all(|layer| (0.55..=1.8).contains(&layer.rate)));
    }
}

#[test]
fn a_built_in_loop_loops_over_its_whole_length_at_its_own_rate() {
    for rpm in BUILT_IN_LAYER_RPMS {
        let pcm = built_in_loop(rpm);
        assert_eq!((pcm.sample_rate, pcm.channels), (BUILT_IN_RATE, 1));
        assert_eq!(pcm.loop_range, Some((0, pcm.samples.len() as u32)));
        assert!(pcm.samples.len() as u32 >= BUILT_IN_RATE / 2 - 1, "{rpm} rpm: at least half a second");
        assert!(pcm.samples.iter().any(|&sample| sample.abs() > 3000), "{rpm} rpm: audible");
    }
}
