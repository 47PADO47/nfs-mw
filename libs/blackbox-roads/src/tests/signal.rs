//! The junction signals: groups, approaches, phases and the controller's clock.

use glam::Vec3;

use super::install::network;
use super::synth::{chain_road, segment, two_way_profile};
use crate::{
    ALL_RED_TIME, AMBER_TIME, GREEN_TIME, Light, MIN_SIGNAL_APPROACHES, Road, RoadNetwork, RoadNode, RoadSegment,
    STOP_LINE_DISTANCE, SignalController, Timing, flags,
};

/// Metres from the centre of the synthetic junction to its junction nodes and from those to the arm ends.
const NODE_RADIUS: f32 = 10.0;
const ARM_LENGTH: f32 = 100.0;
/// The directions of the arms of a four-way junction: west, east, south, north.
const DIRECTIONS: [Vec3; 4] = [Vec3::NEG_X, Vec3::X, Vec3::NEG_Z, Vec3::Z];
/// A step of the clock in the sweeps, seconds.
const SWEEP_STEP: f32 = 0.1;

/// A junction with `arms` roads (1 to 4) along the axes. Nodes `0..arms` are the junction nodes, `arms..2 * arms`
/// the far ends of the arms; segment `i` is the arm of node `i`, driven towards the junction, and the decision
/// segments join every pair of junction nodes.
fn junction(arms: usize) -> RoadNetwork {
    let at = |i: usize, radius: f32| DIRECTIONS[i] * radius;
    let mut nodes: Vec<RoadNode> =
        (0..arms).map(|i| RoadNode { position: at(i, NODE_RADIUS), profile: 0, segments: vec![i as u16] }).collect();
    nodes.extend((0..arms).map(|i| RoadNode {
        position: at(i, NODE_RADIUS + ARM_LENGTH),
        profile: 0,
        segments: vec![i as u16],
    }));
    let mut segments: Vec<RoadSegment> =
        (0..arms).map(|i| segment([(arms + i) as u16, i as u16], ARM_LENGTH, 0)).collect();
    for a in 0..arms {
        for b in a + 1..arms {
            let index = segments.len() as u16;
            segments.push(segment([a as u16, b as u16], 20.0, flags::DECISION | flags::INTERSECTION));
            nodes[a].segments.push(index);
            nodes[b].segments.push(index);
        }
    }
    RoadNetwork { nodes, segments, profiles: vec![two_way_profile()], roads: vec![Road { scale: 1.0, speech_id: 0 }] }
}

#[test]
fn a_four_way_junction_has_four_approaches_in_two_phases() {
    let controller = SignalController::new(&junction(4));
    assert_eq!(controller.junctions().len(), 1);
    assert_eq!(controller.approaches().len(), 4);
    let phase = |node: u16| controller.approaches()[controller.approach_at(node).expect("approach")].phase;
    // West and east face each other, as do south and north.
    assert_eq!(phase(0), phase(1));
    assert_eq!(phase(2), phase(3));
    assert_ne!(phase(0), phase(2));
}

#[test]
fn the_stop_line_is_before_the_node_along_the_approach() {
    let controller = SignalController::new(&junction(4));
    let west = &controller.approaches()[controller.approach_at(0).expect("west")];
    assert_eq!((west.segment, west.node, west.node_ind), (0, 0, 1));
    // Driving east towards the node at x = -10.
    assert!(west.heading.distance(Vec3::X) < 1e-3, "{:?}", west.heading);
    let expected = Vec3::new(-NODE_RADIUS - STOP_LINE_DISTANCE, 0.0, 0.0);
    assert!(west.stop_position.distance(expected) < 0.5, "{:?}", west.stop_position);
    let north = &controller.approaches()[controller.approach_at(3).expect("north")];
    assert!(north.heading.distance(Vec3::NEG_Z) < 1e-3);
}

#[test]
fn a_t_junction_has_signals_and_two_phases() {
    let controller = SignalController::new(&junction(3));
    assert_eq!(controller.approaches().len(), 3);
    let phases: Vec<usize> = controller.approaches().iter().map(|a| a.phase).collect();
    assert!(phases.contains(&0) && phases.contains(&1), "{phases:?}");
}

#[test]
fn a_junction_with_fewer_than_three_roads_has_no_signals() {
    const { assert!(MIN_SIGNAL_APPROACHES > 2) };
    let controller = SignalController::new(&junction(2));
    assert_eq!(controller.junctions().len(), 1);
    assert!(!controller.junctions()[0].has_signals());
    assert!(controller.approaches().is_empty());
    assert_eq!(controller.approach_at(0), None);
}

#[test]
fn a_road_without_junctions_has_no_groups() {
    let controller = SignalController::new(&chain_road());
    assert!(controller.junctions().is_empty());
}

#[test]
fn a_one_way_road_leaving_the_junction_is_not_an_approach() {
    let mut net = junction(4);
    // Arm 0 runs from the junction node outwards: its stored direction leaves the node.
    net.segments[0] = segment([0, 4], ARM_LENGTH, flags::ONE_WAY);
    let controller = SignalController::new(&net);
    assert_eq!(controller.approaches().len(), 3);
    assert_eq!(controller.approach_at(0), None);
}

#[test]
fn the_lights_cycle_green_amber_red_in_turn() {
    let controller = SignalController::new(&junction(4));
    let first = controller.approach_at(0).expect("west");
    let across = controller.approach_at(2).expect("south");
    assert_eq!(controller.state(first, 0.0), Light::Green);
    assert_eq!(controller.state(first, GREEN_TIME + 0.1), Light::Amber);
    assert_eq!(controller.state(first, GREEN_TIME + AMBER_TIME + 0.1), Light::Red);
    // The other phase starts after the gap and the cycle repeats.
    assert_eq!(controller.state(across, 0.0), Light::Red);
    assert_eq!(controller.state(across, GREEN_TIME + AMBER_TIME + ALL_RED_TIME + 0.1), Light::Green);
    assert_eq!(controller.state(first, controller.timing().cycle() + 0.1), Light::Green);
}

#[test]
fn two_crossing_roads_are_never_open_at_once() {
    let controller = SignalController::new(&junction(4));
    let approaches = controller.approaches();
    let open = |a: usize, time: f32| controller.state(a, time) != Light::Red;
    let mut time = 0.0;
    while time < 3.0 * controller.timing().cycle() {
        for a in 0..approaches.len() {
            for b in 0..approaches.len() {
                let crossing = approaches[a].phase != approaches[b].phase;
                assert!(!(crossing && open(a, time) && open(b, time)), "t {time}: approaches {a} and {b}");
            }
        }
        time += SWEEP_STEP;
    }
}

#[test]
fn every_approach_gets_its_green() {
    let controller = SignalController::new(&junction(4));
    let steps = (controller.timing().cycle() / SWEEP_STEP) as usize;
    for approach in 0..4 {
        let greens = (0..steps).filter(|&i| controller.state(approach, i as f32 * SWEEP_STEP) == Light::Green).count();
        let expected = (GREEN_TIME / SWEEP_STEP) as usize;
        assert!(greens.abs_diff(expected) <= 1, "approach {approach}: {greens} of {steps} steps");
    }
}

#[test]
fn junctions_are_shifted_against_each_other() {
    let mut net = junction(4);
    // A second, identical junction far away, with its own node and segment indices.
    let (n, s) = (net.nodes.len() as u16, net.segments.len() as u16);
    for node in net.nodes.clone() {
        net.nodes.push(RoadNode {
            position: node.position + Vec3::new(5000.0, 0.0, 0.0),
            profile: 0,
            segments: node.segments.iter().map(|&x| x + s).collect(),
        });
    }
    for seg in net.segments.clone() {
        net.segments.push(RoadSegment { nodes: seg.nodes.map(|x| x + n), ..seg });
    }
    let controller = SignalController::new(&net);
    assert_eq!(controller.junctions().len(), 2);
    assert_ne!(controller.junctions()[0].offset, controller.junctions()[1].offset);
    let first = controller.approach_at(0).expect("first junction");
    let second = controller.approach_at(n).expect("second junction");
    // Ten seconds in, one is still green and the shifted one is already amber.
    assert_ne!(controller.state(first, 10.0), controller.state(second, 10.0));
}

#[test]
fn custom_timing_changes_the_cycle() {
    let timing = Timing { green: 5.0, amber: 1.0, all_red: 1.0 };
    let controller = SignalController::with_timing(&junction(4), timing);
    let first = controller.approach_at(0).expect("west");
    assert_eq!(controller.timing().cycle(), 14.0);
    assert_eq!(controller.state(first, 5.5), Light::Amber);
    assert_eq!(controller.state(first, 6.5), Light::Red);
    assert_eq!(controller.state(first, 14.5), Light::Green);
}

#[test]
#[ignore = "needs a game install; set NFSMW_GAME_DIR"]
fn real_install_junctions_have_two_phases_each() {
    let Some(net) = network() else { return };
    let controller = SignalController::new(&net);
    // The decision segments form 731 connected groups (docs/formats/road-network.md).
    assert_eq!(controller.junctions().len(), 731);
    let signalled: Vec<_> = controller.junctions().iter().filter(|j| j.has_signals()).collect();
    eprintln!(
        "{} of {} junctions signalled, {} approaches",
        signalled.len(),
        controller.junctions().len(),
        controller.approaches().len()
    );
    assert!(signalled.len() > 700, "{} signalled junctions", signalled.len());
    for junction in signalled {
        for phase in 0..2 {
            let count = junction.approaches.iter().filter(|&&a| controller.approaches()[a].phase == phase).count();
            assert!(count > 0, "junction at nodes {:?}: phase {phase} is empty", junction.nodes);
        }
    }
    for approach in controller.approaches() {
        let distance = approach.stop_position.distance(net.node(approach.node).position);
        assert!(distance <= STOP_LINE_DISTANCE + 1.0, "stop line {distance} m from its node");
    }
}
