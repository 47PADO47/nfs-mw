use chameleon_sound_graph::{Curve, Fade, Group, MixerItem, Modulator, Point};

use super::*;

const NO_FADE: Fade = Fade { shape: 1.0, span: 0.0 };

/// A point whose tangents make the segments straight lines.
fn point(x: f32, y: f32) -> Point {
    Point { x, y, left: [1.0 / 3.0, 1.0 / 3.0], right: [1.0 / 3.0, 1.0 / 3.0] }
}

fn curve(points: &[(f32, f32)]) -> Curve {
    Curve { points: points.iter().map(|&(x, y)| point(x, y)).collect() }
}

fn wave_item(lo: f32, hi: f32, pitch: Modulator, tracks_driver: bool) -> Item {
    let wave = WaveItem {
        waves: vec![7],
        mixer: 70,
        gain: Modulator { base: 2.0, ..Modulator::default() },
        pitch,
        tracks_driver,
        random: false,
    };
    Item { lo, hi, fade_in: NO_FADE, fade_out: NO_FADE, kind: ItemKind::Wave(wave) }
}

fn wave_of(item: &Item) -> &WaveItem {
    let ItemKind::Wave(wave) = &item.kind else { panic!("not a wave") };
    wave
}

/// The pitch of the electric car's first motor loop: 1 at a standstill to 2.2 at the top, over the item's window.
fn motor_pitch() -> Modulator {
    Modulator { output: (1.0, 2.2), curve: curve(&[(0.0, 0.0), (0.394, 0.8), (1.0, 0.991)]), ..Modulator::default() }
}

fn points_of(fields: &str) -> Vec<(f64, f64)> {
    let table: toml::Table = toml::from_str(&format!("v = {fields}")).unwrap();
    table["v"]["points"]
        .as_array()
        .unwrap()
        .iter()
        .map(|point| (point[0].as_float().unwrap(), point[1].as_float().unwrap()))
        .collect()
}

#[test]
fn a_motor_loop_plays_at_a_rate_that_is_the_pitch_curve_over_its_window() {
    let item = wave_item(0.0, 150.0, motor_pitch(), false);
    let fields = pitch_fields(&item, wave_of(&item));
    let curve = fields.strip_prefix(", rate = ").expect("a rate curve");
    assert!(curve.contains("input = \"speed_mph\""));
    let points = points_of(curve);
    assert_eq!(points[0], (0.0, 1.0), "a standing car plays the loop at its own speed");
    let (x, y) = *points.last().unwrap();
    assert_eq!(x, 150.0);
    assert!((y - (1.0 + 1.2 * 0.991)).abs() < 1e-3, "{y}");
    assert!(points.windows(2).all(|pair| pair[1].1 >= pair[0].1), "the motor only rises: {points:?}");
}

#[test]
fn a_whine_loop_keeps_its_reference_speed_and_a_loop_with_no_pitch_curve_has_no_rate() {
    let tracking = Modulator { base: 40.0, ..Modulator::default() };
    let whine = wave_item(20.0, 90.0, tracking, true);
    assert_eq!(pitch_fields(&whine, wave_of(&whine)), ", pitch = { input = \"speed_mph\", reference = 40.0 }");
    let flat = wave_item(0.0, 150.0, Modulator { output: (0.9, 2.0), ..Modulator::default() }, false);
    assert_eq!(pitch_fields(&flat, wave_of(&flat)), "");
    let named = Modulator { expression: "x.GetSomething".into(), ..motor_pitch() };
    let item = wave_item(0.0, 150.0, named, false);
    assert_eq!(pitch_fields(&item, wave_of(&item)), "", "a pitch read from another live value is not the speed's");
}

#[test]
fn a_mixer_item_of_the_loops_channel_shapes_its_gain_over_the_items_window() {
    let swell = Modulator { curve: curve(&[(0.0, 0.0), (0.5, 1.0), (1.0, 0.0)]), ..Modulator::default() };
    let mixer = Item {
        lo: 5.0,
        hi: 150.0,
        fade_in: NO_FADE,
        fade_out: NO_FADE,
        kind: ItemKind::Mixer(MixerItem { channel: 70, gains: vec![swell] }),
    };
    let sequence =
        Sequence { id: 1, driver: "x.GetTransmissionSpeedMph".into(), lo: 0.0, hi: 170.0, items: vec![mixer] };
    let points = points_of(&channel_curve(&sequence, 70).expect("a curve for the channel"));
    assert_eq!((points[0], *points.last().unwrap()), ((5.0, 0.0), (150.0, 0.0)));
    let peak = points.iter().copied().fold((0.0, 0.0), |best, p| if p.1 > best.1 { p } else { best });
    assert!((peak.0 - 77.5).abs() < 1e-3 && (peak.1 - 1.0).abs() < 1e-3, "{peak:?}");
    assert!(channel_curve(&sequence, 71).is_none(), "another channel is not shaped by it");
    assert!(channel_curve(&sequence, 0).is_none());
}

fn speed_sequence(id: u64, driver: &str, waves: usize) -> Sequence {
    let items = (0..waves).map(|_| wave_item(20.0, 90.0, Modulator::default(), false)).collect();
    Sequence { id, driver: driver.into(), lo: 0.0, hi: 170.0, items }
}

fn group(class: &str, id: u64, handles: &[u64]) -> Group {
    Group { class: class.into(), id, handles: handles.to_vec() }
}

#[test]
fn the_motor_is_the_players_and_the_whine_the_transmission_objects_and_the_opponents_are_left_out() {
    let speed = "RaceCars.Entities[\"Id\"].GetTransmissionSpeedMph";
    let graphs = SoundGraphs {
        sequences: vec![
            speed_sequence(10, "Players.LocalPlayer.GetPlayerEngineLoad", 0),
            speed_sequence(11, speed, 5),
            speed_sequence(12, speed, 2),
            speed_sequence(13, speed, 2),
        ],
        groups: vec![
            group("EngineSound2", 1, &[10, 12]),
            group("TransmissionWhine", 2, &[11]),
            group("EngineSound2", 3, &[13]),
        ],
        ..SoundGraphs::default()
    };
    let found: Vec<(Role, u64)> = sequences(&graphs).into_iter().map(|(role, sequence)| (role, sequence.id)).collect();
    assert_eq!(found, [(Role::Motor, 12), (Role::Whine, 11)], "the second engine sound (the opponents') plays nothing");
}

#[test]
fn a_bundle_without_the_gathering_objects_reads_every_speed_driven_sequence_of_several_items_as_a_whine() {
    let speed = "RaceCars.Entities[\"Id\"].GetTransmissionSpeedMph";
    let graphs = SoundGraphs {
        sequences: vec![speed_sequence(1, speed, 5), speed_sequence(2, speed, 1)],
        ..SoundGraphs::default()
    };
    let found: Vec<(Role, u64)> = sequences(&graphs).into_iter().map(|(role, sequence)| (role, sequence.id)).collect();
    assert_eq!(found, [(Role::Whine, 1)]);
}

#[test]
fn the_whine_is_scaled_down_and_the_motor_is_not() {
    assert_eq!(Role::Motor.balance(), 1.0);
    assert!(Role::Whine.balance() < 0.5);
}
