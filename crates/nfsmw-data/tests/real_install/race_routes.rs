//! The opponents' route of every race (`docs/specs/ai-racers-route.md` section 2): the road from the start through
//! every checkpoint to the finish, with the race's barrier lines and the door closing the roads they cross. Reports how
//! many legs each pass of the fallback found: a route search with the lines, the same with the lines lifted near the
//! leg's ends, a plain racer search, or none. A leg counts only when its centre line passes through its gate.

use blackbox_race::Gate;
use blackbox_roads::{
    ActiveBarrier, Goal, LegSource, PathType, Route, RouteBuilder, SegmentIndex, build_leg, build_route,
    closest_segment,
};
use blackbox_scenery::find_group;
use glam::Vec3;
use nfsmw_data::gameplay::{self, BoxTrigger, GameplayData, Race, world_heading};
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
        .filter_map(|line| {
            let group = find_group(&index.scenery_groups, line.group_hash)?;
            Some(ActiveBarrier { a: line.a, b: line.b, drive_through: group.drive_through })
        })
        .collect()
}

/// A trigger's gate, as the race builds it (`race/setup.rs`, `gate_of`).
fn gate_of(trigger: &BoxTrigger) -> Gate {
    let [along, across, half_height] = trigger.dimensions;
    Gate::new(trigger.position, world_heading(trigger.heading_deg), Vec3::Y, [along, across, half_height * 2.0])
}

/// The gates of one lap in order: the checkpoints, then the finish.
fn gates(data: &GameplayData, race: &Race) -> Vec<Gate> {
    race.checkpoints.iter().chain(race.finish.iter()).filter_map(|key| data.trigger(*key)).map(gate_of).collect()
}

/// Whether a racer could cut through `via` on `route`: a road from the nearest leg's start through `via` to its end,
/// at most 1.25 times the leg (`ai/race_route.rs`, `route_through`).
fn has_shortcut(net: &blackbox_roads::RoadNetwork, segments: &SegmentIndex, route: &Route, via: Goal) -> bool {
    let detour = |leg: &blackbox_roads::Leg| {
        leg.from.position.distance(via.position) + via.position.distance(leg.to.position) - leg.length
    };
    let Some(leg) = route.legs.iter().filter(|l| !l.missing()).min_by(|a, b| detour(a).total_cmp(&detour(b))) else {
        return false;
    };
    let first = build_leg(net, segments, leg.from, via, PathType::RaceRoute);
    let second = build_leg(net, segments, via, leg.to, PathType::RaceRoute);
    let (Some(first), Some(second)) = (first, second) else { return false };
    let mut builder = RouteBuilder::new(net);
    builder.push(leg.from, leg.to, Some(&first.join(&second)));
    builder.finish().length <= leg.length * 1.25
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn race_routes() {
    let Some(dir) = install() else { return };
    let data = gameplay::load(&dir).expect("gameplay data");
    let mut index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    let mut net = index.road_network.take().expect("road network");
    let segments = SegmentIndex::build(&net);

    assert!(data.shortcuts.len() > 1000, "{} shortcut markers", data.shortcuts.len());
    let (mut markers, mut cuts) = (0, 0);
    let mut totals = [0usize; 4];
    let (mut courses, mut complete, mut gaps) = (0, 0, Vec::new());
    for race in data.races.iter().filter(|r| r.start.is_some()) {
        let gates = gates(&data, race);
        if gates.is_empty() {
            continue;
        }
        let start = race.start.unwrap();
        let start = Goal { position: start.position, forward: world_heading(start.heading_deg) };
        let exempt: Vec<u16> = closest_segment(&net, &segments, start.position, start.forward, 1.0, Default::default())
            .and_then(|l| net.segment(l.segment).road)
            .into_iter()
            .collect();
        let goals: Vec<Goal> = gates.iter().map(|g| Goal { position: g.centre, forward: g.forward }).collect();
        let lines = active_lines(&index, race);
        let through = |leg: usize, points: &[Vec3]| points.windows(2).any(|pair| gates[leg].touches(pair[0], pair[1]));
        let (route, sources) = build_route(&mut net, &segments, start, &goals, (&lines, &exempt), through);
        assert!(net.segments.iter().all(|s| !s.has(blackbox_roads::flags::CROSSES_BARRIER)), "flags left set");
        courses += 1;
        let missing = sources.iter().filter(|&&s| s == LegSource::Missing).count();
        for source in &sources {
            totals[*source as usize] += 1;
        }
        match missing {
            0 => complete += 1,
            n => gaps.push(format!("{} ({n}/{} missing)", race.name, sources.len())),
        }
        assert_eq!(route.legs.len(), gates.len());
        for marker in race.params.shortcuts.iter().filter_map(|k| data.shortcut(*k)) {
            markers += 1;
            let via = Goal { position: marker.position, forward: world_heading(marker.heading_deg) };
            cuts += usize::from(has_shortcut(&net, &segments, &route, via));
        }
    }
    let legs: usize = totals.iter().sum();
    eprintln!(
        "{courses} courses, {legs} legs: {} with the lines, {} with the ends opened, {} open, {} missing; \
         {complete} courses complete",
        totals[0], totals[1], totals[2], totals[3]
    );
    eprintln!("courses with a missing leg: {gaps:#?}");
    eprintln!("{cuts} of {markers} shortcut markers have a road at most 1.25 times their leg");
    assert!(courses > 200);
    assert!(totals[3] * 10 <= legs, "most legs have a road");
}
