use super::*;
use crate::{InputKey, Spatial, SpatialRecord};

/// A rolloff with camera-distance and camera-azimuth from 10 m to 110 m on shape 8 (linear down) in every
/// quadrant, for the camera state `camera`.
fn record(camera: u32, near: u32, far: u32) -> SpatialRecord {
    SpatialRecord { info: 0x9000_0000 | (camera << 24), curves: 0x8888_0000, ranges: [(far << 16) | near; 4] }
}

fn state(records: Vec<SpatialRecord>) -> State {
    let id = SourceId(0x8000_0070 | ((records.len() as u32) << 24));
    State {
        spatial: vec![Spatial { id, records }],
        masters: vec![master(
            1,
            OutputKind::Volume,
            vec![spatial_ref(0, 0)],
            vec![
                PresetWord { slot: 2, spatial: 0, azimuth: false, offset: 0 },
                PresetWord { slot: 0, spatial: 0, azimuth: true, offset: 0 },
            ],
        )],
        ..State::default()
    }
}

fn publish(m: &mut Mixer, to_camera_cm: i32, azimuth: i32, flags: i32) {
    for (index, value) in [(1u8, to_camera_cm), (3, azimuth), (15, flags)] {
        m.set_input(InputKey::controller(0, 0, 7, index), value);
    }
}

#[test]
fn the_rolloff_falls_with_the_distance() {
    let mut m = mixer(vec![(0, state(vec![record(0, 10, 110)]))], &[1]);
    let mut last = f32::MAX;
    for cm in [500, 1000, 3000, 6000, 9000, 11_000] {
        publish(&mut m, cm, 0, 1);
        run(&mut m, 1);
        let gain = m.volume(obj(0, 0, 1), 2).unwrap();
        assert!(gain <= last, "{cm}: {gain} > {last}");
        last = gain;
    }
    assert!(last < 0.02, "{last}");
    // 60 m of a 10 to 110 m range on a linear curve: half the amplitude.
    publish(&mut m, 6000, 0, 1);
    run(&mut m, 1);
    let half = m.volume(obj(0, 0, 1), 2).unwrap();
    assert!((half - 0.5).abs() < 0.03, "{half}");
}

#[test]
fn beyond_the_far_distance_it_is_silent_and_near_it_is_unity() {
    let mut m = mixer(vec![(0, state(vec![record(0, 10, 110)]))], &[1]);
    publish(&mut m, 20_000, 0, 1);
    run(&mut m, 1);
    assert_eq!(m.volume(obj(0, 0, 1), 2), Some(0.0));
    publish(&mut m, 100, 0, 1);
    run(&mut m, 1);
    assert!(m.volume(obj(0, 0, 1), 2).unwrap() > 0.99);
}

#[test]
fn a_control_without_a_position_is_silent() {
    let mut m = mixer(vec![(0, state(vec![record(0, 10, 110)]))], &[1]);
    publish(&mut m, 100, 0, 0);
    run(&mut m, 1);
    assert_eq!(m.volume(obj(0, 0, 1), 2), Some(0.0));
    assert_eq!(m.graph.spatial_db, vec![-10_000]);
}

#[test]
fn the_azimuth_is_passed_through_to_an_azimuth_word() {
    let mut m = mixer(vec![(0, state(vec![record(0, 10, 110)]))], &[1]);
    publish(&mut m, 100, 0x4321, 1);
    run(&mut m, 1);
    assert_eq!(m.azimuth(obj(0, 0, 1), 0), Some(0x4321));
}

#[test]
fn the_azimuth_blends_the_neighbouring_quadrant() {
    // Quadrant 0 reaches 10..110 m, quadrant 1 only 10..60 m: leaving quadrant 0 the rolloff drops faster.
    let mut s =
        state(vec![SpatialRecord { info: 0x9000_0000, curves: 0x8888_0000, ranges: [0x006E_000A, 0x003C_000A, 0, 0] }]);
    s.spatial[0].records[0].ranges[2] = 0x006E_000A;
    s.spatial[0].records[0].ranges[3] = 0x006E_000A;
    let mut m = mixer(vec![(0, s)], &[1]);
    publish(&mut m, 5000, 0, 1);
    run(&mut m, 1);
    let ahead = m.volume(obj(0, 0, 1), 2).unwrap();
    publish(&mut m, 5000, 0x3FFF, 1);
    run(&mut m, 1);
    let side = m.volume(obj(0, 0, 1), 2).unwrap();
    assert!(side < ahead, "{side} {ahead}");
}

#[test]
fn the_camera_state_picks_the_record() {
    let near = record(0, 10, 110);
    let jump = record(3, 10, 30);
    let mut m = mixer(vec![(0, state(vec![near, jump]))], &[1]);
    publish(&mut m, 2000, 0, 1);
    run(&mut m, 1);
    let default = m.volume(obj(0, 0, 1), 2).unwrap();
    m.set_camera(3);
    run(&mut m, 1);
    let jump_gain = m.volume(obj(0, 0, 1), 2).unwrap();
    assert!(jump_gain < default, "{jump_gain} {default}");
    // A camera state without a record falls back to the default one.
    m.set_camera(2);
    run(&mut m, 1);
    assert_eq!(m.volume(obj(0, 0, 1), 2), Some(default));
}
