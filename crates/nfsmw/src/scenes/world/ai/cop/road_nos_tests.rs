//! Actual road-action regressions; close pursuit remains a separate action.

use super::reinforcement_tests::{body, cop, road, state};
use super::*;
use blackbox_roads::{SegmentIndex, SplitMix};

#[test]
fn a_distant_road_pursuit_uses_its_bottle_after_the_native_rest_interval() {
    let network = road();
    let index = SegmentIndex::build(&network);
    let mut rng = SplitMix(4);
    let mut ctx =
        Ctx { net: &network, index: &index, rng: &mut rng, bodies: &[], me: 0, player: None, aims: &[], lights: None };
    let mut cop = cop(20.0);
    cop.environment(Environment { level: 5, nitrous_boost: 1.0, ..Default::default() });
    let me = state(body(0.0, 20.0));
    let target = Target { body: body(180.0, 60.0), aim: None };
    let mut nav = roads::at_car(&ctx, &me.body, 5).unwrap();
    nav.enable_trail(&network);
    let collision = CollisionWorld::new(None);
    let mut first_on = None;
    for tick in 0..60 {
        let was_on = cop.nitrous();
        cop.think(
            &mut ctx,
            &mut nav,
            &me,
            &target,
            [],
            ActionInputs {
                collision: &collision,
                grade: 0.0,
                ram: false,
                nitrous: RoadNitrous { boost: 1.0, bottle_seconds: 10.0, capacity: 1.0, was_on },
            },
        );
        assert!(!cop.close_active(), "this regression must exercise the road action");
        if cop.nitrous() && first_on.is_none() {
            first_on = Some(tick as f32 * THINK_PERIOD);
        }
    }
    let first_on = first_on.expect("a distant cop with a usable bottle must eventually request road-action nitrous");
    assert!((4.0..7.0).contains(&first_on), "the road action starts with a real rest interval: {first_on}");
}

#[test]
fn returning_from_close_pursuit_resets_the_road_governor_at_the_current_speed() {
    let network = road();
    let index = SegmentIndex::build(&network);
    let mut rng = SplitMix(4);
    let mut ctx =
        Ctx { net: &network, index: &index, rng: &mut rng, bodies: &[], me: 0, player: None, aims: &[], lights: None };
    let mut cop = cop(20.0);
    cop.environment(Environment { level: 5, ..Default::default() });
    let near = state(body(0.0, 20.0));
    let mut nav = roads::at_car(&ctx, &near.body, 5).unwrap();
    nav.enable_trail(&network);
    let collision = CollisionWorld::new(None);
    let inputs = || ActionInputs { collision: &collision, grade: 0.0, ram: false, nitrous: RoadNitrous::default() };
    cop.think(&mut ctx, &mut nav, &near, &Target { body: body(30.0, 20.0), aim: None }, [], inputs());
    assert!(cop.close_active(), "the first think must enter close pursuit");
    let now = state(body(0.0, 40.0));
    let think = cop.think(&mut ctx, &mut nav, &now, &Target { body: body(180.0, 60.0), aim: None }, [], inputs());
    assert!(!cop.close_active(), "the distant target must restore the road action");
    assert!(
        think.speed >= now.forward_speed,
        "a fresh road governor must not brake for stale pre-close speed history: {think:?}"
    );
}
