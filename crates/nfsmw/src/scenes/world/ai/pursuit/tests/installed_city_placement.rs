//! Fixture placement measures physical road travel rather than navigator chord parameters.

use super::*;

pub(super) fn prepare_cursor(world: &TrafficWorld, model: &str, nav: &mut RoadNav) {
    let accepted = (nav.segment, nav.node_ind, nav.lane, nav.t.to_bits());
    let model = world.cop_models.iter().find(|car| car.model.name.eq_ignore_ascii_case(model)).unwrap();
    nav.half_width = model.model.physics.spec.dimension.x;
    // Placement uses the prepared, width-inset trail centre rather than the raw lane centre.
    nav.enable_trail(&world.network);
    assert_eq!((nav.segment, nav.node_ind, nav.lane, nav.t.to_bits()), accepted);
}

pub(super) fn assert_handoff(car: &AiCar, nav: &RoadNav) {
    let distance = car.physics_position().distance(nav.position);
    let facing = car.physics_forward().dot(nav.forward.normalize_or_zero());
    assert!(
        distance < 2.0 && facing > 0.99,
        "trace setup: accepted cursor handoff changed placement; distance={distance} facing_dot={facing} nav=({:?},{:?}) body=({:?},{:?})",
        nav.position,
        nav.forward,
        car.physics_position(),
        car.physics_forward(),
    );
}

pub(super) fn trailing(world: &TrafficWorld, mut nav: RoadNav, target: Body) -> RoadNav {
    nav.reverse(&world.network);
    let mut travelled = 0.0;
    let mut rng = blackbox_roads::SplitMix(53_012);
    for step in 0..2000 {
        let before = nav.position;
        // Navigator metres advance T by chord length; measure the evaluated
        // physical polyline instead of assuming a long curve advances equally.
        nav.advance_with_lookahead(&world.network, 0.25, Vec3::ZERO, 0.0, &mut rng);
        let physical_step = before.distance(nav.position);
        assert!(
            !nav.dead_end && physical_step.is_finite() && physical_step > 1e-5 && physical_step < 5.0,
            "trace setup: backwards road walk stalled, ended or jumped; step={step} moved={physical_step} segment={} t={}",
            nav.segment,
            nav.t,
        );
        travelled += physical_step;
        if travelled < 90.0 {
            continue;
        }
        nav.reverse(&world.network);
        let delta = target.position - nav.position;
        let along = delta.dot(Vec3::new(target.forward.x, 0.0, target.forward.y));
        eprintln!(
            "CITY_TRACE_TRAILING_CURSOR steps={} physical_arc={travelled:.3} radial={:.3} along={along:.3} segment={} lane={} t={:.6} at={:?} facing={:?}",
            step + 1,
            delta.length(),
            nav.segment,
            nav.lane,
            nav.t,
            nav.position,
            nav.forward,
        );
        assert!(
            (60.0..=120.0).contains(&delta.length()) && along > 50.0,
            "trace setup: physical road walk cannot establish the required trailing placement"
        );
        return nav;
    }
    panic!("trace setup: backwards road walk exceeded its bounded work before reaching90 physical metres");
}
