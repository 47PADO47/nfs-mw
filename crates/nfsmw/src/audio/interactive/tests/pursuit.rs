//! Where the pursuit sets start.

use ea_audio::mus::graph::{Graph, Node, NodeKind};

use super::super::pursuit::{set_index, starts};

const HEADS: [usize; 4] = [15, 42, 73, 106];

fn node(kind: NodeKind, section: u8) -> Node {
    Node {
        kind,
        controller: 0,
        section,
        repeat: 0,
        router: 0,
        random: 0,
        beats: 4,
        bars: 1,
        part: 0,
        transitions: vec![],
    }
}

/// A graph big enough to hold the table's nodes, all heads of `sections` (in table order).
fn graph(sections: [u8; 4]) -> Graph {
    let mut nodes: Vec<Node> = (0..0x200).map(|_| node(NodeKind::Audio { stream: 0 }, 5)).collect();
    for (&at, section) in HEADS.iter().zip(sections) {
        nodes[at] = node(NodeKind::Head, section);
    }
    Graph { project: 0, sections: 7, nodes, events: vec![], routers: vec![], variables: vec![] }
}

#[test]
fn each_set_starts_at_the_first_head_of_its_section() {
    // The table lists section 4 first, then 1, 3, 2: set 1 is the second entry, and so on.
    let found = starts(&graph([4, 1, 3, 2])).unwrap();
    assert_eq!(found, [HEADS[1], HEADS[3], HEADS[2], HEADS[0]]);
}

#[test]
fn a_missing_section_is_refused_instead_of_playing_an_unrelated_cue() {
    assert!(starts(&graph([1, 1, 2, 3])).is_err());
    assert!(starts(&graph([5, 5, 5, 5])).is_err());
}

#[test]
fn a_map_without_the_table_nodes_is_refused() {
    let mut small = graph([1, 2, 3, 4]);
    small.nodes.truncate(HEADS[3]);
    assert!(starts(&small).unwrap_err().contains("section 4"));

    let mut audio = graph([1, 2, 3, 4]);
    audio.nodes[HEADS[2]] = node(NodeKind::Audio { stream: 3 }, 3);
    assert!(starts(&audio).unwrap_err().contains("section 3"));
}

#[test]
fn later_section_one_stingers_do_not_replace_the_entry_of_another_section() {
    let mut graph = graph([1, 2, 3, 4]);
    for at in [213, 306, 409] {
        graph.nodes[at] = node(NodeKind::Head, 1);
    }
    assert_eq!(starts(&graph).unwrap(), HEADS);
}

#[test]
fn sets_are_numbered_from_one_to_four() {
    assert_eq!((set_index(1), set_index(4)), (Ok(0), Ok(3)));
    assert!(set_index(0).is_err());
    assert!(set_index(5).is_err());
}
