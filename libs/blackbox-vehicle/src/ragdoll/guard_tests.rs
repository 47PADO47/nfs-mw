//! The guards of the ragdoll against the two ways a thrown rider was lost (`docs/specs/ride6-feel-round14.md`): through the world (a
//! wall, a step in the road) and out of it (a launch into the sky from an overlap or a wild impulse).

use std::f32::consts::FRAC_PI_2;

use glam::{Quat, Vec3};

use super::*;
use crate::ground::{GroundHit, WallHit};
use crate::{FIXED_STEP, FlatGround, SurfaceGrip};

const RADIUS: f32 = 0.08;
const HALF: f32 = 0.15;

/// A capsule lying along the z axis, its centre at `position`.
fn lying(position: Vec3) -> RagdollSpec {
    let segment = Segment {
        mass: 10.0,
        radius: RADIUS,
        half_length: HALF,
        position,
        orientation: Quat::from_rotation_x(FRAC_PI_2),
    };
    RagdollSpec { segments: vec![segment], ..RagdollSpec::default() }
}

/// A flat road with a wall across it at z = 10, facing the start (physics space).
struct Walled(FlatGround);

impl Ground for Walled {
    fn hit(&self, origin: Vec3, dir: Vec3, max_distance: f32) -> Option<GroundHit> {
        self.0.hit(origin, dir, max_distance)
    }

    fn wall(&self, from: Vec3, to: Vec3) -> Option<WallHit> {
        let across = from.z < 10.0 && to.z >= 10.0;
        let t = (10.0 - from.z) / (to.z - from.z);
        across.then(|| WallHit { point: from + (to - from) * t, normal: Vec3::NEG_Z, surface: 0 })
    }
}

/// A flat surface whose height the test changes between steps (a lift, a bridge deck being lowered); nothing under it. A ray that starts
/// under its top finds nothing, as the rays of the world do under a face.
struct Lift(std::cell::Cell<f32>);

impl Ground for Lift {
    fn hit(&self, origin: Vec3, dir: Vec3, max_distance: f32) -> Option<GroundHit> {
        let distance = (self.0.get() - origin.y) / dir.y;
        (dir.y < -1e-6 && distance >= 0.0 && distance <= max_distance).then_some(GroundHit {
            distance,
            normal: Vec3::Y,
            surface: SurfaceGrip::DEFAULT,
        })
    }
}

#[test]
fn a_body_that_starts_inside_the_ground_comes_out_of_it_without_a_launch() {
    // The centre is 25 cm under the road: the overlap of 33 cm used to be undone in the first sub-step and read as a velocity of
    // 300 m/s, which the solver then clipped to 150: the rider was launched into the sky.
    let ground = FlatGround::new(0.0);
    let mut body = Ragdoll::new(&lying(Vec3::new(0.0, -0.25, 0.0)), Vec3::ZERO, Vec3::ZERO);
    let mut fastest = 0.0_f32;
    for _ in 0..90 {
        body.step(FIXED_STEP, &ground);
        fastest = fastest.max(body.velocity(0).length());
    }
    assert!(fastest < 3.0, "the fastest it moved: {fastest} m/s");
    assert!((body.position(0).y - RADIUS).abs() < 0.03, "it lies on the road: {}", body.position(0).y);
}

#[test]
fn a_body_thrown_into_the_ground_at_speed_stops_at_it_and_does_not_bounce() {
    let ground = FlatGround::new(0.0);
    let mut body = Ragdoll::new(&lying(Vec3::new(0.0, 1.0, 0.0)), Vec3::new(0.0, -30.0, 20.0), Vec3::ZERO);
    let mut highest = 0.0_f32;
    for _ in 0..60 {
        body.step(FIXED_STEP, &ground);
        highest = highest.max(body.velocity(0).y);
    }
    assert!(highest < 6.0, "almost no restitution: it rose at {highest} m/s of 30");
}

#[test]
fn a_launch_is_held_to_what_a_bike_can_do_and_a_body_to_a_little_over_its_own_launch() {
    let ground = FlatGround::new(0.0);
    let mut wild =
        Ragdoll::new(&lying(Vec3::new(0.0, 1.0, 0.0)), Vec3::new(0.0, 0.0, 500.0), Vec3::new(0.0, 0.0, 900.0));
    assert!(wild.centre_velocity().length() <= body::MAX_SPEED + 1e-3, "{}", wild.centre_velocity());
    wild.step(FIXED_STEP, &ground);
    assert!(wild.velocity(0).length() <= body::MAX_SPEED + 1e-3);
    // A slow throw: kicked from outside, a segment still cannot go beyond the cap of its launch.
    let mut slow = Ragdoll::new(&lying(Vec3::new(0.0, 5.0, 0.0)), Vec3::new(0.0, 0.0, 3.0), Vec3::ZERO);
    slow.kick(0, Vec3::new(0.0, 80.0, 0.0), Vec3::ZERO);
    slow.step(FIXED_STEP, &FlatGround::new(-1000.0));
    assert!(slow.velocity(0).y <= 3.0 * CAP_RATIO + CAP_EXTRA + 1e-3, "{}", slow.velocity(0));
}

#[test]
fn a_fall_is_not_capped() {
    let mut body = Ragdoll::new(&lying(Vec3::new(0.0, 200.0, 0.0)), Vec3::ZERO, Vec3::ZERO);
    for _ in 0..180 {
        body.step(FIXED_STEP, &FlatGround::new(-1000.0));
    }
    assert!(body.velocity(0).y < -25.0, "three seconds of falling: {}", body.velocity(0).y);
}

#[test]
fn a_body_thrown_at_a_wall_at_bike_speed_stops_at_it() {
    let ground = Walled(FlatGround::new(0.0));
    for speed in [20.0_f32, 50.0, 80.0] {
        let mut body = Ragdoll::new(&lying(Vec3::new(0.0, 0.5, 0.0)), Vec3::new(0.0, 0.0, speed), Vec3::ZERO);
        let mut furthest = f32::MIN;
        for _ in 0..360 {
            body.step(FIXED_STEP, &ground);
            furthest = furthest.max(body.position(0).z + HALF + RADIUS);
        }
        assert!(furthest <= 10.0 + 0.05, "{speed} m/s: its end reached z = {furthest}");
        assert!(body.is_finite() && body.at_rest(), "{speed} m/s: it lies at the wall");
    }
}

#[test]
fn a_body_thrown_along_a_wall_slides_on_without_being_stopped() {
    // The wall is across the road, the body moves across x, parallel to it: nothing is in its way.
    let ground = Walled(FlatGround::new(0.0));
    let mut body = Ragdoll::new(&lying(Vec3::new(0.0, 0.5, 9.0)), Vec3::new(30.0, 0.0, 0.0), Vec3::ZERO);
    for _ in 0..20 {
        body.step(FIXED_STEP, &ground);
    }
    assert!(body.position(0).x > 8.0, "{}", body.position(0));
}

#[test]
fn a_body_that_sank_under_a_rising_road_is_brought_up_onto_it_not_lost() {
    // The road rises 70 cm under a body lying on it: its spheres are under the top of the surface, further than the 40 cm the ray of the
    // road starts above them (what a step of the road or a push of a wall leaves a body in). It comes up without being thrown.
    let road = Lift(std::cell::Cell::new(0.0));
    let mut body = Ragdoll::new(&lying(Vec3::new(0.0, RADIUS, 0.0)), Vec3::ZERO, Vec3::ZERO);
    for _ in 0..30 {
        body.step(FIXED_STEP, &road);
    }
    road.0.set(0.7);
    let mut fastest = 0.0_f32;
    for _ in 0..120 {
        body.step(FIXED_STEP, &road);
        fastest = fastest.max(body.velocity(0).length());
    }
    assert!((body.position(0).y - (0.7 + RADIUS)).abs() < 0.03, "it lies on the new road: {}", body.position(0));
    assert!(fastest < 3.0, "and was not thrown up onto it: {fastest} m/s");
}

#[test]
fn a_surface_far_above_a_body_is_not_taken_for_its_ground() {
    // A deck 2 m over the road: the deep ray would find it from 2.5 m above the sphere, which is more than a step above the road the
    // body touched, so the body under it stays where it is.
    let road = Lift(std::cell::Cell::new(0.0));
    let mut body = Ragdoll::new(&lying(Vec3::new(0.0, RADIUS, 0.0)), Vec3::ZERO, Vec3::ZERO);
    for _ in 0..30 {
        body.step(FIXED_STEP, &road);
    }
    road.0.set(2.0);
    for _ in 0..60 {
        body.step(FIXED_STEP, &road);
    }
    assert!(body.position(0).y < 0.5, "it is not lifted onto the deck: {}", body.position(0));
}

#[test]
fn a_chain_that_is_still_moving_does_not_drag_its_stopped_end_through_a_wall() {
    // Three segments in a line thrown at a wall at 30 m/s: the head has stopped at it while the trunk and the pelvis behind still move and the
    // joints pull it on. A stop of the velocity alone let it through.
    let ground = Walled(FlatGround::new(0.0));
    let segment = |z: f32| Segment {
        mass: 20.0,
        radius: 0.12,
        half_length: 0.2,
        position: Vec3::new(0.0, 0.8, z),
        orientation: Quat::from_rotation_x(FRAC_PI_2),
    };
    let link = |parent, child| Joint {
        parent,
        child,
        anchor_parent: Vec3::Y * 0.32,
        anchor_child: Vec3::NEG_Y * 0.32,
        frame_parent: Quat::IDENTITY,
        frame_child: Quat::IDENTITY,
        limits: JointLimits { swing_z: 3.0, swing_y: 3.0, twist: 3.0 },
    };
    let spec = RagdollSpec {
        segments: vec![segment(7.0), segment(7.64), segment(8.28)],
        joints: vec![link(0, 1), link(1, 2)],
        ..RagdollSpec::default()
    };
    let mut body = Ragdoll::new(&spec, Vec3::new(0.0, 0.0, 30.0), Vec3::ZERO);
    let mut furthest = f32::MIN;
    for _ in 0..240 {
        body.step(FIXED_STEP, &ground);
        for i in 0..body.len() {
            for end in [-0.2_f32, 0.2] {
                let sphere = body.position(i) + body.orientation(i) * Vec3::Y * end;
                furthest = furthest.max(sphere.z + 0.12);
            }
        }
    }
    assert!(furthest <= 10.0 + 0.1, "a sphere reached z = {furthest}");
}
