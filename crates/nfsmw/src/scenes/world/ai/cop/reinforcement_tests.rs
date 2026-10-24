//! Road chase regressions that start from an accepted reinforcement's cursor.

use super::*;
use blackbox_roads::{Road, RoadNetwork, RoadNode, RoadProfile, RoadSegment, SegmentIndex, SplitMix, Zone, zone};
use blackbox_vehicle::performance::TABLE_SIZE;

pub(super) fn road() -> blackbox_roads::RoadNetwork {
    let node = |z, segments| RoadNode { position: Vec3::Z * z, profile: 0, segments };
    let segment = |nodes| RoadSegment {
        nodes,
        length: 600.0,
        road: Some(0),
        flags: 0,
        start_handle: Vec3::ZERO,
        end_handle: Vec3::ZERO,
    };
    RoadNetwork {
        nodes: vec![node(-600.0, vec![0]), node(0.0, vec![0, 1]), node(600.0, vec![1])],
        segments: vec![segment([0, 1]), segment([1, 2])],
        profiles: vec![RoadProfile {
            middle: 1,
            zones: vec![Zone { kind: zone::TRAFFIC, width: 4.0, offset: 2.0 }; 2],
        }],
        roads: vec![Road { scale: 1.0, speech_id: 0 }],
    }
}

pub(super) fn body(z: f32, speed: f32) -> Body {
    Body {
        position: Vec3::new(2.0, 0.0, z),
        velocity: Vec3::Z * speed,
        forward: Vec2::Y,
        half_width: 1.0,
        half_length: 2.5,
    }
}

pub(super) fn cop(speed: f32) -> CopState {
    let performance = Performance { top_speed: 70.0, acceleration: [3.0; TABLE_SIZE], start_grip: 1.0, end_grip: 1.0 };
    CopState::new(CopCar { performance, ai: AiVehicle::default() }, performance, speed)
}

pub(super) fn state(body: Body) -> CarState {
    CarState { forward_speed: body.velocity.length(), body, radius: 3.0, mass: 1500.0, tractor: false, stop: None }
}

#[test]
fn a_distant_ahead_reinforcement_follows_the_predicted_road_instead_of_a_rear_formation_slot() {
    let network = road();
    let index = SegmentIndex::build(&network);
    let mut rng = SplitMix(4);
    let mut ctx =
        Ctx { net: &network, index: &index, rng: &mut rng, bodies: &[], me: 0, player: None, aims: &[], lights: None };
    let me = state(body(150.0, 30.0));
    let target = Target { body: body(-20.0, 40.0), aim: Some(Vec3::new(2.0, 0.0, -40.0)) };
    let mut nav = roads::at_car(&ctx, &me.body, 1).unwrap();
    nav.enable_trail(&network);
    let mut cop = cop(30.0);
    cop.environment(Environment { level: 1, ..Default::default() });
    let collision = CollisionWorld::new(None);
    let think = cop.think(
        &mut ctx,
        &mut nav,
        &me,
        &target,
        [],
        ActionInputs { collision: &collision, grade: 0.0, ram: false, nitrous: RoadNitrous::default() },
    );
    assert!(
        cop.last_find.z > me.body.position.z,
        "road interception must stay ahead despite the rear slot: {:?}",
        cop.last_find
    );
    assert!(think.target.z > me.body.position.z, "same-direction road steering must remain forward: {think:?}");
    assert!(
        think.speed > 0.0,
        "an ahead cop may slow to meet the player, but must not request a reverse stop: {think:?}"
    );
}

#[test]
fn a_road_cursor_at_the_lookahead_radius_behind_the_car_advances_back_in_front() {
    let network = road();
    let index = SegmentIndex::build(&network);
    let mut rng = SplitMix(4);
    let mut ctx =
        Ctx { net: &network, index: &index, rng: &mut rng, bodies: &[], me: 0, player: None, aims: &[], lights: None };
    let me = state(body(0.0, 20.0));
    let target = Target { body: body(120.0, 30.0), aim: None };
    let mut nav = roads::at_car(&ctx, &body(-look_ahead(20.0), 20.0), 1).unwrap();
    nav.enable_trail(&network);
    let mut cop = cop(20.0);
    cop.environment(Environment { level: 1, ..Default::default() });
    let collision = CollisionWorld::new(None);
    let think = cop.think(
        &mut ctx,
        &mut nav,
        &me,
        &target,
        [],
        ActionInputs { collision: &collision, grade: 0.0, ram: false, nitrous: RoadNitrous::default() },
    );
    assert!(nav.position.z > me.body.position.z, "road cursor must advance through the look-ahead circle: {nav:?}");
    assert!(
        think.target.z > me.body.position.z && think.speed > 10.0,
        "a stale cursor must not force a reverse stop: {think:?}"
    );
}
