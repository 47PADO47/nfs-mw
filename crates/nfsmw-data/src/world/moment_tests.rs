use super::*;

fn data_with(kind_of_scene: (u32, u32), markers: &[(&str, Marker)]) -> MomentData {
    let mut data = MomentData::default();
    data.kinds.insert(kind_of_scene.0, kind_of_scene.1);
    for (name, marker) in markers {
        data.markers.entry(bstring_hash(name)).or_default().push(*marker);
    }
    data
}

fn marker(x: f32, y: f32, angle: u16) -> Marker {
    Marker { position: Vec3::new(x, y, 0.0), angle }
}

#[test]
fn the_nearest_marker_of_the_scene_kind_wins() {
    let data = data_with(
        (7, 1),
        &[
            ("GasStationMoment", marker(100.0, 0.0, 0)),
            ("GasStationMoment", marker(30.0, 0.0, 0)),
            ("FirehallMoment", marker(1.0, 0.0, 0)),
        ],
    );
    let (found, score) = data.closest_marker(7, Vec3::ZERO, 0).unwrap();
    assert_eq!(found.position.x, 30.0, "other kinds do not count");
    assert_eq!(score, 30.0);
}

#[test]
fn a_marker_facing_away_costs_up_to_twelve_and_a_half_metres() {
    let data = data_with((7, 1), &[("GasStationMoment", marker(10.0, 0.0, 32768))]);
    assert_eq!(data.closest_marker(7, Vec3::ZERO, 0).unwrap().1, 22.5);
    let aligned = data_with((7, 1), &[("GasStationMoment", marker(10.0, 0.0, 100))]);
    assert_eq!(aligned.closest_marker(7, Vec3::ZERO, 100).unwrap().1, 10.0);
}

#[test]
fn the_turn_between_angles_takes_the_short_way_round() {
    assert_eq!(turn(65000, 536), 1072.0 / 65536.0);
    assert_eq!(turn(536, 65000), 1072.0 / 65536.0);
    assert_eq!(turn(0, 32768), 0.5);
}

#[test]
fn a_facing_marker_a_little_farther_can_beat_a_nearer_one_facing_away() {
    let data = data_with(
        (7, 1),
        &[("GasStationMoment", marker(10.0, 0.0, 32768)), ("GasStationMoment", marker(20.0, 0.0, 0))],
    );
    assert_eq!(data.closest_marker(7, Vec3::ZERO, 0).unwrap().0.position.x, 20.0);
}

#[test]
fn unknown_scenes_and_scenes_without_markers_have_none() {
    let data = data_with((7, 1), &[]);
    assert!(data.closest_marker(7, Vec3::ZERO, 0).is_none());
    assert!(data.closest_marker(8, Vec3::ZERO, 0).is_none());
    let bad_kind = data_with((7, 99), &[("GasStationMoment", marker(0.0, 0.0, 0))]);
    assert!(bad_kind.closest_marker(7, Vec3::ZERO, 0).is_none());
}

#[test]
fn the_candidate_table_lists_scenes_by_marker_kind() {
    let mut table = vec![0u8; MOMENT_LIST + 16];
    table[MOMENT_COUNT..MOMENT_COUNT + 4].copy_from_slice(&2u32.to_le_bytes());
    for (i, (scene, kind)) in [(0xAAAA_0001u32, 1u32), (0xAAAA_0002, 4)].into_iter().enumerate() {
        table[MOMENT_LIST + 8 * i..][..4].copy_from_slice(&scene.to_le_bytes());
        table[MOMENT_LIST + 8 * i + 4..][..4].copy_from_slice(&kind.to_le_bytes());
    }
    let mut data = MomentData::default();
    data.read_kinds(&table).unwrap();
    assert_eq!(data.kind_of(0xAAAA_0001), Some(1));
    assert_eq!(data.kind_of(0xAAAA_0002), Some(4));
    assert_eq!(data.kind_of(3), None);
    assert!(MomentData::default().read_kinds(&[0; 8]).is_err());
}

#[test]
fn markers_are_read_by_name_hash_with_their_position_and_angle() {
    let mut record = vec![0u8; MARKER_LEN * 2];
    record[8..12].copy_from_slice(&bstring_hash("WaterTowerMoment").to_le_bytes());
    for (i, v) in [1.5f32, -2.5, 3.5].into_iter().enumerate() {
        record[0x10 + 4 * i..][..4].copy_from_slice(&v.to_le_bytes());
    }
    record[0x20..0x22].copy_from_slice(&16384u16.to_le_bytes());
    record[MARKER_LEN + 8..MARKER_LEN + 12].copy_from_slice(&0x1234u32.to_le_bytes());
    let mut data = MomentData::default();
    data.read_markers(&record);
    let got = &data.markers[&bstring_hash("WaterTowerMoment")];
    assert_eq!(got, &[Marker { position: Vec3::new(1.5, -2.5, 3.5), angle: 16384 }]);
    assert_eq!(data.markers.len(), 1, "a record with another name is skipped");
}

#[test]
fn the_marker_names_are_the_twenty_one_kinds() {
    assert_eq!(MARKER_NAMES.len(), 21);
    assert_eq!(MARKER_NAMES[1], "GasStationMoment");
    assert_eq!(MARKER_NAMES[20], "StripMallMoment");
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn installed_moment_scenes_have_markers_and_a_camera_track() {
    let Some(path) = std::env::var_os("NFSMW_GAME_DIR") else { return };
    let dir = GameDir::open(path).unwrap();
    let data = load(&dir).unwrap();
    for (name, kind) in [
        ("WMGasStation", 1),
        ("WMWaterTower", 4),
        ("WMPorch", 5),
        ("WMBoat", 10),
        ("WMDriveIn", 16),
        ("WMTrailerPark", 17),
        ("WMAmphitheatre", 19),
        ("WMStripMall", 20),
    ] {
        let hash = scene_hash(name);
        assert_eq!(data.kind_of(hash), Some(kind), "{name}");
        let markers = data.markers.get(&bstring_hash(MARKER_NAMES[kind as usize])).map_or(0, Vec::len);
        assert!(markers > 0, "{name}: no {} markers", MARKER_NAMES[kind as usize]);
        let track = data.track(hash, 0).unwrap_or_else(|| panic!("{name}: no camera track"));
        assert_eq!(track.keys.len(), 1, "{name}");
        assert!((2.5..=4.0).contains(&track.length), "{name}: {} s", track.length);
        let first = track.sample(0.0).unwrap();
        let last = track.sample(1.0).unwrap();
        assert_ne!(first.eye, last.eye, "{name}: the camera moves");
        eprintln!(
            "{name}: {markers} markers, {:.2} s, lens {:.1}, sim speed {:.2}, letterbox {:.3}",
            track.length, first.lens, first.sim_speed, first.letterbox
        );
    }
    assert_eq!(data.markers.get(&bstring_hash("GasStationMoment")).map(Vec::len), Some(9), "nine gas stations");
}
