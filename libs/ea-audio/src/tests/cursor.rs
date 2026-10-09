use super::graph_build::*;
use crate::error::Error;
use crate::mus::graph::{Advance, Cursor, Graph, WalkEnd};

fn parse(map: &Map) -> Graph {
    Graph::parse(&map.build()).unwrap()
}

fn audio_of(step: Advance) -> (usize, u32, Vec<u32>) {
    match step {
        Advance::Audio { node, stream, fired } => (node, stream, fired),
        other => panic!("expected an audio node, got {other:?}"),
    }
}

/// Node 0 is a head; 1 and 4 are audio nodes that play stream 0 and 3; 2 fires an event; 3 is an audio node that
/// branches on the control value (calm below 64 back to 1, intense from 64 on to 4); 5 is the end.
fn branching() -> Map {
    Map {
        nodes: vec![
            control(0, vec![(0, 127, 1)], 0),
            audio(0, 2),
            control(0xFFFD, vec![(0, 127, 3)], 0x00AB_CDEF),
            N { id: 3, section: 1, router: 0, trans: vec![(0, 63, 1), (64, 127, 4)], d3: 0 },
            audio(3, 5),
            control(0xFFFF, vec![], 0),
        ],
        events: vec![],
        routers: vec![],
        variables: vec![],
        streams: vec![(2, 1000), (4, 1000), (6, 1000), (8, 1000)],
    }
}

#[test]
fn the_first_step_enters_the_graph_through_control_nodes() {
    let graph = parse(&branching());
    let mut cursor = Cursor::new(&graph, 0).unwrap();
    assert_eq!(cursor.node(), None);
    assert_eq!(audio_of(cursor.advance(&graph, 0)), (1, 0, vec![]));
    assert_eq!(cursor.node(), Some(1));
}

#[test]
fn fire_events_on_the_way_are_reported_and_not_run() {
    let graph = parse(&branching());
    let mut cursor = Cursor::new(&graph, 0).unwrap();
    cursor.advance(&graph, 0);
    // Node 1 leads to the fire-event node 2 and on to node 3, which plays stream 2.
    assert_eq!(audio_of(cursor.advance(&graph, 0)), (3, 2, vec![0x00AB_CDEF]));
}

#[test]
fn the_control_value_at_each_step_picks_the_branch() {
    let graph = parse(&branching());
    let mut calm = Cursor::new(&graph, 3).unwrap();
    assert_eq!(audio_of(calm.advance(&graph, 0)).0, 3);
    // Calm at node 3 goes back to node 1, which leads on to 3 again: the track loops for ever.
    for _ in 0..3 {
        assert_eq!(audio_of(calm.advance(&graph, 10)).0, 1);
        assert_eq!(audio_of(calm.advance(&graph, 10)).0, 3);
    }
    // The same node with a high value leaves the loop; the value can change between any two steps.
    let mut live = Cursor::new(&graph, 3).unwrap();
    live.advance(&graph, 0);
    assert_eq!(audio_of(live.advance(&graph, 100)).0, 4);
    assert_eq!(live.advance(&graph, 100), Advance::End(WalkEnd::EndNode));
}

#[test]
fn a_cursor_that_ended_stays_ended() {
    let graph = parse(&branching());
    let mut cursor = Cursor::new(&graph, 4).unwrap();
    cursor.advance(&graph, 0);
    assert!(!cursor.ended());
    assert_eq!(cursor.advance(&graph, 0), Advance::End(WalkEnd::EndNode));
    assert!(cursor.ended());
    assert_eq!(cursor.advance(&graph, 0), Advance::End(WalkEnd::EndNode));
    assert_eq!(cursor.node(), None);
}

#[test]
fn dead_ends_random_nodes_and_cycles_of_silent_nodes_end_the_track() {
    let mut map = branching();
    map.nodes[4].trans.clear();
    let graph = parse(&map);
    let mut cursor = Cursor::new(&graph, 4).unwrap();
    cursor.advance(&graph, 0);
    assert_eq!(cursor.advance(&graph, 0), Advance::End(WalkEnd::DeadEnd));

    let mut map = branching();
    map.nodes[2] = control(0xFFFE, vec![(0, 127, 3)], 0);
    let graph = parse(&map);
    let mut cursor = Cursor::new(&graph, 2).unwrap();
    assert_eq!(cursor.advance(&graph, 0), Advance::End(WalkEnd::Random));

    // Heads that lead to each other make no sound; the cursor gives up instead of spinning.
    let mut map = branching();
    map.nodes[0] = control(0, vec![(0, 127, 5)], 0);
    map.nodes[5] = control(0, vec![(0, 127, 0)], 0);
    let graph = parse(&map);
    let mut cursor = Cursor::new(&graph, 0).unwrap();
    assert_eq!(cursor.advance(&graph, 0), Advance::End(WalkEnd::Loop));
}

#[test]
fn a_router_rewrites_the_choice_while_following() {
    let mut map = branching();
    map.routers = vec![vec![(4, 1)]];
    map.nodes[3].router = 1;
    let graph = parse(&map);
    let mut cursor = Cursor::new(&graph, 3).unwrap();
    cursor.advance(&graph, 0);
    // The high value would go to node 4, the router sends it back to node 1.
    assert_eq!(audio_of(cursor.advance(&graph, 100)).0, 1);
}

#[test]
fn a_start_node_that_does_not_exist_is_an_error() {
    let graph = parse(&branching());
    assert_eq!(Cursor::new(&graph, 99).unwrap_err(), Error::NoSuchEntry(99));
}
