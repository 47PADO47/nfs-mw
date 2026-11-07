//! Installed route-wall assertions.
use super::WorldScene;

pub(super) fn assert_shortcuts_respect_walls(scene: &WorldScene) {
    use crate::scenes::world::ai::car::Role;
    use blackbox_roads::{SegmentFilter, closest_segment, flags, mark_barrier_crossings};
    let traffic = scene.traffic.as_ref().unwrap();
    let base = scene.race.as_ref().unwrap().route.as_ref().unwrap();
    let start = base.legs[0].from;
    let mut net = traffic.network.clone();
    let exempt: Vec<_> =
        closest_segment(&net, &traffic.index, start.position, start.forward, 1.0, SegmentFilter::default())
            .and_then(|point| net.segment(point.segment).road)
            .into_iter()
            .collect();
    let lines = scene.scenery_groups.active_barriers(traffic.track_barriers());
    assert!(!lines.is_empty(), "fixture must have active race walls");
    mark_barrier_crossings(&mut net, &traffic.index, &lines, &exempt);
    let both = flags::CROSSES_BARRIER | flags::CROSSES_DRIVE_THROUGH_BARRIER;
    for car in &traffic.cars {
        let Role::Racer(state) = &car.role else { continue };
        let route = state.route.as_ref().unwrap();
        for (index, leg) in route.legs.iter().enumerate() {
            for step in &route.steps[leg.steps.clone()] {
                if net.segment(step.segment).flags & both == 0 {
                    continue;
                }
                let accepted = &base.steps[base.legs[index].steps.clone()];
                assert!(
                    accepted.iter().any(|base| base.segment == step.segment),
                    "racer {} shortcut introduces a new active-wall crossing on segment {}",
                    state.id,
                    step.segment
                );
            }
        }
    }
}
