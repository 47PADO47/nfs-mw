use super::graph_build::*;
use crate::error::Error;
use crate::mus::graph::{Graph, NodeKind, RouterEntry, Transition, WalkEnd};
use crate::mus::{Chain, Mpf};

/// Node 0 is a head, 1 to 3 play streams 0 to 2, 4 fires an event, 5 plays stream 3, 6 is the end. An event
/// starts the song at node 0 (a game id has a project bit above the 24 bits).
fn song() -> Map {
    Map {
        nodes: vec![
            control(0, vec![(0, 127, 1)], 0),
            audio(0, 2),
            audio(1, 3),
            audio(2, 4),
            control(0xFFFD, vec![(0, 127, 5)], 0x0012_3456),
            audio(3, 6),
            control(0xFFFF, vec![], 0),
        ],
        events: vec![song_event(0x00AB_CDEF, 0), E { id: 7, actions: vec![(0x000F_0400, 0x01FF_FFFF)] }],
        routers: vec![],
        variables: vec![("rapsheet", 0), ("ambstate", 9)],
        streams: vec![(2, 1000), (4, 2000), (6, 3000), (8, 500)],
    }
}

#[test]
fn the_graph_reads_nodes_events_routers_and_variables() {
    let mut map = song();
    map.routers = vec![vec![(5, 2)], vec![(1, 3), (1, 4)]];
    map.nodes[1].router = 2;
    let graph = Graph::parse(&map.build()).unwrap();
    assert_eq!((graph.project, graph.sections), (0, 7));
    assert_eq!(graph.nodes.len(), 7);
    assert_eq!(graph.nodes[0].kind, NodeKind::Head);
    assert_eq!(graph.nodes[1].kind, NodeKind::Audio { stream: 0 });
    assert_eq!(graph.nodes[4].kind, NodeKind::FireEvent { event: 0x12_3456 });
    assert_eq!(graph.nodes[6].kind, NodeKind::End);
    assert_eq!((graph.nodes[1].section, graph.nodes[1].beats, graph.nodes[1].bars), (5, 4, 1));
    assert_eq!(graph.nodes[1].router, 2);
    assert_eq!(graph.nodes[2].transitions, [Transition { lo: 0, hi: 127, target: 3 }]);
    assert!(graph.nodes[6].transitions.is_empty());
    assert_eq!(
        graph.routers,
        [
            vec![RouterEntry { key: 5, value: 2 }],
            vec![RouterEntry { key: 1, value: 3 }, RouterEntry { key: 1, value: 4 }]
        ]
    );
    assert_eq!(graph.variables.len(), 2);
    assert_eq!((graph.variables[1].name.as_str(), graph.variables[1].initial), ("ambstate", 9));
    assert_eq!(graph.events.len(), 2);
    assert_eq!(graph.events[0].actions.len(), 2);
}

#[test]
fn events_are_found_by_their_low_24_bits_and_a_song_event_names_its_start() {
    let graph = Graph::parse(&song().build()).unwrap();
    assert_eq!(graph.song_start(0x01AB_CDEF), Some(0));
    assert_eq!(graph.song_start(0x00AB_CDEF), Some(0));
    assert_eq!(graph.song_start(0x01AB_CDEE), None);
    // An event that only stops has no start node.
    assert_eq!(graph.song_start(7), None);
    assert_eq!(graph.event(7).unwrap().actions[0].branch_node(), None);
    assert_eq!(graph.event(0xAB_CDEF).unwrap().actions[1].opcode(), 4);
}

#[test]
fn the_first_matching_range_wins_else_the_nearest() {
    let mut map = song();
    // Overlapping ranges, then a gap between 40 and 59 and none above 90.
    map.nodes[0] = control(0, vec![(-1, 20, 1), (10, 40, 2), (60, 90, 3)], 0);
    let graph = Graph::parse(&map.build()).unwrap();
    let go = |v| graph.next(0, v);
    assert_eq!(go(0), Some(1));
    assert_eq!(go(15), Some(1), "first match wins over the later overlapping range");
    assert_eq!(go(30), Some(2));
    assert_eq!(go(45), Some(2), "45 is 5 from 40 and 15 from 60");
    assert_eq!(go(55), Some(3), "55 is 5 from 60 and 15 from 40");
    assert_eq!(go(127), Some(3), "above every range: the nearest");
    // A tie goes to the first transition.
    let mut map = song();
    map.nodes[0] = control(0, vec![(0, 10, 1), (20, 30, 2)], 0);
    let graph = Graph::parse(&map.build()).unwrap();
    assert_eq!(graph.next(0, 15), Some(1));
    // No transition, or a target of none.
    assert_eq!(graph.next(6, 0), None);
    let mut map = song();
    map.nodes[0] = control(0, vec![(0, 127, -1)], 0);
    assert_eq!(Graph::parse(&map.build()).unwrap().next(0, 0), None);
}

#[test]
fn a_router_rewrites_the_chosen_target_and_the_last_match_wins() {
    let mut map = song();
    map.routers = vec![vec![(5, 6)], vec![(2, 4), (3, 5), (2, 6)]];
    map.nodes[1].router = 2; // goes to 2: both entries with key 2 match, the later one wins
    map.nodes[2].router = 1; // goes to 3: no entry with key 3 in router 1
    map.nodes[3].router = 1; // goes to 4: untouched
    map.nodes[5].router = 9; // a router that does not exist is ignored
    let graph = Graph::parse(&map.build()).unwrap();
    assert_eq!(graph.next(1, 0), Some(6));
    assert_eq!(graph.next(2, 0), Some(3));
    assert_eq!(graph.next(3, 0), Some(4));
    assert_eq!(graph.next(5, 0), Some(6));
}

#[test]
fn a_walk_passes_control_nodes_and_stops_at_the_end() {
    let graph = Graph::parse(&song().build()).unwrap();
    let walk = graph.walk(0, 0).unwrap();
    assert_eq!(walk.nodes, [0, 1, 2, 3, 4, 5, 6]);
    assert_eq!(walk.end, WalkEnd::EndNode);
    assert_eq!(walk.audio(&graph).collect::<Vec<_>>(), [1, 2, 3, 5]);
    assert_eq!(graph.walk(99, 0).unwrap_err(), Error::NoSuchEntry(99));
}

#[test]
fn a_walk_notices_loops_dead_ends_and_random_nodes() {
    let mut map = song();
    map.nodes[5] = audio(3, 1); // back to node 1
    let walk = Graph::parse(&map.build()).unwrap().walk(0, 0).unwrap();
    assert_eq!(walk.end, WalkEnd::Loop);
    assert_eq!(walk.nodes, [0, 1, 2, 3, 4, 5]);

    let mut map = song();
    map.nodes[5].trans.clear(); // an audio node that leads nowhere
    assert_eq!(Graph::parse(&map.build()).unwrap().walk(0, 0).unwrap().end, WalkEnd::DeadEnd);

    let mut map = song();
    map.nodes[2] = control(0xFFFE, vec![(0, 127, 3)], 0);
    let walk = Graph::parse(&map.build()).unwrap().walk(0, 0).unwrap();
    assert_eq!((walk.end, walk.nodes.len()), (WalkEnd::Random, 3));
}

#[test]
fn a_chain_lists_the_streams_with_their_start_times() {
    let map = song();
    let graph = Graph::parse(&map.build()).unwrap();
    let mpf = Mpf::parse(&map.build()).unwrap();
    let chain = mpf.song_chain(&graph, 0x01AB_CDEF, 0).unwrap();
    let streams: Vec<_> = chain.segments.iter().map(|s| (s.node, s.stream, s.start_ms, s.duration_ms)).collect();
    assert_eq!(streams, [(1, 0, 0, 1000), (2, 1, 1000, 2000), (3, 2, 3000, 3000), (5, 3, 6000, 500)]);
    assert_eq!((chain.total_ms(), chain.end), (6500, WalkEnd::EndNode));
    assert!((chain.total_secs() - 6.5).abs() < 1e-9);
    assert_eq!(Chain { segments: vec![], end: WalkEnd::DeadEnd }.total_ms(), 0);
    assert!(mpf.song_chain(&graph, 0x42, 0).is_err());
}

#[test]
fn a_node_that_names_a_missing_stream_is_an_error() {
    let mut map = song();
    map.streams.pop();
    let graph = Graph::parse(&map.build()).unwrap();
    let mpf = Mpf::parse(&map.build()).unwrap();
    assert_eq!(mpf.chain(&graph, 0, 0).unwrap_err(), Error::NoSuchEntry(3));
}

#[test]
fn bad_graphs_are_errors() {
    let good = song().build();
    assert!(matches!(Graph::parse(&good[..3]), Err(Error::BadMagic { .. })));
    let mut v4 = good.clone();
    v4[4] = 4;
    assert!(matches!(Graph::parse(&v4), Err(Error::Unsupported(_))));
    assert!(Graph::parse(&good[..0x44]).is_err());

    let mut dangling = song();
    dangling.nodes[1].trans = vec![(0, 127, 40)];
    assert!(matches!(Graph::parse(&dangling.build()), Err(Error::Corrupt(_))));

    let mut branch = song();
    branch.events.push(song_event(9, 500));
    assert!(matches!(Graph::parse(&branch.build()), Err(Error::Corrupt(_))));

    let mut routers = song();
    routers.routers = vec![vec![(1, 1)]];
    let mut bytes = routers.build();
    // Make the second router offset smaller than the first.
    let table = u32::from_le_bytes(bytes[0x28..0x2C].try_into().unwrap()) as usize;
    bytes[table + 4..table + 8].copy_from_slice(&0u32.to_le_bytes());
    assert!(matches!(Graph::parse(&bytes), Err(Error::Corrupt(_))));
}
