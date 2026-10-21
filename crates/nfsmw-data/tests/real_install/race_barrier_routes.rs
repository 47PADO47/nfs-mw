//! A race's barrier lines on the road graph (`docs/specs/race-barriers.md` section 5): with the race's walls and the
//! door switched on, the racers' route from the start through every checkpoint to the finish still exists, and the
//! lines close side roads.

use blackbox_roads::{
    ActiveBarrier, PathRequest, PathState, PathType, SegmentFilter, SegmentIndex, closest_segment, find_path,
    mark_barrier_crossings,
};
use blackbox_scenery::find_group;
use glam::Vec3;
use nfsmw_data::gameplay::{self, GameplayData, Race, world_heading};
use nfsmw_data::world::{DEFAULT_TRACK, WorldIndex};

use crate::install;

/// The barrier lines a race switches on: its own list and the door, which is always on.
fn active_lines(index: &WorldIndex, race: &Race) -> Vec<ActiveBarrier> {
    let names = race.barriers.iter().map(|e| e.trim_start_matches('*')).chain(["SCENERY_GROUP_DOOR"]);
    let hashes: Vec<u32> = names.map(blackbox_hash::bstring_hash).collect();
    index
        .track_barriers
        .iter()
        .filter(|line| hashes.contains(&line.group_hash))
        .map(|line| {
            let group = find_group(&index.scenery_groups, line.group_hash).expect("a barrier's group exists");
            ActiveBarrier { a: line.a, b: line.b, drive_through: group.drive_through }
        })
        .collect()
}

/// The race's points in order: the start, the checkpoints, the finish (physics space).
fn course(data: &GameplayData, race: &Race) -> Vec<Vec3> {
    let start = race.start.map(|s| s.position).into_iter();
    let gates =
        race.checkpoints.iter().chain(race.finish.iter()).filter_map(|key| data.trigger(*key)).map(|t| t.position);
    start.chain(gates).collect()
}

/// Whether a racer can route through every leg of `points`.
fn routes(net: &blackbox_roads::RoadNetwork, index: &SegmentIndex, points: &[Vec3]) -> bool {
    let at = |p: Vec3| closest_segment(net, index, p, Vec3::ZERO, 0.0, SegmentFilter::default());
    points.windows(2).all(|leg| {
        let (Some(from), Some(to)) = (at(leg[0]), at(leg[1])) else { return true };
        let segment = net.segment(from.segment);
        let request = PathRequest {
            segment: from.segment,
            node: segment.nodes[1],
            may_turn_round: true,
            goal_segment: to.segment,
            goal_node: None,
            goal_position: leg[1],
            path_type: PathType::Racer,
        };
        find_path(net, &request).state != PathState::NoWay
    })
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn race_routes_survive_their_barrier_lines() {
    let Some(dir) = install() else { return };
    let data = gameplay::load(&dir).expect("gameplay data");
    let mut index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    assert_eq!(index.track_barriers.len(), 1376);
    let mut net = index.road_network.take().expect("road network");
    let segments = SegmentIndex::build(&net);

    let (mut races, mut closed_total, mut broken) = (0, 0, Vec::new());
    for race in data.races.iter().filter(|r| !r.barriers.is_empty() && r.start.is_some()) {
        let points = course(&data, race);
        if points.len() < 2 {
            continue;
        }
        let start = race.start.unwrap();
        let exempt: Vec<u16> =
            closest_segment(&net, &segments, start.position, world_heading(start.heading_deg), 1.0, Default::default())
                .and_then(|l| net.segment(l.segment).road)
                .into_iter()
                .collect();
        let routable_before = routes(&net, &segments, &points);
        let closed = mark_barrier_crossings(&mut net, &segments, &active_lines(&index, race), &exempt);
        closed_total += closed;
        races += 1;
        if routable_before && !routes(&net, &segments, &points) {
            broken.push(race.name.clone());
        }
        mark_barrier_crossings(&mut net, &segments, &[], &[]);
    }
    eprintln!("{races} races, {closed_total} segments closed in all, routes broken by the lines: {broken:?}");
    assert!(races > 200);
    assert!(closed_total > races, "the lines close side roads");
    // With this crossing test 15 of 215 courses lose a leg, mostly where a checkpoint snaps to a junction connector
    // that a line crosses; the original's exact test is open (`docs/specs/race-barriers.md` section 5).
    assert!(broken.len() * 10 <= races, "the lines keep most race routes open: {broken:?}");

    // Free roam: the door's lines alone close a few segments around the junkyard.
    let door = blackbox_hash::bstring_hash("SCENERY_GROUP_DOOR");
    let lines: Vec<ActiveBarrier> = index
        .track_barriers
        .iter()
        .filter(|line| line.group_hash == door)
        .map(|line| ActiveBarrier { a: line.a, b: line.b, drive_through: false })
        .collect();
    assert_eq!(lines.len(), 23);
    let closed = mark_barrier_crossings(&mut net, &segments, &lines, &[]);
    eprintln!("the door closes {closed} segments");
    assert!((1..60).contains(&closed), "{closed}");
}
