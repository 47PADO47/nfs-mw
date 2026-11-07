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
    assert!((mixed[0].rate - 2.0).abs() < 1e-5, "{:?}", mixed[0]);
    assert!((mixed[1].rate - 0.5).abs() < 1e-5, "the next layer is slowed to the engine's pitch: {:?}", mixed[1]);
}

#[test]
fn below_the_lowest_and_above_the_highest_layer_one_layer_carries_the_sound() {
    assert_eq!(gains(300.0), [1.0, 0.0, 0.0]);
    assert_eq!(gains(7000.0), [0.0, 0.0, 1.0]);
    let low = mix(&sound(), 300.0);
    assert!((low[0].rate - RATE_RANGE.0).abs() < 1e-5, "slowed, but only so far: {:?}", low[0]);
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
        assert!(
            mixed
                .iter()
                .filter(|layer| layer.gain > 0.0)
                .all(|layer| (RATE_RANGE.0..=RATE_RANGE.1).contains(&layer.rate))
        );
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

/// The engine speed a layer sounds like: its own `rpm` times its playback rate (the engine's rpm unless the rate is held).
fn sounds_like(layer: &EngineLayer, mixed: &mixing::LayerMix) -> f32 {
    layer.rpm * mixed.rate
}

#[test]
fn across_a_gap_of_four_times_between_two_layers_the_sound_keeps_following_the_engine() {
    // The engine model of a 2008 superbike: an idle recording at 1588 rpm, then no loop until 6443.
    let gap = EngineSound {
        idle_rpm: 1000.0,
        redline_rpm: 14500.0,
        layers: vec![EngineLayer::classic("IDLE", 1588.0), EngineLayer::classic("LOW", 6443.0)],
    };
    let mut rpm = 1000.0;
    while rpm <= 14500.0 {
        let mixed = mix(&gap, rpm);
        let weight: f32 = mixed.iter().map(|layer| layer.gain * layer.gain).sum();
        // The pitch of each layer, weighted by the power it carries, is the engine's.
        let pitch_error: f32 = mixed
            .iter()
            .zip(&gap.layers)
            .map(|(m, layer)| m.gain * m.gain * (1200.0 * (sounds_like(layer, m) / rpm).log2()).abs())
            .sum::<f32>()
            / weight;
        assert!(pitch_error < 30.0, "{rpm} rpm: {pitch_error} cents off");
        rpm += 100.0;
    }
}

fn with_off_load() -> EngineSound {
    let layer =
        |rpm: f32, off: bool| EngineLayer { off_load: off, ..EngineLayer::classic(format!("S/{rpm}{off}"), rpm) };
    EngineSound {
        idle_rpm: 800.0,
        redline_rpm: 7000.0,
        layers: vec![
            layer(800.0, false),
            layer(800.0, true),
            layer(3200.0, false),
            layer(3200.0, true),
            layer(6400.0, false),
            layer(6400.0, true),
        ],
    }
}

/// The power of the layers under load and of those off load, at an engine speed fraction and a throttle.
fn set_powers(fraction: f32, load: f32) -> (f32, f32) {
    let engine = with_off_load();
    let mixed = mix_inputs(&engine, &EngineInputs { rpm: fraction, load, speed_mph: 0.0 });
    let power = |off: bool| -> f32 {
        engine.layers.iter().zip(&mixed).filter(|(layer, _)| layer.off_load == off).map(|(_, m)| m.gain * m.gain).sum()
    };
    (power(false), power(true))
}

#[test]
fn the_throttle_crossfades_the_layers_off_load_into_those_under_load_at_equal_power() {
    for fraction in [0.0, 0.2, 0.5, 0.9] {
        let (closed_on, closed_off) = set_powers(fraction, 0.0);
        assert!(closed_on < 1e-6 && (closed_off - 1.0).abs() < 1e-5, "closed: {closed_on} {closed_off}");
        let (dead_on, dead_off) = set_powers(fraction, mixing::OFF_LOAD_DEAD_ZONE);
        assert!(dead_on < 1e-6 && (dead_off - 1.0).abs() < 1e-5, "the dead zone of the pedal: {dead_on} {dead_off}");
        let (open_on, open_off) = set_powers(fraction, 1.0);
        assert!((open_on - 1.0).abs() < 1e-5 && open_off < 1e-6, "open: {open_on} {open_off}");
        let mut last = -1.0;
        for step in 0..=20 {
            let (on, off) = set_powers(fraction, step as f32 / 20.0);
            assert!((on + off - 1.0).abs() < 1e-4, "load {step}/20: {on} + {off}");
            assert!(on >= last, "the layers under load only get louder with the throttle");
            last = on;
        }
        let (half_on, half_off) = set_powers(fraction, 0.12 + 0.88 * 0.5);
        assert!((half_on - 0.5).abs() < 1e-4 && (half_off - 0.5).abs() < 1e-4, "half way: {half_on} {half_off}");
    }
}

#[test]
fn each_set_is_mixed_by_engine_speed_on_its_own_and_both_pitch_to_the_engine() {
    let engine = with_off_load();
    let inputs = EngineInputs { rpm: 0.2, load: 0.5, speed_mph: 0.0 };
    let rpm = engine.rpm_at(inputs.rpm);
    let mixed = mix_inputs(&engine, &inputs);
    // 0.2 of the way is 2040 rpm: between the layers of 800 and 3200 of each set.
    let sounding: Vec<usize> = (0..6).filter(|&at| mixed[at].gain > 0.0).collect();
    assert_eq!(sounding, [0, 1, 2, 3], "the two layers of each set that bracket the engine speed");
    for at in sounding {
        assert!((mixed[at].rate - rpm / engine.layers[at].rpm).abs() < 1e-5, "layer {at}: {:?}", mixed[at]);
    }
    // The two sets have the same shape: layer 0 and 1 (800 rpm, under load and off load) play at the same rate.
    assert_eq!(mixed[0].rate, mixed[1].rate);
    assert_eq!(mixed[2].rate, mixed[3].rate);
}

#[test]
fn an_engine_without_layers_off_load_plays_the_same_whatever_the_throttle() {
    let engine = sound();
    let at = |load: f32| mix_inputs(&engine, &EngineInputs { rpm: 0.4, load, speed_mph: 0.0 });
    assert_eq!(at(0.0), at(1.0));
    assert_eq!(at(0.5), mix(&engine, engine.rpm_at(0.4)));
}
