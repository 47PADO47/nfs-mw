use super::*;
use crate::ground::FlatGround;
use crate::rigid_body::{ContactParams, FrictionState, PlaneContact, RigidBodySpec};
use glam::Quat;

#[test]
fn plane_projection_rejects_the_original_near_tangent_cancellation_control() {
    let mut body = RigidBody::new(1000.0, Vec3::new(1.0, 1.0, 2.0), Vec3::ONE, RigidBodySpec::default());
    let velocity = Vec3::new(14.0, -0.000001, -13.125);
    body.linear_velocity = velocity;
    let plane = PlaneContact {
        point: Vec3::ZERO,
        normal: Vec3::new(0.6, 0.48, 0.64),
        friction_static: 0.0,
        friction_kinetic: 0.0,
    };
    let params = ContactParams { restitution: 0.0, inertia_scale: Vec3::ONE };
    assert!(body.react_plane(&plane, &params).is_none());
    assert_eq!(body.linear_velocity, velocity);
    assert_eq!(body.angular_velocity, Vec3::ZERO);
}

#[test]
fn support_points_at_or_above_the_ceiling_still_query_the_original_minimum_tolerance() {
    struct Recorder(std::cell::Cell<Option<(Vec3, f32)>>);
    impl Ground for Recorder {
        fn hit(&self, origin: Vec3, _: Vec3, distance: f32) -> Option<crate::ground::GroundHit> {
            self.0.set(Some((origin, distance)));
            None
        }
    }
    let body = RigidBody::new(1000.0, Vec3::ONE, Vec3::ONE, RigidBodySpec::default());
    for point in [Vec3::Y, Vec3::Y * 2.0] {
        let ground = Recorder(std::cell::Cell::new(None));
        body.ground_contact(point, &ground, 1.0 / 60.0);
        let (origin, distance) = ground.0.get().expect("original still makes a query above the ceiling");
        assert_eq!(distance, 0.25);
        assert_eq!(origin, point + Vec3::Y * 0.25);
    }
}

#[test]
fn plane_response_matches_measured_centred_sliding_and_offcentre_pc_vectors() {
    let cases = [
        (
            Vec3::ZERO,
            Vec3::new(0.0, -12.0, 0.0),
            Vec3::ZERO,
            Vec3::new(0.0, 2.3999834, 0.0),
            Vec3::ZERO,
            11999.986,
            FrictionState::Static,
        ),
        (
            Vec3::NEG_Y,
            Vec3::new(30.0, -12.0, 0.0),
            Vec3::ZERO,
            Vec3::new(25.200005, 2.3999834, 0.0),
            Vec3::new(0.0, 0.0, -7.1999917),
            11999.986,
            FrictionState::Dynamic,
        ),
        (
            Vec3::new(0.8, -1.0, 1.7),
            Vec3::new(3.0, -12.0, 8.0),
            Vec3::new(0.5, 0.7, 0.2),
            Vec3::new(2.0830956, -7.0669065, 6.635_33),
            Vec3::new(-3.7129536, 0.4197987, 4.7443547),
            4110.911,
            FrictionState::Dynamic,
        ),
    ];
    for (point, velocity, angular, expected_v, expected_w, impulse, friction) in cases {
        let mut body = RigidBody::new(1000.0, Vec3::new(1.0, 1.0, 2.0), Vec3::ONE, RigidBodySpec::default());
        body.linear_velocity = velocity;
        body.angular_velocity = angular;
        let plane = PlaneContact { point, normal: Vec3::Y, friction_static: 0.6, friction_kinetic: 0.4 };
        let reaction = body.react_plane(&plane, &ContactParams { restitution: 0.2, inertia_scale: Vec3::ONE }).unwrap();
        assert!((body.linear_velocity - expected_v).abs().max_element() < 0.00005);
        assert!((body.angular_velocity - expected_w).abs().max_element() < 0.00005);
        assert!((reaction.impulse - impulse).abs() < 0.02);
        assert_eq!(reaction.friction, friction);
    }
}

#[test]
fn deep_ground_queries_recover_a_body_that_passed_the_old_fixed_lift() {
    let mut body = RigidBody::new(1000.0, Vec3::ONE, Vec3::ONE, RigidBodySpec::default());
    body.place(Vec3::Y * 0.1, Quat::IDENTITY);
    body.linear_velocity = Vec3::NEG_Y * 80.0;
    let contact = body.ground_contact(body.position - Vec3::Y, &FlatGround::new(0.0), 1.0 / 60.0).unwrap();
    assert!((contact.depth - 0.9).abs() < 0.00001);
    body.resolve_ground(&mut [contact]);
    assert!((body.position.y - 1.0).abs() < 0.00001);
    assert!(body.linear_velocity.y > 0.0);
}

#[test]
fn disabled_ground_integrates_without_support() {
    let spec = RigidBodySpec { no_ground_collisions: true, ..Default::default() };
    let mut body = RigidBody::new(1.0, Vec3::ONE, Vec3::ONE, spec);
    body.begin_frame(1.0 / 60.0);
    body.begin_frame(1.0 / 60.0);
    assert!(body.position.y < 0.0);
    assert!(body.ground_contact(Vec3::NEG_Y, &FlatGround::new(0.0), 1.0 / 60.0).is_none());
}

#[test]
fn surface_friction_changes_body_sliding_without_retuning_its_spec() {
    let body = RigidBody::new(1000.0, Vec3::ONE, Vec3::ONE, RigidBodySpec::default());
    let collide = |friction| {
        let mut body = body.clone();
        body.linear_velocity = Vec3::new(30.0, -12.0, 0.0);
        body.resolve_ground(&mut [GroundContact {
            point: Vec3::ZERO,
            normal: Vec3::Y,
            depth: 0.1,
            friction,
            ..Default::default()
        }]);
        assert_eq!(body.spec().ground_friction, [0.8, 0.6]);
        body.linear_velocity.x
    };
    assert!(collide(0.2) > collide(1.0));
}

#[test]
fn ground_callback_preserves_normal_speed_and_incoming_linear_velocity() {
    let mut body = RigidBody::new(1000.0, Vec3::ONE, Vec3::ONE, RigidBodySpec::default());
    let incoming = Vec3::new(30.0, -0.5, 0.0);
    body.linear_velocity = incoming;
    let mut contacts = [GroundContact {
        point: Vec3::ZERO,
        normal: Vec3::Y,
        depth: 0.1,
        friction: 1.0,
        velocity: incoming,
        ..Default::default()
    }];
    body.resolve_ground(&mut contacts);
    let callback = contacts[0];
    assert!(callback.impulse_delta_v > 0.0, "a shallow scrape still resolves physically");
    assert!((callback.closing_speed - 0.5).abs() < 1e-6);
    assert!(callback.velocity.length() > 30.0);
    assert_eq!(callback.before_velocity, incoming);
    assert_ne!(body.linear_velocity, callback.before_velocity);
}

#[test]
fn ground_callback_retains_the_original_local_vertex_after_penetration_correction() {
    let mut body = RigidBody::new(1000.0, Vec3::ONE, Vec3::ONE, RigidBodySpec::default());
    body.place(Vec3::Y * 0.9, Quat::from_rotation_y(0.5));
    body.linear_velocity = Vec3::NEG_Y * 2.0;
    let local = Vec3::new(0.8, -1.0, 0.5);
    let point = body.position + body.rotation() * local;
    let contact = body.ground_contact(point, &FlatGround::new(0.0), 1.0 / 60.0).unwrap();
    let mut contacts = [contact];
    body.resolve_ground(&mut contacts);
    assert!(contacts[0].impulse_delta_v > 0.0);
    assert!((contacts[0].local_arm - local).length() < 1e-6);
    let reconstructed_after_push = body.rotation().transpose() * (contacts[0].point - body.position);
    assert!((reconstructed_after_push - local).length() > 0.09, "ground support must move the body");
}
