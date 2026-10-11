use super::*;

fn box_at(at: Vec3, half: Vec3, yaw: f32) -> Obb {
    Obb { centre: at, axes: Mat3::from_rotation_y(yaw), half }
}

fn check(hit: ObbContact, point: Vec3, normal: Vec3, depth: f32, first: bool) {
    assert!((hit.point - point).length() < 0.000002, "{hit:?}");
    assert!((hit.normal - normal).length() < 0.000002, "{hit:?}");
    assert!((hit.overlap - depth).abs() < 0.000002, "{hit:?}");
    assert_eq!(hit.point_on_first, first);
}

#[test]
fn original_face_contact_does_not_invent_a_corner_lever() {
    let a = box_at(Vec3::ZERO, Vec3::new(1.0, 1.0, 2.0), 0.0);
    let b = box_at(Vec3::Z * 3.8, a.half, 0.0);
    check(obb_contact(&a, &b).unwrap(), Vec3::Z * 1.8, Vec3::NEG_Z, 0.2, false);
    let b = box_at(Vec3::Z * 2.2, Vec3::new(0.2, 0.7, 0.3), 0.0);
    check(obb_contact(&a, &b).unwrap(), Vec3::Z * 2.0, Vec3::NEG_Z, 0.1, true);
}

#[test]
fn original_travel_selects_the_approach_face_after_crossing_a_thin_box() {
    let a = box_at(Vec3::Z * 5.0, Vec3::new(1.0, 1.0, 2.0), 0.0);
    let b = box_at(Vec3::ZERO, Vec3::new(0.2, 0.7, 0.3), 0.0);
    assert!(obb_contact(&a, &b).is_none());
    check(box_contact(&a, &b, Vec3::Z * 8.0, Vec3::ZERO).unwrap(), Vec3::NEG_Z * 0.3, Vec3::NEG_Z, 7.3, false);
    let reversed = box_contact(&b, &a, Vec3::ZERO, Vec3::Z * 8.0).unwrap();
    // The first reference face wins a tie: reversing the pair can select the other incident surface.
    check(reversed, Vec3::Z * 7.0, Vec3::Z, 7.3, false);
}

#[test]
fn original_face_ties_and_corner_rejection_are_deterministic() {
    let a = box_at(Vec3::ZERO, Vec3::new(1.0, 1.0, 2.0), 0.0);
    let b = box_at(Vec3::ZERO, Vec3::splat(0.2), 0.0);
    check(obb_contact(&a, &b).unwrap(), Vec3::X * 0.2, Vec3::X, 1.2, false);
    let b = box_at(Vec3::new(1.8, 0.2, 3.8), a.half, 0.0);
    assert!(obb_contact(&a, &b).is_none(), "the original rejects this unsupported corner contact");
    let b = box_at(Vec3::Z * 4.0, a.half, 0.0);
    check(obb_contact(&a, &b).unwrap(), Vec3::Z * 2.0, Vec3::NEG_Z, 0.0, false);
}

#[test]
fn original_rotated_boxes_clip_the_contact_to_the_incident_edge() {
    let a = box_at(Vec3::ZERO, Vec3::new(1.0, 1.0, 2.0), 0.6);
    let b = box_at(Vec3::new(1.0, 0.0, 3.0), a.half, -0.3);
    check(
        obb_contact(&a, &b).unwrap(),
        Vec3::new(0.6357039, 0.0, 0.7938067),
        Vec3::new(-0.5646425, 0.0, -0.8253356),
        0.98589754,
        false,
    );
}

#[test]
fn degenerate_flat_box_and_invalid_geometry_stay_finite() {
    let a = box_at(Vec3::ZERO, Vec3::new(1.0, 1.0, 2.0), 0.0);
    let mut b = box_at(Vec3::Y * 0.5, Vec3::new(4.0, 0.0, 4.0), 0.0);
    check(obb_contact(&a, &b).unwrap(), Vec3::Y * 0.5, Vec3::NEG_Y, 0.5, false);
    b.half.x = -1.0;
    assert!(obb_contact(&a, &b).is_none());
    assert!(box_contact(&a, &a, Vec3::NAN, Vec3::ZERO).is_none());
}

#[test]
fn tumbling_contact_clips_side_planes_in_original_order() {
    let a = Obb {
        centre: Vec3::new(-0.4349601, 0.5805687, -1.6567545),
        half: Vec3::new(2.9804342, 1.8778094, 0.28748396),
        axes: Mat3::from_cols_array(&[
            0.9999499,
            0.0097035235,
            -0.0024635475,
            -0.009736218,
            0.99985975,
            -0.013625182,
            0.0023309896,
            0.013648485,
            0.99990416,
        ]),
    };
    let b = Obb {
        centre: Vec3::new(0.022836482, 0.30564183, -1.1432469),
        half: Vec3::new(0.6697936, 1.7833066, 1.4865518),
        axes: Mat3::from_cols_array(&[
            0.9091657,
            0.19720657,
            -0.3667797,
            -0.085727595,
            0.9505292,
            0.29857165,
            0.4075151,
            -0.24000795,
            0.8810946,
        ]),
    };
    let hit = box_contact(
        &a,
        &b,
        Vec3::new(-2.5845082, 1.0649492, -2.1952977),
        Vec3::new(-0.30940586, -2.6750226, -1.8567322),
    )
    .unwrap();
    check(
        hit,
        Vec3::new(-1.2805693, 2.4540033, -1.3928437),
        Vec3::new(0.085727595, -0.9505292, -0.29857165),
        3.8626022,
        true,
    );
}
