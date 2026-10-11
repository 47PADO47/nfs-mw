use super::*;
use crate::rigid_body::RigidBodySpec;

fn body() -> RigidBody {
    RigidBody::new(1000.0, Vec3::ONE, Vec3::ONE, RigidBodySpec::default())
}

fn limit(mode: LimitMode) -> AngularLimit {
    AngularLimit {
        frame_a: Quat::from_rotation_x(match mode {
            LimitMode::Conical => -std::f32::consts::FRAC_PI_2,
            LimitMode::Prismatic => -std::f32::consts::PI,
        }),
        vector_b: match mode {
            LimitMode::Conical => Vec3::Y * 0.406,
            LimitMode::Prismatic => Vec3::NEG_Z * 0.406,
        },
        positive: 0.2,
        negative: 0.2,
        mode,
    }
}

#[test]
fn angle_decisions_match_measured_pc_frames_for_pitch_roll_and_yaw() {
    let a = body();
    let cases = [
        (LimitMode::Prismatic, Quat::from_rotation_y(0.3), 0.09989324, Quat::from_xyzw(-1.0, 0.0, 0.0, 0.0)),
        (LimitMode::Prismatic, Quat::from_rotation_y(-0.3), -0.09989324, Quat::from_xyzw(-1.0, 0.0, 0.0, 0.0)),
        (
            LimitMode::Conical,
            Quat::from_rotation_x(0.3),
            -0.11302795,
            Quat::from_xyzw(-0.41969684, -0.5690822, -0.41969684, 0.5690822),
        ),
        (
            LimitMode::Conical,
            Quat::from_rotation_z(0.3),
            -0.09989324,
            Quat::from_xyzw(0.0, -0.70710677, -0.70710677, 0.0),
        ),
    ];
    for (mode, orientation, expected_delta, expected_frame) in cases {
        let mut b = body();
        b.set_orientation(orientation);
        let (frame, delta) = limit(mode).exceeded(&a, &b).unwrap();
        // PC atan2 is a lookup approximation; std atan2 differs by about 0.00011 radians here.
        assert!((delta - expected_delta).abs() < 0.0002, "{mode:?}: {delta}");
        assert!(frame.abs_diff_eq(expected_frame, 0.0001) || frame.abs_diff_eq(-expected_frame, 0.0001), "{frame:?}");
    }
}

#[test]
fn yaw_is_free_in_the_cone_and_pitch_is_free_in_the_prismatic_constraint() {
    let a = body();
    let mut b = body();
    b.set_orientation(Quat::from_rotation_y(1.0));
    assert!(limit(LimitMode::Conical).exceeded(&a, &b).is_none());
    b.set_orientation(Quat::from_rotation_x(0.8));
    assert!(limit(LimitMode::Prismatic).exceeded(&a, &b).is_none());
}

#[test]
fn correction_keeps_the_hitch_closed_and_does_not_rotate_an_immobile_partner() {
    let joint = BallJoint { anchor_a: Vec3::ZERO, anchor_b: Vec3::ZERO };
    let mut a = body();
    let mut b = body();
    a.state = BodyState::Frozen;
    b.set_orientation(Quat::from_rotation_y(0.7));
    let rule = limit(LimitMode::Prismatic);
    rule.solve(&joint, &mut a, &mut b);
    assert_eq!(a.orientation, Quat::IDENTITY);
    assert!(joint.gap(&a, &b) < 0.00001);
    assert!(rule.exceeded(&a, &b).is_none_or(|(_, delta)| delta.abs() < 0.00001));
}
