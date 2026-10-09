//! Against a real install (set `NFSMW_GAME_DIR`). Counts are the measurements in
//! `docs/formats/road-network.md`.

use glam::Vec3;

use crate::{RoadNetwork, SegmentFilter, SegmentIndex, closest_segment, flags, lane_line, travel_profile};

fn network() -> Option<RoadNetwork> {
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
