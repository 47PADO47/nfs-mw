use chameleon_sound_graph::{Fade, GinsuItem, Modulator, WaveItem};

use super::*;

fn item(lo: f32, hi: f32, fade_in: (f32, f32), fade_out: (f32, f32)) -> Item {
    Item {
        lo,
        hi,
        fade_in: Fade { shape: fade_in.0, span: fade_in.1 },
        fade_out: Fade { shape: fade_out.0, span: fade_out.1 },
        kind: ItemKind::Other(String::new()),
    }
}

fn sequence(lo: f32, hi: f32) -> Sequence {
    Sequence { id: 1, driver: "x.GetPlayerAudibleRPM".into(), lo, hi, items: Vec::new() }
}

fn points_of(curve: &str) -> Vec<(f64, f64)> {
    let table: toml::Table = toml::from_str(&format!("v = {curve}")).unwrap();
    table["v"]["points"]
        .as_array()
        .unwrap()
        .iter()
        .map(|point| (point[0].as_float().unwrap(), point[1].as_float().unwrap()))
        .collect()
}

#[test]
fn the_idle_loop_window_fades_out_over_its_whole_span_as_a_fraction_of_the_range() {
    let idle = item(1790.0, 2290.0, (1.0, 0.0), (0.5, 500.0));
    let curve = window_curve(&idle, &sequence(1790.0, 7500.0), Domain::Rpm { lo: 1790.0, hi: 7500.0 }).unwrap();
    assert!(curve.contains("input = \"rpm\""));
    let points = points_of(&curve);
    assert_eq!(points[0], (0.0, 1.0), "full at idle: {points:?}");
    let end = points.last().unwrap();
    assert!((end.0 - 500.0 / 5710.0).abs() < 1e-3 && end.1 == 0.0, "gone 500 rpm up: {points:?}");
    assert!(points.windows(2).all(|pair| pair[1].0 > pair[0].0 && pair[1].1 <= pair[0].1), "{points:?}");
}

#[test]
fn a_ginsu_voice_that_fades_in_at_the_floor_rises_from_silence_and_holds_to_the_top() {
    let voice = item(1790.0, 7500.0, (0.5, 100.0), (1.0, 0.0));
    let curve = window_curve(&voice, &sequence(1790.0, 7500.0), Domain::Rpm { lo: 1790.0, hi: 7500.0 }).unwrap();
    let points = points_of(&curve);
    assert_eq!(points[0], (0.0, 0.0));
    assert_eq!(*points.last().unwrap(), (1.0, 1.0));
    assert!(points.windows(2).all(|pair| pair[1].1 >= pair[0].1), "{points:?}");
}

#[test]
fn a_whine_window_is_in_miles_per_hour_with_hard_edges_outside_its_fades() {
    let whine = item(20.0, 90.0, (1.0, 20.0), (1.0, 20.0));
    let curve = window_curve(&whine, &sequence(0.0, 170.0), Domain::Speed).unwrap();
    assert!(curve.contains("input = \"speed_mph\""));
    let points = points_of(&curve);
    let at = |x: f64| points.iter().find(|point| (point.0 - x).abs() < 1e-3).map(|point| point.1);
    assert_eq!((at(19.99), at(20.0), at(90.0), at(90.01)), (Some(0.0), Some(0.0), Some(0.0), Some(0.0)));
    assert!(points.iter().any(|point| point.1 == 1.0));
}

#[test]
fn points_are_sorted_rounded_clamped_and_made_to_rise() {
    let kept = normalise(vec![(0.5, 2.0), (0.0, -1.0), (0.5, 3.0), (f32::NAN, 1.0), (1.0, 20.0)]).unwrap();
    assert_eq!(kept, [(0.0, 0.0), (0.5, 2.0), (1.0, MAX_LEVEL)]);
    assert!(normalise(vec![(0.0, 1.0)]).is_none());
    assert!(normalise(vec![(0.0, 1.0), (0.00001, 1.0)]).is_none(), "two points that round together are one");
}

#[test]
fn a_layer_written_by_the_mod_is_accepted_by_the_games_manifest_parser() {
    let curve = curve_toml("rpm", &[(0.0, 0.0), (0.5, 1.0), (1.0, 1.0)]);
    let load = curve_toml("load", &[(0.0, 0.0), (1.0, 2.0)]);
    let ginsu = format!(
        "{{ file = \"SOUND/MODENGINE/X/L0.GIN\", kind = \"ginsu\", span = [1790.0, 7500.0], volume = 0.35, gain = [{curve}, {load}] }}"
    );
    let whine = format!(
        "{{ file = \"SOUND/MODENGINE/X/L1.WAV\", volume = 0.6, gain = [{}], pitch = {{ input = \"speed_mph\", reference = 40.0 }} }}",
        curve_toml("speed_mph", &[(20.0, 0.0), (40.0, 1.0)])
    );
    let text = format!(
        "role = \"player\"\nname = \"X\"\n[engine_sound]\nidle_rpm = 1000.0\nredline_rpm = 7800.0\nlayers = [\n  {ginsu},\n  {whine},\n]\n"
    );
    let manifest = blackbox_modding::content::parse_car("x", &text);
    let sound = manifest.unwrap().engine_sound.unwrap();
    assert_eq!(sound.layers.len(), 2);
    assert_eq!(sound.layers[0].span, Some((1790.0, 7500.0)));
    assert_eq!(sound.layers[1].pitch.unwrap().reference, 40.0);
}

fn engine_set(id: u64, lo: f32, items: Vec<Item>) -> Sequence {
    Sequence { id, driver: ENGINE_RPM.into(), lo, hi: 7500.0, items }
}

fn ginsu_item(lo: f32) -> Item {
    Item {
        lo,
        hi: 7500.0,
        fade_in: Fade { shape: 0.5, span: 100.0 },
        fade_out: Fade { shape: 1.0, span: 0.0 },
        kind: ItemKind::Ginsu(GinsuItem { resource: 1, mixer: 2 }),
    }
}

fn idle_item(lo: f32) -> Item {
    let wave = WaveItem {
        waves: vec![3],
        mixer: 4,
        gain: Modulator::default(),
        pitch: Modulator::default(),
        tracks_driver: false,
        random: false,
    };
    Item {
        lo,
        hi: lo + 500.0,
        fade_in: Fade { shape: 1.0, span: 0.0 },
        fade_out: Fade { shape: 0.5, span: 500.0 },
        kind: ItemKind::Wave(wave),
    }
}

#[test]
fn of_two_sets_the_one_that_sounds_at_idle_is_the_engine() {
    // The first set starts its items 250 rpm above the bottom of its range: silent at idle. The second does not.
    let silent = engine_set(1, 1400.0, vec![ginsu_item(1650.0), idle_item(1650.0)]);
    let sounding = engine_set(2, 1650.0, vec![ginsu_item(1650.0), idle_item(1650.0)]);
    let graphs = SoundGraphs { sequences: vec![silent.clone(), sounding.clone()], ..SoundGraphs::default() };
    assert_eq!(engine_sequence(&graphs).map(|sequence| sequence.id), Some(2));
    let both = SoundGraphs { sequences: vec![sounding, silent], ..SoundGraphs::default() };
    assert_eq!(engine_sequence(&both).map(|sequence| sequence.id), Some(2), "the first that sounds, in bundle order");
}

#[test]
fn with_no_set_that_sounds_the_first_is_kept_and_with_no_ginsu_there_is_none() {
    let a = engine_set(1, 1400.0, vec![ginsu_item(1650.0)]);
    let b = engine_set(2, 1400.0, vec![ginsu_item(1700.0)]);
    let graphs = SoundGraphs { sequences: vec![a, b], ..SoundGraphs::default() };
    assert_eq!(engine_sequence(&graphs).map(|sequence| sequence.id), Some(1));
    let none =
        SoundGraphs { sequences: vec![engine_set(3, 1400.0, vec![idle_item(1400.0)])], ..SoundGraphs::default() };
    assert!(engine_sequence(&none).is_none());
}
