use chameleon_sound_graph::{
    Curve, Fade, Item, ItemKind, MixerItem, Modulator, Point, Sequence, SoundGraphs, WaveItem,
};

use super::*;

const HARD: Fade = Fade { shape: 1.0, span: 0.0 };

fn point(x: f32, y: f32) -> Point {
    Point { x, y, left: [1.0, 0.0], right: [1.0, 0.0] }
}

fn wave(sample: u64, mixer: u64, lo: f32, hi: f32, fade: f32, gain: Modulator, rate: f32) -> Item {
    Item {
        lo,
        hi,
        fade_in: Fade { shape: 1.0, span: fade },
        fade_out: Fade { shape: 1.0, span: fade },
        kind: ItemKind::Wave(WaveItem {
            waves: vec![sample],
            mixer,
            gain,
            pitch: Modulator { base: rate, ..Modulator::default() },
            tracks_driver: false,
            random: false,
        }),
    }
}

fn mixer_item(channel: u64, lo: f32, hi: f32, gain: Modulator) -> Item {
    Item { lo, hi, fade_in: HARD, fade_out: HARD, kind: ItemKind::Mixer(MixerItem { channel, gains: vec![gain] }) }
}

fn constant(base: f32) -> Modulator {
    Modulator { base, ..Modulator::default() }
}

fn graphs(items: Vec<Item>, channels: &[(u64, f32)]) -> SoundGraphs {
    let sequence = Sequence { id: 1, driver: "LocalPlayer.GetPlayerSpeed".into(), lo: 0.0, hi: 180.0, items };
    SoundGraphs { sequences: vec![sequence], channel_gains: channels.iter().copied().collect(), ..Default::default() }
}

fn level_at(layer: &WindLayer, speed: f32) -> f32 {
    let at = layer.gain.iter().position(|point| point.0 >= speed).expect("the speed is in the curve");
    layer.gain[at].1
}

#[test]
fn a_loop_keeps_its_window_its_fades_its_rate_and_the_gain_of_its_channel() {
    let items = vec![wave(7, 2, 45.0, 90.0, 20.0, constant(1.5), 1.3)];
    let set = build(&graphs(items, &[(2, 0.2)])).unwrap();
    assert_eq!(set.layers.len(), 1);
    let layer = &set.layers[0];
    assert_eq!((layer.sample, layer.volume, layer.rate), (7, 0.3, 1.3));
    assert_eq!(level_at(layer, 30.0), 0.0);
    assert!((level_at(layer, 55.0) - 0.5).abs() < 1e-3, "halfway up the 20 mph fade");
    assert_eq!(level_at(layer, 70.0), 1.0);
    assert!((level_at(layer, 80.0) - 0.5).abs() < 1e-3);
    assert_eq!(level_at(layer, 100.0), 0.0);
}

#[test]
fn a_loop_in_the_air_and_one_on_a_muted_channel_are_not_written() {
    let in_air = Modulator { expression: "RaceCarEntities.RaceCar[].TimeInAir".into(), ..Modulator::default() };
    let items = vec![
        wave(1, 2, 45.0, 250.0, 0.0, in_air, 1.0),
        wave(3, 4, 45.0, 250.0, 0.0, constant(1.0), 1.0),
        wave(5, 6, 45.0, 250.0, 0.0, constant(1.0), 1.0),
    ];
    let set = build(&graphs(items, &[(2, 0.5), (4, 0.0001), (6, 0.35)])).unwrap();
    assert_eq!((set.in_air, set.muted), (1, 1));
    assert_eq!(set.layers.iter().map(|layer| layer.sample).collect::<Vec<_>>(), [5]);
}

#[test]
fn the_speed_is_clamped_to_the_range_of_the_sequence_so_no_point_lies_beyond_it() {
    let items = vec![wave(7, 2, 100.0, 250.0, 40.0, constant(1.0), 2.0)];
    let set = build(&graphs(items, &[(2, 1.0)])).unwrap();
    let layer = &set.layers[0];
    assert_eq!(layer.gain.last().map(|point| point.0), Some(180.0));
    assert_eq!(layer.gain.last().map(|point| point.1), Some(1.0), "the fade-in is over 40 mph above 100");
    assert!(layer.gain.len() <= 64 && layer.gain.windows(2).all(|pair| pair[0].0 < pair[1].0));
    assert_eq!(layer.rate, 2.0);
}

#[test]
fn a_mixer_item_of_the_channel_scales_the_loop_by_its_curve_along_its_own_window() {
    let swell = Modulator {
        curve: Curve { points: vec![point(0.0, 0.0), point(0.5, 1.0), point(1.0, 1.0)] },
        ..Modulator::default()
    };
    let items = vec![mixer_item(2, 40.0, 140.0, swell), wave(7, 2, 40.0, 140.0, 0.0, constant(1.0), 1.0)];
    let layer = build(&graphs(items, &[(2, 1.0)])).unwrap().layers.remove(0);
    assert!(level_at(&layer, 45.0) < 0.2, "the curve starts at nothing at 40 mph");
    assert!(level_at(&layer, 100.0) > 0.99, "and is full from halfway across the window");
}

#[test]
fn a_gain_curve_with_its_own_input_range_reads_the_speed_not_the_window() {
    let over_speed = Modulator {
        input: (70.0, 200.0),
        curve: Curve { points: vec![point(0.0, 0.0), point(1.0, 1.0)] },
        ..Modulator::default()
    };
    let items = vec![wave(7, 2, 45.0, 250.0, 0.0, over_speed, 1.0)];
    let layer = build(&graphs(items, &[(2, 1.0)])).unwrap().layers.remove(0);
    assert_eq!(level_at(&layer, 70.0), 0.0);
    assert!(level_at(&layer, 100.0) < 0.1, "the curve is still low a quarter of the way along 70 to 200 mph");
    assert!(level_at(&layer, 180.0) > 0.9);
}

#[test]
fn a_bundle_without_the_wind_builds_nothing() {
    assert!(build(&SoundGraphs::default()).is_none());
}

#[test]
fn the_profile_lists_each_loop_by_a_file_named_for_its_sample() {
    assert_eq!(file_name(0x0100_0000_0018_2159), "W182159.WAV");
    let set = WindSet {
        layers: vec![WindLayer { sample: 0x18_2159, volume: 0.25, rate: 1.0, gain: vec![(0.0, 0.0), (45.0, 1.0)] }],
        ..WindSet::default()
    };
    let text = profile_text(&set);
    assert!(text.starts_with("format = 1\n"));
    assert!(text.contains("[[layer]]\nfile = \"WIND/MW2012/W182159.WAV\"\nvolume = 0.25\nrate = 1.0\n"));
    assert!(text.contains("gain = [[0.0, 0.0], [45.0, 1.0]]"));
}

#[test]
fn only_loops_that_decoded_are_kept() {
    let layer = |sample| WindLayer { sample, volume: 1.0, rate: 1.0, gain: vec![(0.0, 0.0), (1.0, 1.0)] };
    let samples = vec![
        Sample { id: 1, samples: vec![0.0; 4], rate: 8000, looped: true },
        Sample { id: 2, samples: vec![0.0; 4], rate: 8000, looped: false },
    ];
    let kept = playable(vec![layer(1), layer(2), layer(3)], &samples);
    assert_eq!(kept.iter().map(|layer| layer.sample).collect::<Vec<_>>(), [1]);
}
