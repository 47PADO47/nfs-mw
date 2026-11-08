use super::*;

fn anchor(translation: [f32; 3]) -> Anchor {
    Anchor { bone: None, translation, rotation: [0.0, 0.0, 0.0, 1.0] }
}

fn mounts() -> Mounts {
    Mounts { frame: Frame { centre: 1.0, lift: 0.5 }, front_hub: [1.0, 0.3, 0.1], rear_hub: [-1.0, 0.3, 0.0] }
}

#[test]
fn the_pieces_take_the_exhaust_pad_rims_and_discs_out_of_the_other_solids() {
    let ids = apart_ids(0b1111_1111);
    assert_eq!(ids.len(), 9);
    for id in [
        "exmanifold",
        "muffler",
        "barpad",
        "front_rim",
        "back_rim",
        "front_brake",
        "back_brake",
        "mirrors",
        "homologation",
    ] {
        assert!(ids.contains(&id), "{id}");
    }
    assert!(!ids.contains(&"chassis") && !ids.contains(&"front_tyre"));
}

#[test]
fn a_point_is_turned_into_the_frame_of_a_piece() {
    let m = mounts();
    assert_eq!(m.to(At::Body, [1.0, 2.0, 3.0]), [0.0, -3.0, 2.5]);
    assert_eq!(m.to(At::FrontHub, [1.0, 0.3, 0.1]), [0.0; 3]);
    assert_eq!(m.to(At::RearHub, [-1.0, 0.8, 0.0]), [0.0, 0.0, 0.5]);
}

#[test]
fn the_matrix_of_a_mount_takes_a_solid_to_the_bike() {
    let m = mounts();
    // The body frame: the solid holds (x, -z, y); the bike moves it by the frame's centre and lift.
    assert_eq!(m.matrix(At::Body, &anchor([0.0; 3])), [1.0, 0.0, 0.0, -1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.5]);
    // An anchor of 0.2 along the export x moves the solid 0.2 forward.
    let moved = m.matrix(At::Body, &anchor([0.2, 0.0, 0.0]));
    assert!((moved[3] - (-0.8)).abs() < 1e-6 && moved[7] == 0.0);
    // A hub frame: the front hub is the origin.
    let hub = m.matrix(At::FrontHub, &anchor([0.0; 3]));
    let expected = [1.0, 0.0, 0.0, -1.0, 0.0, 1.0, 0.0, 0.1, 0.0, 0.0, 1.0, -0.3];
    assert!(hub.iter().zip(expected).all(|(got, want)| (got - want).abs() < 1e-6), "{hub:?}");
}

#[test]
fn a_bike_has_the_pieces_its_part_table_lists() {
    let json = r#"{"schema": 1, "vehicle": "x", "parts": [
        {"id": "muffler", "role": "body", "export": "m", "anchor": {"bone": null, "translation": [0,0,0], "rotation": [0,0,0,1]}, "materials": {}},
        {"id": "front_rim", "role": "front_wheel", "export": "r", "anchor": {"bone": null, "translation": [0,0,0], "rotation": [0,0,0,1]}, "materials": {}}
    ]}"#;
    let table = PartTable::parse(json).unwrap();
    let tags: Vec<&str> = present(&table).iter().map(|piece| piece.tag).collect();
    assert_eq!(tags, ["EXHAUST", "FRONT_RIM"]);
}
