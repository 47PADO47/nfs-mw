//! Against a real install (set `NFSMW_GAME_DIR`). Counts are the measurements in
//! `docs/formats/road-network.md`.

use glam::Vec3;

use crate::{RoadNetwork, SegmentFilter, SegmentIndex, closest_segment, flags, lane_line, travel_profile};

pub(super) fn network() -> Option<RoadNetwork> {
    let dir = std::env::var_os("NFSMW_GAME_DIR")?;
    let path = std::path::Path::new(&dir).join("TRACKS/L2RA.BUN");
    let raw = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let data = ea_compress::unwrap(&raw).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    Some(RoadNetwork::read(&data).expect("road network").expect("L2RA.BUN has a road network"))
}

#[test]
#[ignore = "needs a game install; set NFSMW_GAME_DIR"]
fn real_install_road_network_counts() {
    let Some(net) = network() else { return };
    assert_eq!(net.nodes.len(), 4385);
    assert_eq!(net.segments.len(), 6538);
    assert_eq!(net.profiles.len(), 710);
    assert_eq!(net.roads.len(), 1308);
    let count = |flag| net.segments.iter().filter(|s| s.has(flag)).count();
    assert_eq!(count(flags::DECISION), 3463);
    assert_eq!(count(flags::CURVED), 5892);
    assert_eq!(count(flags::ONE_WAY), 104);
    assert_eq!(net.nodes.iter().filter(|n| n.segments.len() == 1).count(), 5);
}

#[test]
#[ignore = "needs a game install; set NFSMW_GAME_DIR"]
fn real_install_curve_length_matches_stored_length() {
    let Some(net) = network() else { return };
    let mut worst: f32 = 1.0;
    for s in 0..net.segments.len() as u16 {
        let seg = net.segment(s);
        let ratio = crate::centre_line(&net, s, 1).length(32) / seg.length.max(0.1);
        assert!((0.8..1.3).contains(&ratio), "segment {s}: curve/stored length {ratio}");
        worst = worst.max(ratio);
    }
    assert!(worst > 1.0);
}

/// A lane stays on one side of the centre line along a whole segment: the offsets of the same lane at
/// both ends have the same sign. This checks the profile inversion flags and the travel-frame reading.
#[test]
#[ignore = "needs a game install; set NFSMW_GAME_DIR"]
fn real_install_lanes_keep_their_side() {
    let Some(net) = network() else { return };
    let (mut checked, mut flipped) = (0, 0);
    for s in 0..net.segments.len() as u16 {
        for node_ind in 0..2 {
            let (from, to) = (travel_profile(&net, s, node_ind, false), travel_profile(&net, s, node_ind, true));
            // Lanes only correspond one to one where both ends have the same layout.
            if (from.zones.len(), from.middle) != (to.zones.len(), to.middle) {
                continue;
            }
            for lane in 0..from.zones.len() {
                let (a, b) = (from.signed_offset(lane), to.signed_offset(lane));
                checked += 1;
                if a * b < -0.5 {
                    flipped += 1;
                }
            }
        }
    }
    assert!(checked > 10_000);
    assert!(flipped * 200 < checked, "{flipped} of {checked} lanes change side");
}

#[test]
#[ignore = "needs a game install; set NFSMW_GAME_DIR"]
fn real_install_finds_the_segment_under_a_node() {
    let Some(net) = network() else { return };
    let index = SegmentIndex::build(&net);
    for node in net.nodes.iter().step_by(37) {
        let hit = closest_segment(&net, &index, node.position, Vec3::Z, 0.0, SegmentFilter::default()).unwrap();
        assert!(hit.point.distance(node.position) < 0.5, "{:?}", hit);
    }
    let seg = net.segment(100);
    let line = lane_line(&net, 100, 1, seg_lane(&net));
    assert!(line.start().distance(net.node(seg.nodes[0]).position) < 20.0);
}

fn seg_lane(net: &RoadNetwork) -> usize {
    usize::from(net.profiles[usize::from(net.node(net.segment(100).nodes[0]).profile)].middle)
}

/// Spawn a traffic cursor in every traffic lane of every plain segment where traffic is allowed and
/// advance it 300 m: it only dead-ends at the dead-end nodes (spec §8).
#[test]
#[ignore = "needs a game install; set NFSMW_GAME_DIR"]
fn real_install_traffic_cursors_never_dead_end_early() {
    use crate::{RoadNav, SplitMix};
    let Some(net) = network() else { return };
    let mut rng = SplitMix(7);
    let (mut runs, mut dead) = (0, 0);
    let mut stuck_at = Vec::new();
    for s in 0..net.segments.len() as u16 {
        let seg = net.segment(s);
        if seg.is_decision() || seg.has(flags::NO_TRAFFIC) {
            continue;
        }
        let profile = net.profile_at(s, seg.nodes[0]);
        for lane in profile
            .lanes_of(crate::zone::TRAFFIC, true)
            .into_iter()
            .chain(profile.lanes_of(crate::zone::TRAFFIC, false))
        {
            let mut nav = RoadNav::traffic();
            nav.init_in_lane(&net, s, lane, 0.5);
            for _ in 0..30 {
                nav.advance(&net, 10.0, Vec3::ZERO, &mut rng);
            }
            runs += 1;
            if nav.dead_end {
                dead += 1;
                stuck_at.push((s, nav.segment));
            }
        }
    }
    eprintln!("traffic runs {runs}, dead ends {dead}");
    assert!(runs > 5000, "{runs} runs");
    // Dead ends are the 5 dead-end nodes plus one-way and no-traffic exits: a small share of runs.
    assert!(dead * 20 < runs, "{dead} of {runs} cursors dead-ended: {:?}", &stuck_at[..stuck_at.len().min(10)]);
}

/// Routes between far apart nodes are connected walks; ordinary routes never chain two junction
/// connectors, and are not much longer than the straight line (spec §9).
#[test]
#[ignore = "needs a game install; set NFSMW_GAME_DIR"]
fn real_install_paths_are_connected_walks() {
    use crate::{PathRequest, PathState, PathType, RandomSource, find_path};
    let Some(net) = network() else { return };
    let mut rng = crate::SplitMix(3);
    let mut ratios = Vec::new();
    let (mut full, mut other) = (0, 0);
    for _ in 0..400 {
        let (a, b) = (rng.index(net.segments.len()) as u16, rng.index(net.segments.len()) as u16);
        let (sa, sb) = (net.segment(a), net.segment(b));
        if sa.is_decision() || sb.is_decision() || a == b {
            continue;
        }
        let goal = net.node(sb.nodes[0]).position;
        for path_type in [PathType::Racer, PathType::Cop] {
            let request = PathRequest {
                segment: a,
                node: sa.nodes[1],
                may_turn_round: true,
                goal_segment: b,
                goal_node: None,
                goal_position: goal,
                path_type,
            };
            let result = find_path(&net, &request);
            if result.state != PathState::Full {
                other += 1;
                continue;
            }
            full += 1;
            for pair in result.segments.windows(2) {
                let (x, y) = (net.segment(pair[0]), net.segment(pair[1]));
                assert!(x.nodes.iter().any(|n| y.nodes.contains(n)), "{pair:?} are not connected");
                if path_type != PathType::Cop {
                    assert!(!(x.is_decision() && y.is_decision()), "two connectors in a row: {pair:?}");
                }
            }
            let length: f32 = result.segments.iter().map(|&s| net.segment(s).length).sum();
            let straight = net.node(sa.nodes[1]).position.distance(goal);
            if path_type == PathType::Racer && straight > 300.0 {
                ratios.push(length / straight);
            }
        }
    }
    eprintln!("{full} routes found, {other} not");
    assert!(ratios.len() > 10, "{} long routes", ratios.len());
    let mean = ratios.iter().sum::<f32>() / ratios.len() as f32;
    eprintln!("mean route length over straight distance: {mean:.2} over {} routes", ratios.len());
    assert!((1.0..2.0).contains(&mean), "{mean}");
}

#[test]
#[ignore = "needs a game install; set NFSMW_GAME_DIR"]
fn real_install_track_zones_match_the_documented_counts() {
    let Some(dir) = std::env::var_os("NFSMW_GAME_DIR") else { return };
    let path = std::path::Path::new(&dir).join("TRACKS/L2RA.BUN");
    let raw = std::fs::read(&path).unwrap();
    let data = ea_compress::unwrap(&raw).unwrap();
    let zones = crate::TrackZones::read(&data).expect("zones").expect("L2RA.BUN has track path zones");
    assert_eq!(zones.zones.len(), 705);
    let of_kind = |kind| zones.zones.iter().filter(|z| z.kind == kind).count();
    assert_eq!((of_kind(3), of_kind(4), of_kind(5), of_kind(6)), (70, 143, 178, 211));
    assert_eq!((of_kind(9), of_kind(13), of_kind(14)), (11, 3, 3));
    // Every polygon lies inside its own bounding box.
    for z in &zones.zones {
        assert!(z.polygon.iter().all(|p| p.cmpge(z.bbox_min - 1e-3).all() && p.cmple(z.bbox_max + 1e-3).all()));
    }
}
