use super::reinforcement_tests::{body, cop, road, state};
use super::*;
use crate::scenes::world::StateHasher;
use blackbox_roads::{SegmentIndex, SplitMix};

#[test]
fn primary_uses_its_shared_seek_and_failed_tracking_uses_the_published_body() {
    let network = road();
    let index = SegmentIndex::build(&network);
    for observation in [
        Some(TargetRoad {
            gap: 0.0,
            direction: Vec2::Y,
            centre: body(120.0, 30.0).position,
            seek: body(450.0, 0.0).position,
        }),
        None,
    ] {
        let mut rng = SplitMix(4);
        let mut ctx = Ctx {
            net: &network,
            index: &index,
            rng: &mut rng,
            bodies: &[],
            me: 0,
            player: None,
            aims: &[],
            lights: None,
        };
        let me = state(body(0.0, 20.0));
        let target = Target { body: body(120.0, 30.0), aim: None };
        let mut nav = roads::at_car(&ctx, &me.body, 1).unwrap();
        nav.enable_trail(&network);
        let mut cop = cop(20.0);
        cop.environment(Environment {
            level: 1,
            shared_target_road: true,
            target_road: observation,
            ..Default::default()
        });
        cop.think(
            &mut ctx,
            &mut nav,
            &me,
            &target,
            [],
            ActionInputs {
                collision: &CollisionWorld::new(None),
                grade: 0.0,
                ram: false,
                nitrous: RoadNitrous::default(),
            },
        );
        assert_eq!(cop.last_find, observation.map_or(target.body.position, |road| road.seek));
    }
}

#[test]
fn shared_road_snapshot_and_its_admission_flag_affect_driver_hash() {
    let hash = |cop: &CopState| {
        let mut hash = StateHasher::default();
        cop.hash_into(&mut hash);
        hash.finish()
    };
    let mut cop = cop(20.0);
    let old = hash(&cop);
    let mut environment = Environment { shared_target_road: true, ..Default::default() };
    cop.environment(environment);
    assert_ne!(hash(&cop), old);
    let missing = hash(&cop);
    environment.target_road = Some(TargetRoad { gap: 1.0, direction: Vec2::Y, centre: Vec3::Z, seek: Vec3::Z * 100.0 });
    cop.environment(environment);
    assert_ne!(hash(&cop), missing);
    let original = hash(&cop);
    environment.target_road.as_mut().unwrap().seek.x = 2.0;
    cop.environment(environment);
    assert_ne!(hash(&cop), original);
}
